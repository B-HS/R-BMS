//! The clock a scene's skin is animated against.

use std::time::{Duration, Instant};

/// How long a scene has run: what the clock read when it was last put somewhere, and the instant
/// that was.
///
/// A scene is put at a time as often as it is begun. A scene held still for its document is set
/// going where it stood, a parked one is put back, and a replay being paused or scrubbed carries
/// its scene along with the chart. None of that is kept as the instant the scene would have begun
/// at, because that instant need not be one the machine can name: Windows counts its instants from
/// the moment the machine came up, so there is none earlier than that, and a scene put further on
/// than the machine has been up would have nowhere to have begun.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SceneClock {
    /// What the clock read at [`SceneClock::read_at`].
    read: Duration,
    /// The instant the clock was last put somewhere.
    read_at: Instant,
}

impl SceneClock {
    /// A clock that reads `elapsed` at `now` and runs on from there.
    pub(crate) fn at(now: Instant, elapsed: Duration) -> Self {
        Self { read: elapsed, read_at: now }
    }

    /// A clock at a scene's first moment: it reads nothing at `now`.
    pub(crate) fn begun(now: Instant) -> Self {
        Self::at(now, Duration::ZERO)
    }

    /// What the clock reads at `now`. An instant from before the clock was put where it is reads
    /// as the one it was put at.
    pub(crate) fn elapsed(&self, now: Instant) -> Duration {
        self.read.saturating_add(now.saturating_duration_since(self.read_at))
    }

    /// The same clock `by` further on.
    #[cfg(test)]
    pub(crate) fn aged(self, by: Duration) -> Self {
        Self { read: self.read.saturating_add(by), ..self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// How long the clock is left to run in the tests below.
    const RUN: Duration = Duration::from_millis(250);

    /// A scene time no machine has been up for: no instant lies that far back on any platform.
    const BEYOND_ANY_UPTIME: Duration = Duration::from_secs(u64::MAX);

    #[test]
    fn a_clock_reads_what_it_was_put_at_and_runs_on_from_there() {
        const PUT_AT: Duration = Duration::from_secs(90);
        let now = Instant::now();
        let clock = SceneClock::at(now, PUT_AT);
        assert_eq!(clock.elapsed(now), PUT_AT);
        assert_eq!(clock.elapsed(now + RUN), PUT_AT + RUN);
        assert_eq!(SceneClock::begun(now).elapsed(now), Duration::ZERO);
        assert_eq!(SceneClock::begun(now).elapsed(now + RUN), RUN);
    }

    #[test]
    fn a_scene_is_put_further_on_than_the_machine_has_been_up() {
        let now = Instant::now();
        assert_eq!(now.checked_sub(BEYOND_ANY_UPTIME), None, "the fixture names a time the machine could have begun a scene at");
        let clock = SceneClock::at(now, BEYOND_ANY_UPTIME);
        assert_eq!(clock.elapsed(now), BEYOND_ANY_UPTIME, "a scene put past the machine's own start read as something else");
        assert_eq!(clock.elapsed(now + RUN), BEYOND_ANY_UPTIME + RUN);
        assert_eq!(SceneClock::begun(now).aged(BEYOND_ANY_UPTIME).elapsed(now), BEYOND_ANY_UPTIME, "ageing a scene is held to the machine's uptime");
    }

    #[test]
    fn ageing_adds_to_what_the_clock_already_reads() {
        const FIRST: Duration = Duration::from_secs(3);
        const SECOND: Duration = Duration::from_secs(3600);
        let now = Instant::now();
        let clock = SceneClock::begun(now).aged(FIRST).aged(SECOND);
        assert_eq!(clock.elapsed(now + RUN), FIRST + SECOND + RUN);
    }

    #[test]
    fn an_instant_from_before_the_clock_was_put_reads_as_the_one_it_was_put_at() {
        const PUT_AT: Duration = Duration::from_secs(5);
        let earlier = Instant::now();
        let clock = SceneClock::at(earlier + RUN, PUT_AT);
        assert_eq!(clock.elapsed(earlier), PUT_AT);
    }
}
