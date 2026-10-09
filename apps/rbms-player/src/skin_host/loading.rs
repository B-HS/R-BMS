//! Property cluster M: the load in progress.
//!
//! Where the values come from: how far the chart and its sounds and images have loaded, and
//! whether the player screen has finished preloading.
//!
//! What this cluster answers: number 165; float 165; rate 102; options 80-81.
//!
//! The progress is read as `BMSResource` reports it, on every screen: the share of the chart's
//! sounds (and, with a BGA, its images) that are in. The two options are about the player screen
//! alone (`BMSPlayer.getState()`): on any other screen both are off, and negating them is on.

use rbms_skin::property::generated::*;

use super::ClusterState;

/// How a progress share becomes the whole percent a skin shows.
const PROGRESS_PERCENT_SCALE: f32 = 100.0;

/// Which screen the load belongs to, as far as the two loading options are concerned.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LoadingScreen {
    /// Nothing is connected: this cluster knows nothing, so another source may answer.
    #[default]
    Unconnected,
    /// A screen other than the player, on which neither option is on.
    Elsewhere,
    /// The player screen, still preloading (`BMSPlayer.STATE_PRELOAD`).
    Preload,
    /// The player screen, past its preload.
    Started,
}

/// What this cluster reads from the running game.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct LoadingState {
    pub screen: LoadingScreen,
    /// How much of the load is done, from nothing (0) to everything (1).
    pub progress: f32,
}

impl LoadingState {
    fn is_connected(&self) -> bool {
        self.screen != LoadingScreen::Unconnected
    }
}

impl ClusterState for LoadingState {
    fn boolean(&self, id: i32) -> Option<bool> {
        match id {
            OPTION_NOW_LOADING => self.is_connected().then_some(self.screen == LoadingScreen::Preload),
            OPTION_LOADED => self.is_connected().then_some(self.screen == LoadingScreen::Started),
            _ => None,
        }
    }

    fn integer(&self, id: i32) -> Option<i32> {
        match id {
            NUMBER_LOADING_PROGRESS => self.is_connected().then_some((self.progress * PROGRESS_PERCENT_SCALE) as i32),
            _ => None,
        }
    }

    fn rate(&self, id: i32) -> Option<f32> {
        match id {
            RATE_LOAD_PROGRESS => self.is_connected().then_some(self.progress),
            _ => None,
        }
    }

    fn float(&self, id: i32) -> Option<f32> {
        match id {
            FLOAT_LOADING_PROGRESS => self.is_connected().then_some(self.progress),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests;
