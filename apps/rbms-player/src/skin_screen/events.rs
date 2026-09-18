//! What one of a document's own events asks of the player, once the document has run its own half.
//!
//! [`rbms_render::skin_render::events`] answers everything a document can answer by itself: its
//! timers, its expressions, its own numbered events. What is left is one of the reference's own
//! event numbers, and this is the one place that says what this build does with each of them --
//! step a configuration row, or hand the browser the action it already has a button for.
//!
//! Split from its parent module because it is the only part of a document's frame that reaches the
//! configuration and the stages rather than the renderer.

use rbms_config::{AdjustOutcome, SettingId, adjust};
use rbms_render::SkinExprEval;
use rbms_render::skin_render::events::{DocumentEvents, SkinEventClick, SkinEventFrame, SkinEventRequest};
use rbms_render::skin_render::state::SelectViewState;
use rbms_skin::dst::WarnOnce;
use rbms_skin::loader::{LoadedSkin, SKIN_TYPE_MUSIC_SELECT};
use rbms_skin::property::SkinStateSource;
use rbms_skin::timer::TimerState;

use super::SkinSandboxFrame;
use crate::notify::{Level, notify};
use crate::{AppShared, Hot, SelectScene};

/// The configuration row each of the reference's own numbered events steps.
///
/// One entry per event that is a settings row in this build and nothing more, so a document that
/// draws its own gauge or hi-speed control moves the same field the settings screen and the option
/// panel move. The step the click carried is the step the row takes, which is why an event fired by
/// a condition -- carrying a step of zero -- leaves the row where it is.
const EVENT_SETTING_ROWS: &[(i32, SettingId)] = &[
    (16, SettingId::Autoplay),
    (40, SettingId::Gauge),
    (42, SettingId::Random),
    (55, SettingId::FixHiSpeed),
    (57, SettingId::HiSpeed),
    (72, SettingId::Bga),
    (77, SettingId::Target),
    (308, SettingId::LnMode),
    (330, SettingId::LaneCover),
    (331, SettingId::Lift),
    (332, SettingId::Hidden),
    (340, SettingId::JudgeAlgorithm),
];

/// The browser action each of the reference's own numbered events stands for.
///
/// These are the events that move the player somewhere rather than change a value, so they are
/// answered with the same [`Hot`] the browser's own buttons produce and carried out where a
/// transition can be returned. Both of the reference's numbers for the sort order are here, and all
/// four of its replay slots, because this build keeps one of each.
const EVENT_BROWSER_ACTIONS: &[(i32, Hot)] = &[
    (12, Hot::NavSort),
    (312, Hot::NavSort),
    (13, Hot::NavKeyConfig),
    (14, Hot::NavSkinConfig),
    (15, Hot::SelectPlay),
    (315, Hot::SelectPractice),
    (19, Hot::ModalReplay),
    (316, Hot::ModalReplay),
    (317, Hot::ModalReplay),
    (318, Hot::ModalReplay),
    (89, Hot::SelectFavorite),
    (90, Hot::SelectFavorite),
    (210, Hot::NavRanking),
    (211, Hot::NavRescan),
];

/// Fires once when a document names one of the reference's events that this build has no action
/// for, so a document written against every one of them says so instead of failing quietly.
static UNANSWERED_EVENT: WarnOnce = WarnOnce::new();

/// Fires once when an event that fired by its own condition asked to leave the screen.
static CONDITION_LEAVES_SCREEN: WarnOnce = WarnOnce::new();

