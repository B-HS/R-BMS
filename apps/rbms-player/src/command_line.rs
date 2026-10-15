//! The player's command line: what each option sets for one launch, and the asks answered without
//! starting the app at all (`--help`, `-h`, `--version`, and a command line this build refuses).
//!
//! The command line is read before anything on disk or on the network is touched, so a launch that
//! only asks for the usage or the version, or one that is refused, leaves the settings, the
//! databases, the installed skin and every server as they were.

use std::process::ExitCode;

use rbms_config::{Config, gauge_from_name};
use winit::keyboard::KeyCode;

use crate::LaunchOptions;
use crate::keyconfig::key_from_name;
use crate::notify::{Level, notify};

/// The exit code of a refused command line, the one command-line tools use for a usage error.
pub(crate) const USAGE_EXIT_CODE: u8 = 2;

/// What every option is spelled with, and what a value never starts with.
const OPTION_PREFIX: &str = "--";

/// What an argument that is not a chart starts with: an option of either spelling, `-h` among them.
/// A chart or folder whose name starts with it is named with its folder in front (`./-name.bms`),
/// so a mistyped option such as `-v` is refused rather than started as a chart.
const ANY_OPTION_PREFIX: &str = "-";

/// The program name the usage and the version are printed under.
const PROGRAM: &str = "rbms-player";

/// What `--help` prints, and what follows the reason a command line is refused.
const USAGE: &str = "\
Usage: rbms-player [OPTIONS] [CHART | SONGS FOLDER]

With no chart or folder, the song select opens on the songs folder in the settings
(or RBMS_SONGS).

Options:
  --interactive         play by hand
  --auto                let the engine play
  --hispeed F           hi-speed
  --lift F              lane lift
  --gauge NAME          gauge: assist, easy, normal, hard, exhard, hazard
  --sc-left             scratch on the left
  --sc-auto             the engine plays the scratch
  --keys Z,S,X,...      lane keys for this launch, in place of the key config
  --skin FILE.ron       a built-in play-field layout
  --font FILE.ttf       UI font
  --table URL           a difficulty table added for this launch
  --keyconfig FILE.ron  key config file to use
  --replay FILE.ron     play a saved replay back
  --timing-csv FILE     where the per-input timing samples are written
  --server URL          score server
  --player ID           player id
  -h, --help            print this and exit
  --version             print the version and exit";

/// What one launch is asked for on its command line, before it is folded over the settings.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct CommandLine {
    autoplay: Option<bool>,
    scratch_left: bool,
    scratch_auto: bool,
    lift: Option<f32>,
    hispeed: Option<f64>,
    gauge: Option<String>,
    font: Option<String>,
    server: Option<String>,
    player: Option<String>,
    skin: Option<String>,
    table: Option<String>,
    keyconfig: Option<String>,
    replay: Option<String>,
    timing_csv: Option<String>,
    keys: Option<Vec<(KeyCode, usize)>>,
    /// The first argument that is not an option: a chart, or a songs folder.
    chart: Option<String>,
}

/// A command line answered without starting the app.
#[derive(Debug, PartialEq)]
pub(crate) enum Answer {
    /// `--help` or `-h`.
    Usage,
    /// `--version`.
    Version,
    /// An option this build does not know, or one given without its value. The reason is printed.
    Refused(String),
}

impl Answer {
    /// Print the answer and give the exit code it ends the process with.
    pub(crate) fn give(self) -> ExitCode {
        match self {
            Answer::Usage => {
                println!("{USAGE}");
                ExitCode::SUCCESS
            }
            Answer::Version => {
                println!("{PROGRAM} {}", env!("CARGO_PKG_VERSION"));
                ExitCode::SUCCESS
            }
            Answer::Refused(reason) => {
                notify(Level::Error, format!("{PROGRAM}: {reason}\n\n{USAGE}"));
                ExitCode::from(USAGE_EXIT_CODE)
            }
        }
    }
}

