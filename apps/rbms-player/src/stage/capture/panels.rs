//! The default pack's option panels after a key has changed what they show, and the assist grade of
//! a light-assist lamp where the pack names a lamp: the record window of the browser and the result
//! screen.
//!
//! Like the other captures of the default pack these save frames for a person to look at and pass
//! without drawing anything unless [`CAPTURE_DIR_ENV`] is set. The pack is the one in the
//! repository, so what is saved is what the working tree's Lua draws.
//!
//! Each panel is called up and a key of it is pressed while it is up, the way a player steps an
//! option, so the frame shows the panel with the option stepped and the panel's information line
//! naming what changed. The browser's record window is captured on a chart whose best is a
//! SCRATCH AUTO clear on the HARD gauge, and the result screen on a run the engine played with the
//! turntable lane played for it on that gauge, over a best that was such a clear on the EASY gauge.

use rbms_judge::GaugeKind;
use rbms_store::SCRATCH_AUTO_ASSIST;

use super::*;
use crate::app_play::scratch_auto_lanes;
use crate::select_tree::SelectFolder;

/// The default pack, from this crate's folder.
const PANEL_PACK: &str = "../../assets/skins/rbms-default";

/// The keys the default seven-key layout puts on keys two, three and five.
const KEY_TWO: KeyCode = KeyCode::KeyS;
const KEY_THREE: KeyCode = KeyCode::KeyX;
const KEY_FIVE: KeyCode = KeyCode::KeyC;

/// Each panel walked: the name it is saved under once up, the keys held for it, the key pressed while
/// it is up, and the name it is saved under once that key has stepped its option.
const PANEL_STEPS: [(&str, &[KeyCode], KeyCode, &str); 4] = [
    ("panels-play", &[BROWSER_START_KEY], KEY_FIVE, "panels-play-stepped"),
    ("panels-assist", &[BROWSER_SELECT_KEY], KEY_THREE, "panels-assist-switched"),
    ("panels-assist-again", &[BROWSER_SELECT_KEY], KEY_FIVE, "panels-assist-keeping"),
    ("panels-detail", &[BROWSER_START_KEY, BROWSER_SELECT_KEY], KEY_TWO, "panels-detail-stepped"),
];

/// The chart whose best is graded, its gauge, and the name the browser is saved under on it.
const GRADED_TITLE: &str = "Iris";
const GRADED_GAUGE: &str = "hard";
const GRADED_BROWSER: &str = "panels-graded-best";

/// The chart the graded result is played on, with notes on the turntable lane for SCRATCH AUTO to
/// play, and the gauge the best before it was cleared on.
const GRADED_CHART: &str = "#PLAYER 1\n#GENRE Capture\n#TITLE Graded\n#ARTIST Capture Artist\n#BPM 150\n#PLAYLEVEL 3\n#RANK 2\n#TOTAL 300\n#00111:01010101\n#00116:01000100\n#00212:0101\n#00216:0001\n";
const GRADED_CHART_FILE: &str = "graded.bms";
const PREVIOUS_GAUGE: &str = "easy";
const PREVIOUS_EX: u32 = 4;
const PREVIOUS_BREAKS: u32 = 1;
const GRADED_RESULT: &str = "panels-graded-result";

/// The browse library with the best of [`GRADED_TITLE`] made a SCRATCH AUTO clear on
/// [`GRADED_GAUGE`], in the score book and in the score database the record window reads.
fn grade_the_best(app: &mut App) -> Result<(), String> {
    let records = BROWSE_CHARTS.iter().filter_map(|chart| {
        let (clear, ex) = chart.record?;
        let entry = app.shared.library.songs().iter().find(|entry| entry.title == chart.title && entry.subtitle == chart.subtitle)?;
        let base = rbms_store::ScoreRecord {
            mode: entry.mode.name.to_owned(),
            counts: [ex / EX_PER_PGREAT, ex % EX_PER_PGREAT, 0, BROWSER_RECORD_BREAKS, 0, 0],
            ..record(&entry.md5, clear, ex, BROWSER_RECORD_BREAKS, BROWSER_RECORD_PLAYED_AT)
        };
        Some(match chart.title == GRADED_TITLE {
            true => rbms_store::ScoreRecord { assist: SCRATCH_AUTO_ASSIST, gauge: GRADED_GAUGE.to_owned(), ..base },
            false => base,
        })
    });
    app.shared.scores = rbms_store::ScoreBook::from_records(records.collect());
    let mut scores = ScoreDb::open_in_memory().map_err(|error| format!("the score database did not open: {error}"))?;
    scores.migrate().map_err(|error| format!("the score database was not laid out: {error}"))?;
    migrate_score_book(&mut scores, &app.shared.scores).map_err(|error| format!("the records were not stored: {error}"))?;
    app.shared.scoredb = Some(scores);
    Ok(())
}

/// Call up each panel of [`PANEL_STEPS`], save it, press its key, save it again, and let it go.
fn capture_stepped_panels(app: &mut App, pixels: &mut HeadlessCanvas, size: (u32, u32)) -> Result<Vec<String>, String> {
    let deadline = Instant::now() + PACK_LOAD_TIMEOUT;
    let mut saved = Vec::new();
    let mut at_ms = BROWSER_PANELS_FROM_MS;
    for (name, held, step, stepped) in PANEL_STEPS {
        for key in held {
            app.shared.note_key(&key_down(*key));
        }
        browser_frame_at(app, at_ms, pixels, deadline)?;
        at_ms += BROWSER_PANEL_SETTLE_MS;
        browser_frame_at(app, at_ms, pixels, deadline)?;
        save(name, CAPTURE_EXTENSION, size, pixels.rgba());
        saved.push(name.to_owned());
        for input in [key_down, key_up] {
            app.shared.note_key(&input(step));
            browser_frame_at(app, at_ms, pixels, deadline)?;
        }
        at_ms += BROWSER_PANEL_SETTLE_MS;
        browser_frame_at(app, at_ms, pixels, deadline)?;
        save(stepped, CAPTURE_EXTENSION, size, pixels.rgba());
        saved.push(stepped.to_owned());
        for key in held {
            app.shared.note_key(&key_up(*key));
        }
        browser_frame_at(app, at_ms, pixels, deadline)?;
        at_ms += BROWSER_PANEL_SETTLE_MS;
    }
    Ok(saved)
}

