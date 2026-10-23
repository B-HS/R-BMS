//! How far down the ranking of the chart under the cursor the list of scores a skin draws is
//! scrolled (`MusicSelector.rankingOffset`).
//!
//! The offset is the reference's: a count of scores down the ranking, written through the slider
//! that the skin binds to `ranking_position` (rate 8) as the share `0..1` of the scores received so
//! far, and read back as that share ([`crate::skin_host::ir::IrBrowser`]). The reference never
//! puts the offset back, so it stays where it was when the cursor goes on to another chart, over a
//! ranking that is not the one it was set for; here it starts again from the top on every chart.
//!
//! The service pages its ranking, so a list scrolled to the end of the scores received asks for the
//! page after them ([`SelectState::grow_scrolled_ranking`]), and the list goes on once it arrives.
//! The scores wait in the ranking cache, where the ranking mode and the panel read them from too.

use super::SelectState;
use crate::ir_ranking::RankingState;
use crate::skin_host::ir::RANKING_SLOTS;
use crate::skin_host::writers::scaled_index;
use crate::{AppShared, SelectTab};

/// The offset of the ranking list and the chart it was set for.
#[derive(Debug, Default)]
pub(super) struct RankingScroll {
    chart: Option<String>,
    offset: usize,
}

impl RankingScroll {
    /// How many scores down the ranking the list is scrolled.
    pub(super) fn offset(&self) -> usize {
        self.offset
    }

    /// Take `chart` as the chart the ranking belongs to: a chart other than the one the list was
    /// scrolled for is read from its top.
    pub(super) fn settle(&mut self, chart: Option<&str>) {
        if self.chart.as_deref() != chart {
            self.chart = chart.map(str::to_owned);
            self.offset = 0;
        }
    }
}

impl SelectState {
    /// Scroll the list of scores to the place `share` of the way down the scores received so far
    /// (`MusicSelector.setRankingPosition`). A share outside `0..1` leaves the list where it is.
    pub(super) fn scroll_ranking_to_share(&mut self, shared: &AppShared, share: f32) {
        if self.tab != SelectTab::Songs {
            return;
        }
        self.browse.ranking.settle(self.ranked_chart(shared));
        if let Some(offset) = scaled_index(share, shared.ranking_list_extent()) {
            self.browse.ranking.offset = offset;
        }
    }

    /// Ask the service for the next page of the ranking while the list a skin draws reaches the end
    /// of the scores received. Nothing is asked of a list at the top, of a ranking the service has no
    /// more of, or while another request is on its way: a list left at the end asks again on the
    /// next frame.
    pub(super) fn grow_scrolled_ranking(&mut self, shared: &mut AppShared) {
        if self.tab != SelectTab::Songs {
            return;
        }
        self.browse.ranking.settle(self.ranked_chart(shared));
        let offset = self.browse.ranking.offset;
        if offset == 0 {
            return;
        }
        let Some(chart) = shared.focused_chart_id() else {
            return;
        };
        let Some(RankingState::Ready(board)) = shared.ranking_cache.peek(&chart.md5) else {
            return;
        };
        if offset.saturating_add(RANKING_SLOTS) < board.rows.len() {
            return;
        }
        let next = board.pages + 1;
        shared.start_ranking_page(chart, next);
    }
}

#[cfg(test)]
mod tests;
