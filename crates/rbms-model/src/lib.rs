pub mod mode;

pub use mode::Mode;

/// Absolute time, microseconds. The global time unit across rbms.
pub type Micros = i64;

/// Microseconds per 4/4 measure at the given BPM (the `240_000_000 / bpm` invariant).
pub const US_PER_MEASURE_NUM: f64 = 240_000_000.0;

pub fn measure_us(bpm: f64) -> f64 {
    US_PER_MEASURE_NUM / bpm
}

/// Default `#TOTAL` for a chart with `notes` playable notes when the header is missing or
/// non-positive — reference implementation `BMSPlayerRule.calculateDefaultTotal` (`BMSPlayerRule.java:84-89`) for
/// the BEAT and POPN modes. Single source for the gauge, the note-density report and the app.
pub fn default_total(notes: usize) -> f64 {
    let n = notes as f64;
    (7.605 * n / (0.01 * n + 6.5)).max(260.0)
}

/// `calculateDefaultTotal` for the 24-key KEYBOARD modes: a higher floor and a `notes + 100`
/// numerator. Kept alongside [`default_total`] so wiring that mode in later needs no new formula.
pub fn default_total_keyboard(notes: usize) -> f64 {
    let n = notes as f64;
    (7.605 * (n + 100.0) / (0.01 * n + 6.5)).max(300.0)
}

/// `#VOLWAV` value assumed when the header is absent or does not parse as an integer. It is the
/// percentage that maps to unity gain, so an unspecified chart plays at its authored level.
pub const VOLWAV_DEFAULT_PERCENT: i32 = 100;

const VOLWAV_MIN_PERCENT_EXCLUSIVE: i32 = 0;
const VOLWAV_MAX_PERCENT_EXCLUSIVE: i32 = 200;
const VOLWAV_PERCENT_PER_UNIT_GAIN: f32 = 100.0;
const VOLWAV_FALLBACK_GAIN: f32 = 1.0;

