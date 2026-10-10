//! Playing the movies the compiled screens draw from.
//!
//! A screen says which movies it draws from and when each was started
//! ([`SkinScreen::movies`]): the first time one of its objects was prepared, on the scene clock.
//! This module does the rest. A movie that has been started is given a player -- a decoder on a
//! thread of its own ([`VideoPlayer`]) -- and on every frame the player is told how far into the
//! movie the scene clock is, and the frame that is due is handed to the screen when it is a new one.
//!
//! The scene clock is the only clock a movie has. While it stands still -- a document on its way,
//! or a screen parked under another -- nothing asks for a frame, the player's queue stays full and
//! its thread sleeps, so the movie stands still with the scene and picks up where it was. When it
//! jumps -- a replay being stepped through, back or ahead -- the player is told the new time like
//! any other and finds the frame due then; the frame that was on show stays until it has.
//!
//! A screen that is let go of takes its players with it: their threads are stopped and joined and
//! their files closed before the screen's textures are handed back.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use rbms_render::{Renderer, SkinScreen};
use rbms_video::{VideoDecoder, VideoPlayer};

use crate::notify::{Level, notify};

/// The most movies that are decoded at one time, over every compiled screen. A published skin
/// plays one on a screen; this is what keeps a skin that names a movie for every panel from
/// starting a decoder thread for each.
pub(crate) const PLAYING_MOVIES_LIMIT: usize = 4;

/// One movie of one compiled screen.
#[derive(Default)]
struct ScreenMovie {
    /// The movie as the worker opened it, until something starts playing it.
    decoder: Option<Box<dyn VideoDecoder>>,
    player: Option<VideoPlayer>,
    /// How many times the screen had started the movie when the player was last told to
    /// ([`rbms_render::skin_render::textures::MoviePlayback::starts`]).
    starts: u64,
    /// Whether the movie was given up on: it would not decode, or there were too many playing.
    given_up: bool,
    /// Whether it has been said that the movie is damaged.
    said_damaged: bool,
}

/// The players of every compiled screen's movies.
#[derive(Default)]
pub(crate) struct MoviePlayers {
    /// Each screen's movies, in the order the screen numbers them.
    screens: BTreeMap<i32, Vec<ScreenMovie>>,
    /// How long a frame waits for the frame of a movie that is due, when it waits at all.
    patience: Option<Duration>,
}

impl MoviePlayers {
    /// Takes the movies `compiled` draws from, each as the decoder the worker opened for its file.
    /// A movie that was already playing for an earlier compile of `screen` is stopped.
    pub(super) fn adopt(&mut self, screen: i32, compiled: &SkinScreen, mut decoders: BTreeMap<PathBuf, Box<dyn VideoDecoder>>) {
        let movies: Vec<ScreenMovie> =
            compiled.movies().into_iter().map(|movie| ScreenMovie { decoder: decoders.remove(&movie.path), ..ScreenMovie::default() }).collect();
        if movies.is_empty() {
            self.screens.remove(&screen);
        } else {
            self.screens.insert(screen, movies);
        }
    }

    /// Stops every movie of `screen` and closes its file. The decoder threads have ended by the time
    /// this returns.
    pub(super) fn let_go(&mut self, screen: i32) {
        self.screens.remove(&screen);
    }

    /// How many movies are being decoded right now.
    pub(crate) fn playing(&self) -> usize {
        self.screens.values().flatten().filter(|movie| movie.player.is_some()).count()
    }

    /// When the frame of each playing movie that is on show is due, in microseconds since its movie
    /// was started, for the captures that say which frame they caught.
    #[cfg(test)]
    pub(crate) fn frames_on_show(&self) -> Vec<i64> {
        self.screens.values().flatten().filter_map(|movie| Some(movie.player.as_ref()?.frame()?.time_us)).collect()
    }

    /// Makes every frame wait up to `patience` for the frame of a movie that is due at its scene
    /// time, instead of showing whatever has been decoded so far: for a capture, which draws one
    /// frame and has to have the right one. `None` is how the application runs, never waiting.
    #[cfg(test)]
    pub(crate) fn wait_for_frames(&mut self, patience: Option<Duration>) {
        self.patience = patience;
    }

    /// Brings the movies of `screen` up to `now_us` on the scene clock and hands `compiled` the
    /// frame of each that is newly due.
    ///
    /// A movie the screen has started and that has no player yet is given one here, which is where
    /// its first frame begins to be decoded: the screen draws nothing for it until that frame has
    /// arrived. A movie the screen started over is played from its first frame again. A movie with
    /// damage in it is said once and played around the damage. A movie that stops decoding is said
    /// once and drawn no more, from the moment the last frame it did decode has had its turn.
    pub(super) fn show<R: Renderer>(&mut self, screen: i32, compiled: &SkinScreen, r: &mut R, now_us: i64) {
        let mut playing = self.playing();
        let patience = self.patience;
        let Some(movies) = self.screens.get_mut(&screen) else {
            return;
        };
        for playback in compiled.movies() {
            let (Some(movie), Some(started_us)) = (movies.get_mut(playback.index), playback.started_us) else {
                continue;
            };
            if movie.given_up {
                continue;
            }
            let name = playback.path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
            if movie.player.is_none() {
                let Some(decoder) = movie.decoder.take() else {
                    movie.given_up = true;
                    continue;
                };
                if playing >= PLAYING_MOVIES_LIMIT {
                    movie.given_up = true;
                    notify(Level::Warn, format!("skin: {name} is not played: {PLAYING_MOVIES_LIMIT} movies are playing already"));
                    continue;
                }
                match VideoPlayer::spawn(decoder) {
                    Ok(player) => {
                        movie.player = Some(player);
                        movie.starts = playback.starts;
                        playing += 1;
                    }
                    Err(error) => {
                        movie.given_up = true;
                        notify(Level::Warn, format!("skin: {name} is not played: {error}"));
                        continue;
                    }
                }
            }
            let Some(player) = movie.player.as_mut() else {
                continue;
            };
            if movie.starts != playback.starts {
                movie.starts = playback.starts;
                player.restart();
            }
            let into_movie_us = now_us.saturating_sub(started_us).max(0);
            let changed = match patience {
                Some(patience) => player.advance_blocking(into_movie_us, patience),
                None => player.advance(into_movie_us),
            };
            if changed && let Some(frame) = player.frame() {
                compiled.show_movie_frame(r, playback.index, &frame.rgba);
            }
            if !movie.said_damaged && player.lost_samples() > 0 {
                movie.said_damaged = true;
                notify(Level::Warn, format!("skin: {name} is damaged: the frames that do not decode are left out"));
            }
            if player.has_ended() {
                let reason = player.failure().unwrap_or_default();
                notify(Level::Warn, format!("skin: {name} stopped playing: {reason}"));
                compiled.hide_movie(playback.index);
                movie.player = None;
                movie.given_up = true;
            }
        }
    }
}
