use std::io::Cursor;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{CODEC_TYPE_NULL, DecoderOptions};
use symphonia::core::errors::Error as SymError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::AudioError;

pub struct DecodedAudio {
    pub samples: Vec<f32>,
    pub channels: u16,
    pub rate: u32,
}

/// Decode a WAV/OGG/FLAC/MP3 byte buffer to interleaved f32 at its native rate.
pub fn decode_bytes(bytes: Vec<u8>, ext: Option<&str>) -> Result<DecodedAudio, AudioError> {
    let mss = MediaSourceStream::new(Box::new(Cursor::new(bytes)), MediaSourceStreamOptions::default());
    let mut hint = Hint::new();
    if let Some(e) = ext {
        hint.with_extension(e);
    }
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| AudioError::Decode(e.to_string()))?;
    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| AudioError::Decode("no audio track".into()))?;
    let track_id = track.id;
    let codec_params = track.codec_params.clone();

    let mut decoder = symphonia::default::get_codecs()
        .make(&codec_params, &DecoderOptions::default())
        .map_err(|e| AudioError::Decode(e.to_string()))?;

    let mut samples: Vec<f32> = Vec::new();
    let mut channels = 0u16;
    let mut rate = 0u32;

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(SymError::IoError(ref e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(SymError::ResetRequired) => break,
            Err(e) => return Err(AudioError::Decode(e.to_string())),
        };
        if packet.track_id() != track_id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(decoded) => {
                let spec = *decoded.spec();
                rate = spec.rate;
                channels = spec.channels.count() as u16;
                let cap = decoded.capacity() as u64;
                if cap > 0 {
                    let mut sb = SampleBuffer::<f32>::new(cap, spec);
                    sb.copy_interleaved_ref(decoded);
                    samples.extend_from_slice(sb.samples());
                }
            }
            Err(SymError::DecodeError(_)) => continue,
            Err(SymError::IoError(ref e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(AudioError::Decode(e.to_string())),
        }
    }

    if channels == 0 || samples.is_empty() {
        return Err(AudioError::Decode("empty audio".into()));
    }
    Ok(DecodedAudio { samples, channels, rate })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav_16le_mono(rate: u32, samples: &[i16]) -> Vec<u8> {
        let data_size = (samples.len() * 2) as u32;
        let mut v = Vec::new();
        v.extend_from_slice(b"RIFF");
        v.extend_from_slice(&(36 + data_size).to_le_bytes());
        v.extend_from_slice(b"WAVE");
        v.extend_from_slice(b"fmt ");
        v.extend_from_slice(&16u32.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&rate.to_le_bytes());
        v.extend_from_slice(&(rate * 2).to_le_bytes());
        v.extend_from_slice(&2u16.to_le_bytes());
        v.extend_from_slice(&16u16.to_le_bytes());
        v.extend_from_slice(b"data");
        v.extend_from_slice(&data_size.to_le_bytes());
        for s in samples {
            v.extend_from_slice(&s.to_le_bytes());
        }
        v
    }

    #[test]
    fn decodes_in_memory_wav() {
        let samples: Vec<i16> = (0..480).map(|i| ((i as f32 * 0.1).sin() * 10000.0) as i16).collect();
        let wav = wav_16le_mono(44100, &samples);
        let dec = decode_bytes(wav, Some("wav")).unwrap();
        assert_eq!(dec.channels, 1);
        assert_eq!(dec.rate, 44100);
        assert_eq!(dec.samples.len(), 480);
        assert!(dec.samples.iter().any(|&s| s.abs() > 0.01));
    }
}
