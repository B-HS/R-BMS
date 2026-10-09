//! Unit tests for the host skeleton: the routing table is well formed, and a host whose clusters
//! know nothing answers every read with the absent value of its kind.

use std::borrow::Cow;
use std::path::Path;

use rbms_model::Mode;
use rbms_skin::dst::{DrawStateSource, OffsetSource};
use rbms_skin::lua::LocalTime;
use rbms_skin::property::generated::*;
use rbms_skin::property::{AudioCommand, FLOAT_ABSENT, HostCall, IMAGE_INDEX_ABSENT, INTEGER_ABSENT, SkinHost, StaticScreen, TEXT_ABSENT, VolumeBus};
use rbms_skin::timer::{TIMER_OFF, TimerId, TimerState};

use super::chart::{BpmRange, ChartContents, ChartMeta, ChartState, Density, NoteCounts};
use super::loading::{LoadingScreen, LoadingState};
use super::system::{CourseStage, SystemState, Volumes};
use super::{Cluster, ClusterState, IdSpace, ROUTES, ScreenHost, clusters_of};

/// A timer id no test switches on, and one a test does.
const QUIET_TIMER: i32 = 41;
const RUNNING_TIMER: i32 = 40;

/// When the running timer was switched on, and the frame clock the host is built at.
const STARTED_US: i64 = 250_000;
const NOW_US: i64 = 1_000_000;

#[test]
fn every_route_is_a_run_that_goes_upwards_and_every_cluster_has_one() {
    for route in ROUTES {
        assert!(route.first <= route.last, "{route:?} runs backwards");
    }
    for cluster in Cluster::ALL {
        assert!(ROUTES.iter().any(|route| route.cluster == cluster), "{cluster:?} answers nothing");
    }
}

#[test]
fn an_id_is_routed_to_the_clusters_that_claim_it_in_the_order_they_are_asked() {
    assert_eq!(clusters_of(IdSpace::Integer, NUMBER_PLAYLEVEL).collect::<Vec<_>>(), [Cluster::Chart]);
    assert_eq!(clusters_of(IdSpace::Boolean, OPTION_NOW_LOADING).collect::<Vec<_>>(), [Cluster::Loading]);
    assert_eq!(clusters_of(IdSpace::Text, STRING_TITLE).collect::<Vec<_>>(), [Cluster::Chart]);
    assert_eq!(clusters_of(IdSpace::Rate, RATE_MUSICSELECT_POSITION).collect::<Vec<_>>(), [Cluster::Select]);
    assert_eq!(clusters_of(IdSpace::Offset, OFFSET_LIFT).collect::<Vec<_>>(), [Cluster::Play]);
    assert_eq!(
        clusters_of(IdSpace::Integer, NUMBER_HIGHSCORE2).collect::<Vec<_>>(),
        [Cluster::Result, Cluster::Score],
        "a score both screens report is the result screen's first"
    );
    assert_eq!(clusters_of(IdSpace::Text, NUMBER_PLAYLEVEL).count(), 0, "an id is routed within its own space only");
}

#[test]
fn a_host_whose_clusters_know_nothing_answers_every_read_as_absent() {
    let mut timers = TimerState::new();
    timers.set_on(TimerId(RUNNING_TIMER), STARTED_US);
    let mut host = ScreenHost::new(NOW_US, &timers);
    host.static_screen = Some(StaticScreen::Result);

    assert_eq!(host.boolean(OPTION_NOW_LOADING), None);
    assert_eq!(host.boolean(-OPTION_NOW_LOADING), None, "negating an option nobody knows still knows nothing");
    assert_eq!(host.integer(NUMBER_PLAYLEVEL), INTEGER_ABSENT);
    assert_eq!(host.image_index(NUMBER_PLAYLEVEL), IMAGE_INDEX_ABSENT);
    assert_eq!(host.rate(RATE_MUSICSELECT_POSITION), None);
    assert_eq!(host.float(RATE_MUSICSELECT_POSITION), FLOAT_ABSENT);
    assert_eq!(host.text(STRING_TITLE), Cow::Borrowed(TEXT_ABSENT));
    assert_eq!(host.offset(OFFSET_LIFT), None);
    assert_eq!(host.timer_us(QUIET_TIMER), TIMER_OFF);
    assert_eq!(host.timer_us(RUNNING_TIMER), STARTED_US, "a timer is read from the scene's own table");
    assert_eq!(host.now_us(), NOW_US);
    assert!(host.is_static(OPTION_RESULT_AAA_1P), "an option the reference settles once on a result screen is static there");
    assert!(!host.is_static(OPTION_NOW_LOADING), "and one it evaluates on every frame is not");
}