/// The value the option `option` is followed by. A missing value, or another option in its place,
/// refuses the command line rather than letting the option take the next option as its value.
fn value_of(option: &str, args: &mut impl Iterator<Item = String>) -> Result<String, Answer> {
    args.next().filter(|value| !value.starts_with(OPTION_PREFIX)).ok_or_else(|| Answer::Refused(format!("{option} needs a value")))
}

/// Read the process argument list, program name included. A number that does not read leaves the
/// option unset, and the arguments after the first that is not an option are not read as charts.
pub(crate) fn read(args: impl Iterator<Item = String>) -> Result<CommandLine, Answer> {
    let mut line = CommandLine::default();
    let mut args = args.skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => return Err(Answer::Usage),
            "--version" => return Err(Answer::Version),
            "--interactive" => line.autoplay = Some(false),
            "--auto" => line.autoplay = Some(true),
            "--sc-left" => line.scratch_left = true,
            "--sc-auto" => line.scratch_auto = true,
            "--lift" => {
                if let Ok(lift) = value_of(&arg, &mut args)?.parse::<f32>() {
                    line.lift = Some(lift);
                }
            }
            "--hispeed" => {
                if let Ok(hispeed) = value_of(&arg, &mut args)?.parse::<f64>() {
                    line.hispeed = Some(hispeed);
                }
            }
            "--gauge" => line.gauge = Some(value_of(&arg, &mut args)?),
            "--skin" => line.skin = Some(value_of(&arg, &mut args)?),
            "--font" => line.font = Some(value_of(&arg, &mut args)?),
            "--server" => line.server = Some(value_of(&arg, &mut args)?),
            "--table" => line.table = Some(value_of(&arg, &mut args)?),
            "--keyconfig" => line.keyconfig = Some(value_of(&arg, &mut args)?),
            "--replay" => line.replay = Some(value_of(&arg, &mut args)?),
            "--timing-csv" => line.timing_csv = Some(value_of(&arg, &mut args)?),
            "--player" => line.player = Some(value_of(&arg, &mut args)?),
            "--keys" => {
                let spec = value_of(&arg, &mut args)?;
                let keys: Vec<(KeyCode, usize)> =
                    spec.split(',').enumerate().filter_map(|(lane, name)| key_from_name(name.trim()).map(|key| (key, lane))).collect();
                if !keys.is_empty() {
                    line.keys = Some(keys);
                }
            }
            system if crate::is_system_launch_arg(system) => {}
            option if option.starts_with(ANY_OPTION_PREFIX) => return Err(Answer::Refused(format!("unknown option {option}"))),
            chart => {
                line.chart.get_or_insert_with(|| chart.to_owned());
            }
        }
    }
    Ok(line)
}

impl CommandLine {
    /// Fold the options over the settings read from disk, set the launch options the command line
    /// names, and give the chart or songs folder named, if one was.
    pub(crate) fn apply(self, config: &mut Config, launch: &mut LaunchOptions) -> Option<String> {
        if let Some(autoplay) = self.autoplay {
            config.play.autoplay = autoplay;
        }
        config.play.scratch_left |= self.scratch_left;
        config.play.scratch_auto |= self.scratch_auto;
        if let Some(lift) = self.lift {
            config.play.lift = lift.clamp(rbms_config::LANE_SHADE_MIN, rbms_config::LANE_SHADE_MAX);
        }
        if let Some(hispeed) = self.hispeed {
            config.play.hispeed = hispeed.clamp(rbms_config::HISPEED_MIN, rbms_config::HISPEED_MAX);
        }
        if let Some(gauge) = self.gauge {
            config.play.gauge = gauge_from_name(&gauge);
        }
        if let Some(font) = self.font {
            config.display.font_path = Some(font);
        }
        if let Some(server) = self.server {
            config.network.server_url = Some(server);
        }
        if let Some(player) = self.player {
            config.network.player_id = player;
        }
        launch.skin_path = self.skin;
        launch.table_url = self.table;
        launch.keyconfig_path = self.keyconfig;
        launch.replay_path = self.replay;
        launch.timing_csv = self.timing_csv;
        launch.keys_override = self.keys;
        self.chart
    }
}

#[cfg(test)]
mod tests;
