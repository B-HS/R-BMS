use std::fmt;

mod decode;
mod engine;
mod mixer;

pub use decode::{DecodedAudio, decode_bytes};
pub use engine::AudioEngine;
pub use mixer::{Command, Mixer, SampleData};

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