#[test]
fn what_a_skin_tells_the_host_is_kept_in_order_until_it_is_taken() {
    let timers = TimerState::new();
    let host = ScreenHost::new(NOW_US, &timers);
    let sound = Path::new("sound/decide.ogg");

    host.exec_event(BUTTON_MODE, 1, 0);
    host.write_rate(RATE_MUSICSELECT_POSITION, 0.5);
    host.write_text(STRING_TITLE, "typed");
    host.audio(AudioCommand::Play { path: sound, volume: 1.0, looped: false });
    host.set_volume(VolumeBus::System, 0.25);

    assert_eq!(
        host.take_calls(),
        [
            HostCall::Event { id: BUTTON_MODE, arg1: 1, arg2: 0 },
            HostCall::WriteRate { id: RATE_MUSICSELECT_POSITION, value: 0.5 },
            HostCall::WriteText { id: STRING_TITLE, value: "typed".to_owned() },
            HostCall::AudioPlay { path: sound.to_path_buf(), volume: 1.0, looped: false },
            HostCall::SetVolume { bus: VolumeBus::System, value: 0.25 },
        ]
    );
    assert!(host.take_calls().is_empty(), "taking the calls leaves none behind");
}

/// The decide screen's fade-out timer, which the scene's timer table answers and no cluster does.
const FADEOUT_TIMER: i32 = 2;

/// The play-time numbers, which the scores a frame borrows do not hold.
const UNANSWERED_PLAY_TIME: std::ops::RangeInclusive<i32> = NUMBER_TOTALPLAYTIME_HOUR..=NUMBER_TOTALPLAYTIME_SECOND;

/// A chart the player knows everything about.
fn known_chart() -> ChartMeta<'static> {
    ChartMeta {
        title: "Decide Title",
        subtitle: "[SP ANOTHER]",
        genre: "TRANCE",
        artist: "Composer",
        subartist: "feat. Singer",
        heading: None,
        md5: "md5-of-the-chart",
        sha256: "sha256-of-the-chart",
        table_name: "Satellite",
        table_level: "sl",
        level: 9,
        difficulty: 3,
        mode: Some(Mode::BEAT_7K),
        judge: Some(2),
        length_ms: Some(130_000),
        notes: Some(900),
        bpm: Some(BpmRange { min: 140, max: 160 }),
        main_bpm: Some(150.0),
        note_counts: Some(NoteCounts { normal: 800, long: 50, scratch: 40, long_scratch: 10 }),
        density: Some(Density { average: 7.5, peak: 15.0, end: 11.0 }),
        total: Some(300.0),
        contents: ChartContents {
            bga: Some(true),
            text: Some(false),
            long_note: Some(true),
            random_sequence: Some(false),
            bpm_stop: Some(false),
            stagefile: Some(true),
            banner: Some(true),
            backbmp: Some(false),
        },
    }
}

/// Whether the cluster gives `id` of `space` an answer.
fn is_answered(cluster: &dyn ClusterState, space: IdSpace, id: i32) -> bool {
    match space {
        IdSpace::Boolean => cluster.boolean(id).is_some(),
        IdSpace::Integer => cluster.integer(id).is_some(),
        IdSpace::ImageIndex => cluster.image_index(id).is_some(),
        IdSpace::Rate => cluster.rate(id).is_some(),
        IdSpace::Float => cluster.float(id).is_some(),
        IdSpace::Text => cluster.text(id).is_some(),
        IdSpace::Offset => cluster.offset(id).is_some(),
    }
}

