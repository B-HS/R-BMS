//! The RESULT screen: the summary of the run that just ended, plus what the score server said
//! about it.
#![allow(clippy::wildcard_imports)]

use rbms_render::result::ResultExtras;

use crate::app_result::{next_song, offers_retry, retry};
use crate::stage::{Canvas, FrameCtx, KeyInput, StageHandler, Transition};
use crate::*;

/// The result screen's own state: the summary view built when the run ended, and what surrounds it —
/// the target the run was paced against and whether it can be run again. The submission status under
/// it lives in [`AppShared`], because the request outlives this screen.
///
/// The view is boxed: it carries the run's whole measurement, which would otherwise set the size of
/// every other screen and make each stage change a large copy.
pub(crate) struct ResultState {
    view: Box<ResultView>,
    extras: ResultExtras,
    /// Whether the run counted as a clear. The view carries the lamp's label and colour but not the
    /// verdict itself, and a document asks for the verdict.
    cleared: bool,
}

impl ResultState {
    /// A screen reporting a run and nothing around it: no target, no offer to run it again, and a
    /// run that did not clear.
    pub(crate) fn new(view: ResultView) -> ResultState {
        ResultState { view: Box::new(view), extras: ResultExtras::default(), cleared: false }
    }

    /// The same screen reporting a run that reached the end with its gauge up.
    pub(crate) fn cleared(mut self, cleared: bool) -> ResultState {
        self.cleared = cleared;
        self
    }

    /// The same screen with what surrounds the run on it: the target it was paced against, and
    /// whether the run-again keys are live.
    pub(crate) fn paced_by(mut self, extras: ResultExtras) -> ResultState {
        self.extras = extras;
        self
    }

    /// The run this screen reports, for the tests that check what the run put on it.
    #[cfg(test)]
    pub(crate) fn view(&self) -> &ResultView {
        &self.view
    }

    /// Put a run's measurements on the view, for the snapshots that check they are drawn.
    #[cfg(test)]
    pub(crate) fn set_measurements(&mut self, gauge_series: Vec<f32>, timing_hist: Box<[u32]>, judge_dist: [u32; 6]) {
        self.view.gauge_series = gauge_series;
        self.view.timing_hist = timing_hist;
        self.view.judge_dist = judge_dist;
    }
}

impl StageHandler for ResultState {
    fn update(&mut self, _ctx: &mut FrameCtx<'_>) -> Transition {
        Transition::Stay
    }

    /// Keys on the result screen: back to the browser, or straight into another run. The two
    /// run-again keys stand down for a run the player did not play, which has nothing to repeat.
    fn handle_key(&mut self, ctx: &mut FrameCtx<'_>, key: KeyInput<'_>) -> Transition {
        if !key.pressed {
            return Transition::Stay;
        }
        match key.code {
            KeyCode::Escape | KeyCode::Enter | KeyCode::NumpadEnter => ctx.shared.leave_play(),
            KeyCode::KeyR if offers_retry(ctx.shared) => retry(ctx.shared),
            KeyCode::KeyN if offers_retry(ctx.shared) => next_song(ctx.shared),
            _ => Transition::Stay,
        }
    }

