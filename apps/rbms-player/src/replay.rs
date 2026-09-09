use std::path::Path;

use serde::{Deserialize, Serialize};

/// One recorded input: a press or release of `lane` at song time `t` (µs, raw — no offset).
#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct ReplayEvent {
    pub t: i64,
    pub lane: usize,
    pub press: bool,
}

/// A recorded interactive play: the chart it was played on, the exact note-shuffle that was
/// used (random + seed → identical lane layout on playback), the judge offset that was active,
/// and the timestamped input stream.
#[derive(Clone, Serialize, Deserialize)]
pub struct Replay {
    pub chart_path: String,
    pub md5: String,
    pub mode: String,
    pub random: String,
    pub seed: u64,
    pub offset_ms: i32,
    #[serde(default)]
    pub scratch_auto: bool,
    #[serde(default)]
    pub gauge: String,
    pub events: Vec<ReplayEvent>,
}

impl Replay {
    pub fn load(path: &Path) -> Result<Replay, String> {
        let s = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        ron::from_str(&s).map_err(|e| e.to_string())
    }

    pub fn save(&self, path: &Path) {
        match ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()) {
            Ok(s) => match crate::write_atomic(path, &s) {
                Ok(()) => println!("replay saved: {}", path.display()),
                Err(e) => eprintln!("replay write failed ({}): {e}", path.display()),
            },
            Err(e) => eprintln!("replay save failed: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Replay {
        Replay {
            chart_path: "/songs/a.bms".into(),
            md5: "DEADBEEF".into(),
            mode: "BEAT-7K".into(),
            random: "RANDOM".into(),
            seed: 0xDEAD_BEEF_CAFE_F00D,
            offset_ms: -12,
            scratch_auto: true,
            gauge: "HARD".into(),
            events: vec![
                ReplayEvent { t: 0, lane: 0, press: true },
                ReplayEvent { t: 1500, lane: 0, press: false },
                ReplayEvent { t: 2000, lane: 7, press: true },
            ],
        }
    }

    #[test]
    fn ron_round_trip_preserves_events_and_meta() {
        let r = sample();
        let s = ron::ser::to_string_pretty(&r, ron::ser::PrettyConfig::default()).unwrap();
        let back: Replay = ron::from_str(&s).unwrap();
        assert_eq!(back.chart_path, r.chart_path);
        assert_eq!(back.md5, r.md5);
        assert_eq!(back.seed, r.seed, "u64 seed survives exactly");
        assert_eq!(back.offset_ms, r.offset_ms);
        assert!(back.scratch_auto);
        assert_eq!(back.gauge, "HARD");
        assert_eq!(back.events.len(), r.events.len(), "event count preserved");
        for (a, b) in r.events.iter().zip(&back.events) {
            assert_eq!((a.t, a.lane, a.press), (b.t, b.lane, b.press));
        }
    }

    #[test]
    fn empty_event_stream_round_trips() {
        let mut r = sample();
        r.events.clear();
        let s = ron::ser::to_string_pretty(&r, ron::ser::PrettyConfig::default()).unwrap();
        let back: Replay = ron::from_str(&s).unwrap();
        assert!(back.events.is_empty(), "an empty replay is valid");
    }

    #[test]
    fn scratch_auto_and_gauge_default_when_absent() {
        // Replays written before scratch_auto/gauge existed must still load (serde default).
        let s = r#"(
            chart_path: "/c.bms", md5: "AA", mode: "BEAT-7K", random: "OFF",
            seed: 7, offset_ms: 0, events: []
        )"#;
        let r: Replay = ron::from_str(s).expect("back-compat replay parses");
        assert!(!r.scratch_auto, "scratch_auto defaults to false");
        assert_eq!(r.gauge, "", "gauge defaults to empty string");
        assert_eq!(r.seed, 7);
    }

    #[test]
    fn load_missing_file_is_err() {
        let path = std::env::temp_dir().join(format!("rbms_replay_missing_{}.ron", std::process::id()));
        let _ = std::fs::remove_file(&path);
        assert!(Replay::load(&path).is_err(), "missing replay file is an Err, not a default");
    }

    #[test]
    fn load_malformed_file_is_err() {
        let dir = std::env::temp_dir().join(format!("rbms_replay_bad_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bad.ron");
        std::fs::write(&path, "<<< not ron >>>").unwrap();
        assert!(Replay::load(&path).is_err(), "malformed replay is an Err");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_then_load_round_trips_on_disk() {
        let dir = std::env::temp_dir().join(format!("rbms_replay_io_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("nested/r.ron");
        sample().save(&path); // creates nested dir
        assert!(path.exists());
        let back = Replay::load(&path).expect("saved replay loads back");
        assert_eq!(back.events.len(), 3);
        assert_eq!(back.seed, 0xDEAD_BEEF_CAFE_F00D);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