/// The clock the filled system cluster is handed: the first second of 1970.
const EPOCH_CLOCK: LocalTime = LocalTime { year: 1970, month: 1, day: 1, hour: 0, minute: 0, second: 0 };

#[test]
fn every_id_routed_to_a_filled_cluster_is_answered_by_it() {
    let records: [rbms_store::ScoreRecord; 0] = [];
    let meta = known_chart();
    let chart = ChartState::Chart(&meta);
    let system = SystemState {
        clock: Some(EPOCH_CLOCK),
        uptime_ms: Some(1),
        fps: Some(60.0),
        player_name: Some("guest"),
        version: Some("R-BMS"),
        history: Some(&records),
        volumes: Some(Volumes { system: 0.5, key: 0.5, background: 0.5 }),
        course: Some(CourseStage { index: 0, count: 2 }),
    };
    let loading = LoadingState { screen: LoadingScreen::Preload, progress: 0.5 };

    for route in ROUTES {
        let cluster: &dyn ClusterState = match route.cluster {
            Cluster::Chart => &chart,
            Cluster::System => &system,
            Cluster::Loading => &loading,
            _ => continue,
        };
        for id in route.first..=route.last {
            let unanswered_on_purpose = route.cluster == Cluster::System && route.space == IdSpace::Integer && UNANSWERED_PLAY_TIME.contains(&id);
            assert_ne!(
                is_answered(cluster, route.space, id),
                unanswered_on_purpose,
                "{:?} id {id} in {:?} is routed to {:?}",
                route.space,
                route,
                route.cluster
            );
        }
    }
}

#[test]
fn the_ids_this_wave_added_are_routed_to_the_cluster_that_reads_them() {
    assert_eq!(clusters_of(IdSpace::Text, STRING_TABLE_FULL).collect::<Vec<_>>(), [Cluster::Chart]);
    assert_eq!(clusters_of(IdSpace::Text, STRING_PLAYER).collect::<Vec<_>>(), [Cluster::System]);
    assert_eq!(clusters_of(IdSpace::Boolean, OPTION_MODE_COURSE).collect::<Vec<_>>(), [Cluster::System]);
    assert_eq!(clusters_of(IdSpace::Boolean, OPTION_COURSE_STAGE_FINAL).collect::<Vec<_>>(), [Cluster::System]);
    assert_eq!(clusters_of(IdSpace::Boolean, OPTION_COURSE_STAGE2).collect::<Vec<_>>(), [Cluster::System]);
    assert_eq!(clusters_of(IdSpace::Rate, RATE_KEYVOLUME).collect::<Vec<_>>(), [Cluster::Options, Cluster::System]);
    assert_eq!(clusters_of(IdSpace::Integer, NUMBER_TOTALNOTES).collect::<Vec<_>>(), [Cluster::Score, Cluster::Chart]);
    assert_eq!(clusters_of(IdSpace::Integer, NUMBER_CURRENT_FPS).collect::<Vec<_>>(), [Cluster::System]);
}