/// Chart-wide linear playback gain derived from `#VOLWAV`, applied once per chart on top of the
/// per-bus gains. Only values strictly inside `(0, 200)` scale the output; everything else -
/// including `0`, `200` and negatives - plays at unity, matching the reference implementation
/// (`AbstractAudioDriver.java:355-358`).
pub fn chart_gain(volwav_percent: i32) -> f32 {
    if volwav_percent > VOLWAV_MIN_PERCENT_EXCLUSIVE && volwav_percent < VOLWAV_MAX_PERCENT_EXCLUSIVE {
        volwav_percent as f32 / VOLWAV_PERCENT_PER_UNIT_GAIN
    } else {
        VOLWAV_FALLBACK_GAIN
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LnKind {
    Ln,
    Cn,
    Hcn,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NoteKind {
    Normal,
    LongStart { ln: LnKind },
    LongEnd { ln: LnKind },
    Mine { damage: f64 },
}

#[derive(Debug, Clone)]
pub struct Note {
    pub kind: NoteKind,
    pub wav: i32,
    pub start_us: Micros,
    pub duration_us: Micros,
    pub time_us: Micros,
    pub section: f64,
    pub layered: Vec<Note>,
}

impl Note {
    pub fn normal(wav: i32, time_us: Micros, section: f64) -> Self {
        Note { kind: NoteKind::Normal, wav, start_us: 0, duration_us: 0, time_us, section, layered: Vec::new() }
    }
}

#[derive(Debug, Clone)]
pub struct TimeLine {
    pub time_us: Micros,
    pub section: f64,
    pub notes: Vec<Option<Note>>,
    pub hidden: Vec<Option<Note>>,
    pub bgnotes: Vec<Note>,
    pub section_line: bool,
    pub bpm: f64,
    pub stop_us: Micros,
    pub scroll: f64,
    pub bga: i32,
    pub layer: i32,
}

impl TimeLine {
    pub fn empty(lanes: usize, time_us: Micros, section: f64, bpm: f64) -> Self {
        TimeLine {
            time_us,
            section,
            notes: vec![None; lanes],
            hidden: vec![None; lanes],
            bgnotes: Vec::new(),
            section_line: false,
            bpm,
            stop_us: 0,
            scroll: 1.0,
            bga: -1,
            layer: -1,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ModelMeta {
    pub title: String,
    pub subtitle: String,
    pub artist: String,
    pub subartist: String,
    pub genre: String,
    pub play_level: String,
    pub difficulty: i32,
    pub rank: i32,
    /// `#DEFEXRANK`: when positive it replaces the `#RANK` table and scales the NORMAL judgerank
    /// (reference implementation `BMSPlayerRule.java:63`).
    pub defexrank: Option<f64>,
    pub total: f64,
    /// `#VOLWAV` as authored, in percent. Convert with [`chart_gain`] before use; `Default` yields
    /// `0`, which that conversion maps to unity gain.
    pub volwav: i32,
    pub stagefile: String,
}

#[derive(Debug, Clone)]
pub struct Model {
    pub mode: Mode,
    pub meta: ModelMeta,
    pub wavmap: Vec<String>,
    pub bgamap: Vec<String>,
    pub init_bpm: f64,
    pub timelines: Vec<TimeLine>,
    pub md5: String,
    pub sha256: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- measure_us / US_PER_MEASURE_NUM invariant ----

    #[test]
    fn us_per_measure_num_is_240_million() {
        assert_eq!(US_PER_MEASURE_NUM, 240_000_000.0);
    }

    #[test]
    fn measure_us_matches_invariant_formula() {
        for &bpm in &[60.0, 120.0, 130.0, 174.0, 200.0, 0.5, 999.0] {
            assert_eq!(measure_us(bpm), US_PER_MEASURE_NUM / bpm, "bpm={bpm}");
        }
    }

    #[test]
    fn measure_us_known_values() {
        // 240_000_000 / bpm
        assert_eq!(measure_us(120.0), 2_000_000.0);
        assert_eq!(measure_us(60.0), 4_000_000.0);
        assert_eq!(measure_us(240.0), 1_000_000.0);
        assert_eq!(measure_us(1.0), 240_000_000.0);
    }

    #[test]
    fn measure_us_is_monotonically_decreasing_in_bpm() {
        // Higher BPM => shorter measure.
        let bpms = [30.0, 60.0, 90.0, 120.0, 180.0, 240.0, 480.0];
        for w in bpms.windows(2) {
            assert!(measure_us(w[0]) > measure_us(w[1]), "{} vs {}", w[0], w[1]);
        }
    }

    #[test]
    fn measure_us_inverse_proportionality() {
        // Doubling BPM halves the measure length.
        assert_eq!(measure_us(120.0), measure_us(240.0) * 2.0);
        assert_eq!(measure_us(100.0) / 2.0, measure_us(200.0));
    }

    #[test]
    fn measure_us_zero_bpm_is_infinite() {
        // NOTE: suspect - division by zero yields +inf rather than an error/guard.
        assert!(measure_us(0.0).is_infinite());
        assert!(measure_us(0.0).is_sign_positive());
    }

    #[test]
    fn measure_us_negative_bpm_is_negative() {
        // NOTE: suspect - negative BPM produces a negative measure length, not guarded.
        assert!(measure_us(-120.0) < 0.0);
        assert_eq!(measure_us(-120.0), -2_000_000.0);
    }

    #[test]
    fn measure_us_is_deterministic() {
        assert_eq!(measure_us(174.0), measure_us(174.0));
    }

    // ---- TimeLine::empty defaults ----

    #[test]
    fn timeline_empty_default_fields() {
        let tl = TimeLine::empty(8, 1_234, 0.5, 130.0);
        assert_eq!(tl.time_us, 1_234);
        assert_eq!(tl.section, 0.5);
        assert_eq!(tl.bpm, 130.0);
        assert_eq!(tl.stop_us, 0);
        assert_eq!(tl.scroll, 1.0);
        assert_eq!(tl.bga, -1);
        assert_eq!(tl.layer, -1);
        assert!(!tl.section_line);
        assert!(tl.bgnotes.is_empty());
    }

    #[test]
    fn timeline_empty_allocates_lanes_with_none() {
        let tl = TimeLine::empty(8, 0, 0.0, 120.0);
        assert_eq!(tl.notes.len(), 8);
        assert_eq!(tl.hidden.len(), 8);
        assert!(tl.notes.iter().all(|n| n.is_none()));
        assert!(tl.hidden.iter().all(|n| n.is_none()));
    }

    #[test]
    fn timeline_empty_zero_lanes() {
        let tl = TimeLine::empty(0, 0, 0.0, 120.0);
        assert!(tl.notes.is_empty());
        assert!(tl.hidden.is_empty());
    }

    #[test]
    fn timeline_empty_lane_count_matches_argument() {
        for lanes in [0usize, 1, 6, 8, 9, 12, 16] {
            let tl = TimeLine::empty(lanes, 0, 0.0, 120.0);
            assert_eq!(tl.notes.len(), lanes, "lanes={lanes}");
            assert_eq!(tl.hidden.len(), lanes, "lanes={lanes}");
        }
    }

    #[test]
    fn timeline_empty_notes_and_hidden_are_independent() {
        let mut tl = TimeLine::empty(2, 0, 0.0, 120.0);
        tl.notes[0] = Some(Note::normal(1, 0, 0.0));
        // Mutating notes must not affect hidden.
        assert!(tl.notes[0].is_some());
        assert!(tl.hidden[0].is_none());
    }

    #[test]
    fn timeline_empty_preserves_negative_time() {
        let tl = TimeLine::empty(4, -500, -1.0, 120.0);
        assert_eq!(tl.time_us, -500);
        assert_eq!(tl.section, -1.0);
    }

    // ---- Note::normal zeroing ----

    #[test]
    fn note_normal_zeroes_long_note_fields() {
        let n = Note::normal(42, 9_999, 1.25);
        assert_eq!(n.kind, NoteKind::Normal);
        assert_eq!(n.wav, 42);
        assert_eq!(n.time_us, 9_999);
        assert_eq!(n.section, 1.25);
        assert_eq!(n.start_us, 0);
        assert_eq!(n.duration_us, 0);
        assert!(n.layered.is_empty());
    }

    #[test]
    fn note_normal_preserves_negative_wav_and_time() {
        let n = Note::normal(-1, -42, -0.5);
        assert_eq!(n.wav, -1);
        assert_eq!(n.time_us, -42);
        assert_eq!(n.section, -0.5);
        assert_eq!(n.start_us, 0);
        assert_eq!(n.duration_us, 0);
    }

    #[test]
    fn note_normal_kind_is_not_long_or_mine() {
        let n = Note::normal(0, 0, 0.0);
        assert!(matches!(n.kind, NoteKind::Normal));
        assert!(!matches!(n.kind, NoteKind::LongStart { .. }));
        assert!(!matches!(n.kind, NoteKind::LongEnd { .. }));
        assert!(!matches!(n.kind, NoteKind::Mine { .. }));
    }

    #[test]
    fn note_normal_is_deterministic() {
        let a = Note::normal(7, 100, 2.0);
        let b = Note::normal(7, 100, 2.0);
        assert_eq!(a.wav, b.wav);
        assert_eq!(a.time_us, b.time_us);
        assert_eq!(a.section, b.section);
        assert_eq!(a.start_us, b.start_us);
        assert_eq!(a.duration_us, b.duration_us);
        assert_eq!(a.kind, b.kind);
    }

    // ---- enum value equality ----

    #[test]
    fn long_kind_distinguishes_variants() {
        assert_eq!(LnKind::Ln, LnKind::Ln);
        assert_ne!(LnKind::Ln, LnKind::Cn);
        assert_ne!(LnKind::Cn, LnKind::Hcn);
    }

    #[test]
    fn note_kind_long_start_carries_ln_kind() {
        let s = NoteKind::LongStart { ln: LnKind::Hcn };
        assert_eq!(s, NoteKind::LongStart { ln: LnKind::Hcn });
        assert_ne!(s, NoteKind::LongStart { ln: LnKind::Ln });
        assert_ne!(s, NoteKind::LongEnd { ln: LnKind::Hcn });
    }

    #[test]
    fn note_kind_mine_carries_damage() {
        assert_eq!(NoteKind::Mine { damage: 1.0 }, NoteKind::Mine { damage: 1.0 });
        assert_ne!(NoteKind::Mine { damage: 1.0 }, NoteKind::Mine { damage: 2.0 });
    }

    // ---- ModelMeta default ----

    #[test]
    fn model_meta_default_is_empty_and_zeroed() {
        let m = ModelMeta::default();
        assert!(m.title.is_empty());
        assert!(m.subtitle.is_empty());
        assert!(m.artist.is_empty());
        assert!(m.subartist.is_empty());
        assert!(m.genre.is_empty());
        assert!(m.play_level.is_empty());
        assert_eq!(m.difficulty, 0);
        assert_eq!(m.rank, 0);
        assert_eq!(m.total, 0.0);
        assert_eq!(m.volwav, 0);
        assert!(m.stagefile.is_empty());
    }

    #[test]
    fn chart_gain_default_percent_is_unity() {
        assert_eq!(chart_gain(VOLWAV_DEFAULT_PERCENT), 1.0);
    }

    #[test]
    fn chart_gain_default_model_meta_is_unity() {
        assert_eq!(chart_gain(ModelMeta::default().volwav), 1.0);
    }

    #[test]
    fn chart_gain_scales_inside_the_open_range() {
        assert_eq!(chart_gain(1), 0.01);
        assert_eq!(chart_gain(50), 0.5);
        assert_eq!(chart_gain(99), 0.99);
        assert_eq!(chart_gain(150), 1.5);
        assert_eq!(chart_gain(199), 1.99);
    }

    #[test]
    fn chart_gain_excludes_both_bounds() {
        assert_eq!(chart_gain(0), 1.0);
        assert_eq!(chart_gain(200), 1.0);
    }

    #[test]
    fn chart_gain_out_of_range_values_are_unity() {
        assert_eq!(chart_gain(201), 1.0);
        assert_eq!(chart_gain(-5), 1.0);
        assert_eq!(chart_gain(i32::MIN), 1.0);
        assert_eq!(chart_gain(i32::MAX), 1.0);
    }

    #[test]
    fn chart_gain_is_never_negative_or_above_two() {
        for v in -300..=500 {
            let g = chart_gain(v);
            assert!(g > 0.0, "volwav {v} produced a non-positive gain");
            assert!(g < 2.0, "volwav {v} produced a gain at or above 2.0");
        }
    }

    #[test]
    fn chart_gain_is_monotonic_inside_the_open_range() {
        let mut prev = chart_gain(1);
        for v in 2..VOLWAV_MAX_PERCENT_EXCLUSIVE {
            let g = chart_gain(v);
            assert!(g > prev, "volwav {v} did not increase the gain");
            prev = g;
        }
    }

    // ---- Note clone round-trip ----

    #[test]
    fn note_clone_round_trip_preserves_fields() {
        let mut n = Note::normal(5, 123, 0.75);
        n.layered.push(Note::normal(6, 124, 0.76));
        let c = n.clone();
        assert_eq!(c.wav, n.wav);
        assert_eq!(c.time_us, n.time_us);
        assert_eq!(c.section, n.section);
        assert_eq!(c.layered.len(), 1);
        assert_eq!(c.layered[0].wav, 6);
    }
}