/// The browser over the browse library on the chart with the graded best, then its panels stepped.
fn capture_graded_browser(pack: &Path) -> Result<Vec<String>, String> {
    let mut app = browse_app(pack, "panels-browser")?;
    grade_the_best(&mut app)?;
    let size = authored_size(&app, SKIN_TYPE_MUSIC_SELECT).ok_or_else(|| NO_DOCUMENT.to_owned())?;
    let mut pixels = HeadlessCanvas::new(size.0, size.1);
    app.shared.open_for_tests([SelectFolder::AllSongs]);
    app.shared.rebuild_select_items();
    let graded = browser_row_of(&app, GRADED_TITLE).ok_or("the scan did not find the chart with the graded best")?;
    app.shared.sel = graded.checked_sub(1).ok_or("the chart with the graded best is the first of the list")?;
    let mut saved = capture_browser_view(&mut app, &mut pixels, size, GRADED_BROWSER, SelectState::new())?;
    saved.extend(capture_stepped_panels(&mut app, &mut pixels, size)?);
    Ok(saved)
}

/// The result screen of a run of [`GRADED_CHART`] the engine played on the HARD gauge with the
/// turntable lane played for it, over a best that was a SCRATCH AUTO clear on [`PREVIOUS_GAUGE`].
fn capture_graded_result(pack: &Path) -> Result<Vec<String>, String> {
    let tag = "panels-result";
    let mut app = pack_app(pack, tag);
    let size = authored_size(&app, SKIN_TYPE_RESULT).ok_or_else(|| NO_DOCUMENT.to_owned())?;
    let chart = settings_of(tag).with_file_name(GRADED_CHART_FILE);
    std::fs::write(&chart, GRADED_CHART).map_err(|error| format!("the chart was not written: {error}"))?;
    let source = rbms_parser::parse_with(GRADED_CHART.as_bytes(), Default::default());
    let name = chart.to_string_lossy().into_owned();
    let model = rbms_chart::to_model(&source, rbms_chart::detect_mode(&source, &name));
    let previous = rbms_store::ScoreRecord {
        assist: SCRATCH_AUTO_ASSIST,
        gauge: PREVIOUS_GAUGE.to_owned(),
        ..record(&model.md5, rbms_judge::clear_type_id(rbms_judge::ClearType::LightAssistEasy), PREVIOUS_EX, PREVIOUS_BREAKS, BROWSER_RECORD_PLAYED_AT)
    };
    app.shared.scores = rbms_store::ScoreBook::from_records(vec![previous]);
    let mut scores = ScoreDb::open_in_memory().map_err(|error| format!("the score database did not open: {error}"))?;
    scores.migrate().map_err(|error| format!("the score database was not laid out: {error}"))?;
    migrate_score_book(&mut scores, &app.shared.scores).map_err(|error| format!("the record was not stored: {error}"))?;
    app.shared.scoredb = Some(scores);

    let run_us = rbms_play::play_time_ms(&model, true) * MICROS_PER_MILLI + RESULT_RUN_MARGIN_US;
    app.shared.chart_path = name;
    app.shared.mode = model.mode;
    app.shared.config.play.autoplay = true;
    let options = SessionOptions { autoplay: true, gauge: GaugeKind::Hard, auto_lanes: scratch_auto_lanes(model.mode, true), ..SessionOptions::default() };
    let mut session = PlaySession::new(model, options);
    for frame in 0..=run_us / RESULT_RUN_FRAME_US {
        session.tick(SessionClock::at(frame * RESULT_RUN_FRAME_US), &mut NullSink);
    }
    let mut played = PlayState::new(session, std::collections::HashMap::new(), 0, SCORE_LN_MODE_FROM_CHART.to_string());
    let entered = enter_result(&mut played, &mut app.shared);
    if !matches!(entered, Transition::To(Stage::Result(_))) {
        return Err("the run did not reach the result screen".to_owned());
    }
    app.switch(entered);
    walk_result_scene(&mut app, SKIN_TYPE_RESULT, size, GRADED_RESULT)
}

/// Captures the default pack's option panels with an option stepped on each, the record window of a
/// chart whose best is a graded SCRATCH AUTO clear, and the result screen of a graded run.
///
/// Opt-in: without [`CAPTURE_DIR_ENV`] this passes without drawing anything. With it, every screen
/// has to draw, and the pack's folder has to be left as it was.
#[test]
fn the_default_packs_option_panels_and_graded_lamps_are_captured() {
    if std::env::var_os(CAPTURE_DIR_ENV).is_none() {
        return;
    }
    let pack = Path::new(env!("CARGO_MANIFEST_DIR")).join(PANEL_PACK);
    let before = files_under(&pack);
    for (label, outcome) in [("panels", capture_graded_browser(&pack)), ("graded result", capture_graded_result(&pack))] {
        let saved = outcome.unwrap_or_else(|reason| panic!("{label}: not drawn: {reason}"));
        println!("{label}: captured {}", saved.join(", "));
    }
    assert_eq!(files_under(&pack), before, "capturing the default pack changed its folder");
}