/// What the decide screen of a skin reads (`m5-decide-result.md` appendix B): the difficulty
/// options, the level, the song texts and the table, whether there is a stage image, and the
/// fade-out timer.
#[test]
fn a_decide_screen_host_has_a_value_for_everything_the_decide_skin_reads() {
    let mut timers = TimerState::new();
    timers.set_on(TimerId(FADEOUT_TIMER), STARTED_US);
    let meta = known_chart();
    let mut host = ScreenHost::new(NOW_US, &timers);
    host.chart = ChartState::Chart(&meta);
    host.loading = LoadingState { screen: LoadingScreen::Elsewhere, progress: 0.0 };

    let difficulty_options = [OPTION_DIFFICULTY0, OPTION_DIFFICULTY1, OPTION_DIFFICULTY2, OPTION_DIFFICULTY3, OPTION_DIFFICULTY4, OPTION_DIFFICULTY5];
    let on = difficulty_options.into_iter().filter(|id| host.boolean(*id) == Some(true)).collect::<Vec<_>>();
    assert_eq!(on, [OPTION_DIFFICULTY3], "exactly one difficulty option is on");
    assert!(difficulty_options.into_iter().all(|id| host.boolean(id).is_some() && host.boolean(-id).is_some()), "and each answers on its own and negated");

    assert_eq!(host.integer(NUMBER_PLAYLEVEL), 9);
    for text in [STRING_TITLE, STRING_SUBTITLE, STRING_FULLTITLE, STRING_GENRE, STRING_ARTIST, STRING_SUBARTIST, STRING_TABLE_FULL] {
        assert!(!host.text(text).is_empty(), "text {text} is empty");
    }
    assert_eq!(host.text(STRING_TABLE_FULL), "slSatellite");
    assert_eq!(host.boolean(OPTION_STAGEFILE), Some(true));
    assert_eq!(host.boolean(OPTION_NO_STAGEFILE), Some(false));
    assert_eq!(host.boolean(OPTION_NOW_LOADING), Some(false), "the decide screen is not the player's preload");
    assert_eq!(host.timer_us(FADEOUT_TIMER), STARTED_US);
}

#[test]
fn a_decide_screen_host_with_no_chart_in_hand_reads_the_reference_s_missing_song() {
    let timers = TimerState::new();
    let mut host = ScreenHost::new(NOW_US, &timers);
    host.chart = ChartState::Empty;

    assert_eq!(host.integer(NUMBER_PLAYLEVEL), INTEGER_ABSENT);
    assert_eq!(host.float(FLOAT_CHART_TOTALGAUGE), FLOAT_ABSENT);
    assert_eq!(host.text(STRING_TITLE), "");
    assert_eq!(host.boolean(OPTION_DIFFICULTY0), Some(false));
    assert_eq!(host.boolean(-OPTION_DIFFICULTY0), Some(true), "negating an option that is off is on");
    assert_eq!(host.boolean(-OPTION_BGA), Some(true));
}

#[test]
fn the_machine_clusters_answer_through_the_host_and_negation_applies_to_them() {
    let timers = TimerState::new();
    let mut host = ScreenHost::new(NOW_US, &timers);
    host.system = SystemState {
        player_name: Some("guest"),
        volumes: Some(Volumes { system: 0.5, key: 1.0, background: 0.0 }),
        course: Some(CourseStage { index: 1, count: 3 }),
        ..SystemState::default()
    };
    host.loading = LoadingState { screen: LoadingScreen::Preload, progress: 0.25 };

    assert_eq!(host.text(STRING_PLAYER), "guest");
    assert_eq!(host.integer(NUMBER_MASTER_VOLUME), 50);
    assert_eq!(host.volume(VolumeBus::Key), 1.0);
    assert_eq!(host.rate(RATE_BGMVOLUME), Some(0.0));
    assert_eq!(host.float(RATE_KEYVOLUME), 1.0, "a float read falls back to the rate of the same id");
    assert_eq!((host.boolean(OPTION_MODE_COURSE), host.boolean(-OPTION_MODE_COURSE)), (Some(true), Some(false)));
    assert_eq!(host.boolean(OPTION_COURSE_STAGE2), Some(true));
    assert_eq!(host.integer(NUMBER_LOADING_PROGRESS), 25);
    assert_eq!(host.rate(RATE_LOAD_PROGRESS), Some(0.25));
    assert_eq!(host.float(FLOAT_LOADING_PROGRESS), 0.25);
    assert_eq!((host.boolean(OPTION_NOW_LOADING), host.boolean(OPTION_LOADED)), (Some(true), Some(false)));
}