    fn draw(&mut self, ctx: &mut FrameCtx<'_>, canvas: &mut Canvas<'_>) {
        canvas.clear_bga();
        ctx.shared.prepare_skin(canvas, SKIN_TYPE_RESULT);
        if ctx.shared.draw_result_skin(canvas, &self.view, self.extras.target.as_ref(), self.cleared) {
            return;
        }
        render_result_with_palette(canvas, &self.view, &ctx.shared.result_palette, &self.extras);
        for (i, (text, kind)) in ctx.shared.ir_status.lines().iter().enumerate() {
            draw_text(canvas, IR_RESULT_X, IR_RESULT_Y + i as f32 * IR_RESULT_LINE_H, IR_RESULT_SCALE, ir_line_color(*kind), text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::{FrameCtx, KeyInput};
    use crate::{App, Config, LaunchOptions};

    /// Where the chart the fixture run was played on sits.
    const PLAYED_PATH: &str = "/songs/played.bms";

    fn app() -> App {
        let dir = std::env::temp_dir().join(format!("rbms-result-stage-tests-{}", std::process::id()));
        let mut config = Config::default();
        config.play.autoplay = false;
        App::new(String::new(), config, LaunchOptions::default(), dir.join("settings.ron"))
    }

    /// The same app with the played chart in its library and a second chart after it, which is what
    /// puts the two run-again keys within reach.
    fn app_with_library() -> App {
        let mut app = app();
        let entry = |path: &str, md5: &str| rbms_library::SongEntry {
            path: PathBuf::from(path),
            title: md5.to_string(),
            subtitle: String::new(),
            artist: String::new(),
            genre: String::new(),
            maker: String::new(),
            level: String::new(),
            difficulty: 0,
            init_bpm: 120.0,
            rank: 2,
            total: 100.0,
            mode: Mode::BEAT_7K,
            md5: md5.to_string(),
            stagefile: String::new(),
            banner: String::new(),
            preview: String::new(),
        };
        app.shared.library = Library::from_songs(vec![entry(PLAYED_PATH, "played"), entry("/songs/after.bms", "after")]);
        app.shared.select_items = vec![SelectItem::Song(0), SelectItem::Song(1)];
        app.shared.chart_path = PLAYED_PATH.to_string();
        app
    }

    fn view() -> ResultView {
        ResultView {
            title: "run".into(),
            mode_label: "7K",
            counts: [1, 0, 0, 0, 0, 0],
            ex_score: 2,
            max_score: 2,
            max_combo: 1,
            total_notes: 1,
            fast: [0, 0],
            slow: [0, 0],
            gauge: 100.0,
            clear_label: "CLEAR",
            clear_color: Color::GREEN,
            prev_best_ex: None,
            prev_ex: None,
            show_graph: true,
            show_result_graphs: true,
            gauge_series: Vec::new(),
            timing_hist: Box::new([]),
            judge_dist: [0; 6],
        }
    }

    fn state() -> ResultState {
        ResultState::new(view())
    }

    fn press(state: &mut ResultState, app: &mut App, code: KeyCode) -> Transition {
        let now = std::time::Instant::now();
        state.handle_key(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, KeyInput { code, pressed: true, released: false, text: None })
    }

    #[test]
    fn leaving_keys_go_back_to_the_browser() {
        for code in [KeyCode::Escape, KeyCode::Enter, KeyCode::NumpadEnter] {
            let mut app = app();
            let mut state = state();
            assert!(!matches!(press(&mut state, &mut app, code), Transition::Stay), "{code:?} did not leave the result screen");
        }
    }

    /// Both run-again keys start a chart from the result screen, so the LOADING screen replaces it
    /// and the browser stays suspended underneath for the next run to return to.
    #[test]
    fn the_run_again_keys_start_another_run_without_going_back_to_the_browser() {
        for code in [KeyCode::KeyR, KeyCode::KeyN] {
            let mut app = app_with_library();
            let mut state = state();
            let moved = press(&mut state, &mut app, code);
            assert!(matches!(moved, Transition::To(Stage::Loading(_))), "{code:?} did not start a run");
        }
    }

    /// A run the player did not play has nothing to repeat, and a chart that is not in the library
    /// has nothing to start again from, so neither key may start a run in either case.
    #[test]
    fn the_run_again_keys_stand_down_for_a_run_that_cannot_be_repeated() {
        let mut demonstration = app_with_library();
        demonstration.shared.config.play.autoplay = true;
        let mut shown = state();
        assert!(matches!(press(&mut shown, &mut demonstration, KeyCode::KeyR), Transition::Stay));
        assert!(matches!(press(&mut shown, &mut demonstration, KeyCode::KeyN), Transition::Stay));

        let mut loose = app();
        let mut played = state();
        assert!(matches!(press(&mut played, &mut loose, KeyCode::KeyR), Transition::Stay), "there is no library to start from");
        assert!(matches!(press(&mut played, &mut loose, KeyCode::KeyN), Transition::Stay));
    }

    /// A key going back up is not a press, or every leaving key would fire twice.
    #[test]
    fn a_released_key_does_nothing() {
        let mut app = app();
        let mut state = state();
        let now = std::time::Instant::now();
        let released = KeyInput { code: KeyCode::Escape, pressed: false, released: true, text: None };
        let moved = state.handle_key(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, released);
        assert!(matches!(moved, Transition::Stay));
    }

    /// What surrounds the run reaches the screen: a screen built with a target and the run-again
    /// offer cannot paint the same frame as the bare one.
    #[test]
    fn the_target_and_the_run_again_offer_are_carried_onto_the_screen() {
        use crate::stage::{Canvas, HeadlessCanvas};
        use rbms_render::result::TargetView;
        let mut app = app_with_library();
        let paint = |app: &mut App, state: &mut ResultState| {
            rbms_render::font::use_embedded_fonts_only();
            let mut pixels = HeadlessCanvas::new(crate::CW, crate::CH);
            let mut canvas = Canvas::Headless(&mut pixels);
            let now = std::time::Instant::now();
            app.shared.hot.clear();
            state.draw(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, &mut canvas);
            pixels.signature()
        };
        let bare = paint(&mut app, &mut state());
        let extras = ResultExtras { target: Some(TargetView { name: "RANK AAA".into(), ex: 2 }), run_again: true };
        let paced = paint(&mut app, &mut ResultState::new(view()).paced_by(extras));
        assert_ne!(bare, paced, "the target and the run-again hint are not drawn");
    }
}
