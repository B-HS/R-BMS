#![forbid(unsafe_code)]

use std::fmt;

mod decode;
mod engine;
mod mixer;

/// Re-exported so callers can name the stream types that appear in [`AudioOpenReport`] without
/// depending on cpal themselves.
pub use cpal;
pub use decode::{DecodedAudio, decode_bytes};
pub use engine::{AudioClocks, AudioEngine, AudioOpenReport, AudioOptions, ClockSnapshot, DEFAULT_MAX_VOICES, IdNamespace, monotonic_us};
pub use mixer::{Bus, Command, MixStats, Mixer, SampleData};
/// Re-exported so callers can build the retirement ring [`Mixer::set_retire`] expects without
/// pinning the same rtrb version themselves.
pub use rtrb;

#[derive(Debug)]
pub enum AudioError {
    NoDevice,
    Decode(String),
    Stream(String),
    Unsupported(String),
}

impl fmt::Display for AudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AudioError::NoDevice => write!(f, "no default audio output device"),
            AudioError::Decode(s) => write!(f, "decode error: {s}"),
            AudioError::Stream(s) => write!(f, "audio stream error: {s}"),
            AudioError::Unsupported(s) => write!(f, "unsupported: {s}"),
        }
    }
}

impl std::error::Error for AudioError {}