impl AppShared {
    /// One frame of the document's own timers and events, run against `state`.
    ///
    /// `None` when no document is compiled for `screen`, which is the caller's cue that there is
    /// nothing of the document's own to run.
    fn with_skin_events<T>(
        &mut self,
        screen: i32,
        state: &dyn SkinStateSource,
        run: impl FnOnce(&DocumentEvents, &mut TimerState, &SkinEventFrame<'_>) -> T,
    ) -> Option<T> {
        let events = self.skin_screens.events(screen)?;
        let sandbox = self.skins.document(screen).and_then(LoadedSkin::lua);
        let lua = sandbox.map(|sandbox| SkinSandboxFrame::new(sandbox, state));
        let frame = SkinEventFrame { now_ms: state.now_ms(), state, lua: lua.as_ref().map(|frame| frame as &dyn SkinExprEval) };
        Some(run(events, &mut self.skin_timers, &frame))
    }

    /// Carries out one numbered event a document fired.
    ///
    /// Two kinds of answer. A configuration row is stepped here and written out, because that is
    /// the whole of what the event asks for. Anything that moves the screen the player is on is
    /// answered with the browser's own action instead, for the caller to carry out where it can
    /// return a transition.
    pub(crate) fn dispatch_skin_event(&mut self, request: SkinEventRequest) -> Option<Hot> {
        if let Some((_, row)) = EVENT_SETTING_ROWS.iter().find(|(event, _)| *event == request.id) {
            if adjust(&mut self.config, *row, request.step) == AdjustOutcome::Changed {
                self.save_settings();
            }
            return None;
        }
        if let Some((_, hot)) = EVENT_BROWSER_ACTIONS.iter().find(|(event, _)| *event == request.id) {
            return Some(*hot);
        }
        if UNANSWERED_EVENT.should_warn() {
            notify(Level::Warn, format!("skin: event {} is not one this build has an action for", request.id));
        }
        None
    }

    /// Runs the browser document's own timers and conditional events for this frame.
    ///
    /// Called where the browser is drawn rather than where it handles a key, because a document's
    /// timers have to have been computed before the frame that animates against them is drawn.
    ///
    /// An event that fired by itself and asks to leave the browser is reported rather than carried
    /// out: a frame being drawn has nothing to return a transition to, and opening a screen from
    /// under the draw would leave the browser half-drawn behind it.
    pub(crate) fn update_select_skin_events(&mut self, view: &SelectScene) {
        if self.skin_screens.events(SKIN_TYPE_MUSIC_SELECT).is_none_or(DocumentEvents::is_empty) {
            return;
        }
        let options = crate::app_options::options_rows(self);
        let now_ms = self.skin_now_ms();
        let state = SelectViewState::new(view, now_ms, None, Some(&options));
        let Some(requests) = self.with_skin_events(SKIN_TYPE_MUSIC_SELECT, &state, |events, timers, frame| events.update(timers, frame)) else {
            return;
        };
        let left = requests.into_iter().filter_map(|request| self.dispatch_skin_event(request)).count();
        if left > 0 && CONDITION_LEAVES_SCREEN.should_warn() {
            notify(Level::Warn, "skin: an event fired by its own condition asked to leave the browser, which only a click can do".to_owned());
        }
    }

    /// Answers one click that landed on a rectangle the browser document offered.
    ///
    /// The document answers what it can -- its own events, its own timers -- and what is left is
    /// the browser action the click stands for, or nothing at all.
    pub(crate) fn select_skin_click(&mut self, view: &SelectScene, click: SkinEventClick) -> Option<Hot> {
        let options = crate::app_options::options_rows(self);
        let now_ms = self.skin_now_ms();
        let state = SelectViewState::new(view, now_ms, None, Some(&options));
        let requests = self.with_skin_events(SKIN_TYPE_MUSIC_SELECT, &state, |events, timers, frame| events.click(click, timers, frame))?;
        let mut hot = None;
        for request in requests {
            hot = hot.or(self.dispatch_skin_event(request));
        }
        hot
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An app on a temp settings directory, so a test that steps a configuration row writes somewhere
    /// disposable rather than over the player's own settings.
    fn app() -> crate::App {
        let dir = std::env::temp_dir().join(format!("rbms-skin-events-{}-{:?}", std::process::id(), std::thread::current().id()));
        let path = dir.join("settings.ron");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a temp dir");
        crate::App::new(String::new(), crate::Config::default(), crate::LaunchOptions::default(), path)
    }

    /// A document fires the reference's own numbered events, and this build answers them in one of two
    /// ways: by stepping the configuration row the event is, or by handing back the browser action it
    /// stands for. An event that is neither is warned about rather than quietly dropped.
    #[test]
    fn a_numbered_event_reaches_either_a_configuration_row_or_a_browser_action() {
        let mut app = app();
        let fire = |app: &mut crate::App, id: i32, step: i32| app.shared.dispatch_skin_event(SkinEventRequest { id, step });

        assert_eq!(fire(&mut app, 12, 1), Some(Hot::NavSort), "the sort order is the browser's own action");
        assert_eq!(fire(&mut app, 312, 1), Some(Hot::NavSort), "and the reference's second number for it answers the same");
        assert_eq!(fire(&mut app, 315, 1), Some(Hot::SelectPractice), "the practice panel is one too");

        let autoplay = app.shared.config.play.autoplay;
        assert_eq!(fire(&mut app, 16, 1), None, "a configuration row is stepped here rather than handed back");
        assert_ne!(app.shared.config.play.autoplay, autoplay, "and the row actually moved");

        let hispeed = app.shared.config.play.hispeed;
        assert_eq!(fire(&mut app, 57, -1), None);
        assert!(app.shared.config.play.hispeed < hispeed, "the step the click carried is the step the row takes");

        let held = app.shared.config.play.hispeed;
        assert_eq!(fire(&mut app, 57, 0), None);
        assert_eq!(app.shared.config.play.hispeed, held, "an event fired by its own condition carries no step, so the row stays put");

        assert_eq!(fire(&mut app, 213, 1), None, "an event this build has no action for is answered by nothing at all");
    }
}
