//! The clock of LAYOUT PICK's preview: where in the chart the judgement line stands, moved on by
//! the time each frame was up while the preview plays.
//!
//! It counts the screen's own frames, not the song: the preview makes no sound, and a frame handed
//! no time (as a test hands it) leaves the chart where it stands. A preview that runs past the end of
//! its chart begins it again.

/// Microseconds in a second, which a frame's time is handed in.
const MICROS_PER_SECOND: f32 = 1_000_000.0;

/// How long after the chart's last timeline the preview goes on before it begins the chart again,
/// in microseconds: long enough for the last notes to be seen down to the judgement line.
pub(crate) const END_TAIL_US: i64 = 1_000_000;

/// Where the preview stands in its chart, and whether it moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PreviewClock {
    /// Chart time at the judgement line, in microseconds.
    at_us: i64,
    playing: bool,
}

impl Default for PreviewClock {
    /// The top of the chart, playing.
    fn default() -> PreviewClock {
        PreviewClock { at_us: 0, playing: true }
    }
}

impl PreviewClock {
    /// Chart time at the judgement line.
    pub(crate) fn at_us(self) -> i64 {
        self.at_us
    }

    pub(crate) fn is_playing(self) -> bool {
        self.playing
    }

    /// Play a stopped preview, or stop a playing one where it stands.
    pub(crate) fn toggle(&mut self) {
        self.playing = !self.playing;
    }

    /// Move a playing preview on by a frame of `dt_seconds`, and back to the top once it is past
    /// `end_us`.
    pub(crate) fn advance(&mut self, dt_seconds: f32, end_us: i64) {
        if !self.playing {
            return;
        }
        self.at_us = self.at_us.saturating_add((dt_seconds.max(0.0) * MICROS_PER_SECOND) as i64);
        if self.at_us > end_us {
            self.at_us = 0;
        }
    }

    /// Put the judgement line at `at_us`, playing or not as it was.
    pub(crate) fn seek(&mut self, at_us: i64) {
        self.at_us = at_us.max(0);
    }

    /// The measure the judgement line is in, among measures starting at `starts`.
    pub(crate) fn measure(self, starts: &[i64]) -> usize {
        starts.partition_point(|start| *start <= self.at_us).saturating_sub(1)
    }

    /// Put the judgement line at the start of the measure `by` measures on from the one it is in,
    /// never before the first nor past the last.
    pub(crate) fn move_measures(&mut self, starts: &[i64], by: i64) {
        let Some(last) = starts.len().checked_sub(1) else {
            return;
        };
        let at = i64::try_from(self.measure(starts)).unwrap_or(i64::MAX).saturating_add(by).max(0);
        let to = usize::try_from(at).unwrap_or(usize::MAX).min(last);
        self.seek(starts[to]);
    }
}

/// Where the preview of a chart whose last timeline is at `last_us` begins again.
pub(crate) fn end_of(last_us: i64) -> i64 {
    last_us.saturating_add(END_TAIL_US)
}
