//! What the command line asks for, read without starting anything.

use super::*;

/// The hi-speed the tests ask for.
const HISPEED_ASKED: f64 = 3.5;

/// A score server address nothing answers on.
const SERVER_NOBODY_ANSWERS: &str = "http://127.0.0.1:9";

/// The command line `rbms-player <args>`.
fn line(args: &[&str]) -> Result<CommandLine, Answer> {
    read(std::iter::once(PROGRAM).chain(args.iter().copied()).map(str::to_owned))
}

#[test]
fn the_usage_and_the_version_are_asked_for_wherever_they_stand() {
    assert_eq!(line(&["--help"]), Err(Answer::Usage));
    assert_eq!(line(&["-h"]), Err(Answer::Usage));
    assert_eq!(line(&["/songs", "--auto", "--help"]), Err(Answer::Usage));
    assert_eq!(line(&["--version"]), Err(Answer::Version));
    assert_eq!(line(&["--server", SERVER_NOBODY_ANSWERS, "--version"]), Err(Answer::Version));
}

#[test]
fn an_option_this_build_does_not_know_is_refused() {
    assert_eq!(line(&["--no-such-option"]), Err(Answer::Refused("unknown option --no-such-option".to_owned())));
    assert_eq!(line(&["/songs", "--"]), Err(Answer::Refused("unknown option --".to_owned())));
}

/// An argument with a single dash in front is a mistyped option, never a chart: a launch that took
/// `-v` for a chart would read the settings, install the skin and open the databases of the real
/// home for a chart that is not there. The system's own argument is still let through.
#[test]
fn a_single_dash_option_this_build_does_not_know_is_refused_rather_than_read_as_a_chart() {
    for option in ["-v", "-V", "-help", "-version", "-"] {
        assert_eq!(line(&[option]), Err(Answer::Refused(format!("unknown option {option}"))), "{option} was not refused");
    }
    assert_eq!(line(&["./-name.bms"]).map(|read| read.chart), Ok(Some("./-name.bms".to_owned())), "a chart named with its folder in front was refused");
    assert_eq!(line(&["-psn_0_12345"]).map(|read| read.chart), Ok(None), "the system's own argument was refused");
}

#[test]
fn an_option_without_its_value_is_refused_rather_than_taking_the_next_option() {
    assert_eq!(line(&["--server"]), Err(Answer::Refused("--server needs a value".to_owned())));
    assert_eq!(line(&["--player", "--help"]), Err(Answer::Refused("--player needs a value".to_owned())));
}

#[test]
fn the_options_fold_over_the_settings_and_the_launch() {
    let hispeed = HISPEED_ASKED.to_string();
    let read = line(&["--auto", "--hispeed", &hispeed, "--gauge", "hard", "--server", SERVER_NOBODY_ANSWERS, "--keys", "Z,S", "/songs/a.bms", "/songs/b.bms"])
        .expect("the command line reads");
    let mut config = Config::default();
    let mut launch = LaunchOptions::default();
    let chart = read.apply(&mut config, &mut launch);

    assert_eq!(chart.as_deref(), Some("/songs/a.bms"), "the first argument that is not an option is the chart");
    assert!(config.play.autoplay);
    assert!((config.play.hispeed - HISPEED_ASKED).abs() < f64::EPSILON);
    assert_eq!(config.play.gauge, rbms_judge::GaugeKind::Hard);
    assert_eq!(config.network.server_url.as_deref(), Some(SERVER_NOBODY_ANSWERS));
    assert_eq!(launch.keys_override, Some(vec![(KeyCode::KeyZ, 0), (KeyCode::KeyS, 1)]));
}

#[test]
fn a_number_that_does_not_read_leaves_the_setting_alone() {
    let read = line(&["--hispeed", "fast"]).expect("the command line reads");
    let mut config = Config::default();
    let before = config.play.hispeed;
    read.apply(&mut config, &mut LaunchOptions::default());
    assert!((config.play.hispeed - before).abs() < f64::EPSILON);
}

#[test]
fn the_argument_finder_adds_is_neither_a_chart_nor_an_unknown_option() {
    assert_eq!(line(&["-psn_0_1234567"]), Ok(CommandLine::default()));
}
