//! Playing the focused chart laid out as a replay of it was, from the browser (LAYOUT PLAY): the
//! record modal's button and key take the replay a run of this machine kept, and the ranking panel's
//! key takes the replay behind a row of the leaderboard, which is downloaded first. Either starts a
//! run played by hand on the replay's lane arrangement and seed ([`crate::borrowed_layout`]).
//!
//! A layout that is refused -- a replay of another chart, one that does not state how its lanes were
//! laid out, a course or practice in the way -- is said so in a notice and leaves the browser, and
//! the modal over it, where they were.

use rbms_ir::ReplayData;
use rbms_model::translations::Catalog;
use rbms_render::text::language;
use rbms_store::Replay;

use super::SelectState;
use crate::AppShared;
use crate::borrowed_layout::{LayoutRefusal, LayoutRequest};
use crate::notify::{Level, notify};
use crate::stage::Transition;
use crate::syssound::SystemSound;
use crate::text::layout_play::LayoutPlayText;

impl SelectState {
    /// Play the focused chart laid out as the replay of the record the modal shows, and held
    /// against that run: a record of this machine that kept a replay it can read. A record that kept
    /// none has nothing to lend and nothing happens.
    pub(super) fn play_record_layout(&mut self, shared: &mut AppShared) -> Transition {
        let Some(row) = self.record_modal else {
            return Transition::Stay;
        };
        let (Some(md5), Some(chart_index)) = (shared.focused_md5(), shared.focused_song_index()) else {
            return Transition::Stay;
        };
        let Some(record) = shared.scores.for_md5(&md5).get(row).map(|record| (*record).clone()) else {
            return Transition::Stay;
        };
        let Some(file) = record.replay_file.as_deref() else {
            return Transition::Stay;
        };
        let replay = match Replay::load(&shared.replay_dir.join(file)) {
            Ok(replay) => replay,
            Err(error) => {
                notify(Level::Warn, LayoutPlayText::ReplayNotRead.fill(language(), &[&error]));
                return Transition::Stay;
            }
        };
        match shared.borrow_layout_of_kept_run(&record, &replay, chart_index) {
            Ok(started) => {
                self.record_modal = None;
                shared.replay_download_rx = None;
                shared.replay_download_target = None;
                shared.play_system_sound(SystemSound::Select);
                started
            }
            Err(refusal) => {
                notify(Level::Warn, refusal.to_string());
                Transition::Stay
            }
        }
    }

    /// Play the chart the ranking row `player` belongs to laid out as the replay downloaded for it
    /// was ([`SelectState::play_ranking_replay`] asks for it). The chart is the library's of the
    /// path the row was asked for from, the one of its hash when the library has since changed.
    ///
    /// The replay may have come from anywhere a score server has them: it is held to name the chart
    /// it is of, when it names one, and its arrangement is read as the wire names it
    /// ([`LayoutRequest::of_downloaded_replay`]).
    pub(super) fn start_downloaded_layout(&mut self, shared: &mut AppShared, data: &ReplayData, chart_path: &str, md5: &str, player: String) -> Transition {
        let songs = shared.library.songs();
        let chart_index = songs.iter().position(|entry| entry.path.to_string_lossy() == chart_path).or_else(|| songs.iter().position(|entry| entry.md5 == md5));
        let Some(chart_index) = chart_index else {
            notify(Level::Warn, LayoutRefusal::ChartMismatch { replay: md5.to_string(), chart: String::new() }.to_string());
            return Transition::Stay;
        };
        match shared.borrow_layout(LayoutRequest::of_downloaded_replay(data, md5, player), chart_index) {
            Ok(started) => {
                self.ranking_open = false;
                self.record_modal = None;
                shared.play_system_sound(SystemSound::Select);
                started
            }
            Err(refusal) => {
                notify(Level::Warn, refusal.to_string());
                Transition::Stay
            }
        }
    }
}
