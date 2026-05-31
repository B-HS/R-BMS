pub mod mode;

pub use mode::Mode;

/// Absolute time, microseconds. The global time unit across rbms.
pub type Micros = i64;

/// Microseconds per 4/4 measure at the given BPM (the `240_000_000 / bpm` invariant).
pub const US_PER_MEASURE_NUM: f64 = 240_000_000.0;

pub fn measure_us(bpm: f64) -> f64 {
    US_PER_MEASURE_NUM / bpm
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
    pub total: f64,
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
