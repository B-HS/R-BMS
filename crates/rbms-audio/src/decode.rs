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

    let track = format.tracks().iter().find(|t| t.codec_params.codec != CODEC_TYPE_NULL).ok_or_else(|| AudioError::Decode("no audio track".into()))?;
    let track_id = track.id;
    let codec_params = track.codec_params.clone();

    let mut decoder = symphonia::default::get_codecs().make(&codec_params, &DecoderOptions::default()).map_err(|e| AudioError::Decode(e.to_string()))?;

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

    /// Build a 16-bit little-endian PCM WAV with the given channel count and interleaved samples.
    fn wav_16le(rate: u32, channels: u16, samples: &[i16]) -> Vec<u8> {
        let data_size = (samples.len() * 2) as u32;
        let byte_rate = rate * channels as u32 * 2;
        let block_align = channels * 2;
        let mut v = Vec::new();
        v.extend_from_slice(b"RIFF");
        v.extend_from_slice(&(36 + data_size).to_le_bytes());
        v.extend_from_slice(b"WAVE");
        v.extend_from_slice(b"fmt ");
        v.extend_from_slice(&16u32.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&channels.to_le_bytes());
        v.extend_from_slice(&rate.to_le_bytes());
        v.extend_from_slice(&byte_rate.to_le_bytes());
        v.extend_from_slice(&block_align.to_le_bytes());
        v.extend_from_slice(&16u16.to_le_bytes());
        v.extend_from_slice(b"data");
        v.extend_from_slice(&data_size.to_le_bytes());
        for s in samples {
            v.extend_from_slice(&s.to_le_bytes());
        }
        v
    }

    #[test]
    fn empty_bytes_returns_error() {
        let err = decode_bytes(Vec::new(), Some("wav"));
        assert!(err.is_err());
    }

    #[test]
    fn garbage_bytes_returns_error() {
        let err = decode_bytes(vec![0u8; 64], Some("wav"));
        assert!(err.is_err());
    }

    #[test]
    fn truncated_header_returns_error() {
        // Only the RIFF magic, nothing else.
        let err = decode_bytes(b"RIFF".to_vec(), Some("wav"));
        assert!(err.is_err());
    }

    #[test]
    fn decodes_stereo_wav_channel_count_and_sample_count() {
        // 240 frames * 2 channels = 480 interleaved samples
        let frames = 240usize;
        let interleaved: Vec<i16> = (0..frames * 2).map(|i| ((i as f32 * 0.05).sin() * 8000.0) as i16).collect();
        let wav = wav_16le(48000, 2, &interleaved);
        let dec = decode_bytes(wav, Some("wav")).unwrap();
        assert_eq!(dec.channels, 2);
        assert_eq!(dec.rate, 48000);
        assert_eq!(dec.samples.len(), frames * 2);
    }

    #[test]
    fn decodes_preserves_rate_field() {
        let samples: Vec<i16> = (0..100).map(|i| (i * 100) as i16).collect();
        let wav = wav_16le(22050, 1, &samples);
        let dec = decode_bytes(wav, Some("wav")).unwrap();
        assert_eq!(dec.rate, 22050);
    }

    #[test]
    fn decoded_samples_are_normalized_to_unit_range() {
        // i16 max 32767 -> ~1.0; all decoded f32 should stay within [-1, 1].
        let samples: Vec<i16> = vec![i16::MAX, i16::MIN, 0, i16::MAX / 2];
        let wav = wav_16le(44100, 1, &samples);
        let dec = decode_bytes(wav, Some("wav")).unwrap();
        assert!(dec.samples.iter().all(|&s| (-1.0..=1.0).contains(&s)), "{:?}", dec.samples);
        // first decoded sample (i16::MAX) close to +1.0
        assert!(dec.samples[0] > 0.99);
    }

    #[test]
    fn decode_round_trips_silence_to_zeros() {
        let samples: Vec<i16> = vec![0i16; 200];
        let wav = wav_16le(44100, 1, &samples);
        let dec = decode_bytes(wav, Some("wav")).unwrap();
        assert_eq!(dec.samples.len(), 200);
        assert!(dec.samples.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn decode_is_deterministic() {
        let samples: Vec<i16> = (0..256).map(|i| ((i as f32 * 0.2).sin() * 9000.0) as i16).collect();
        let wav = wav_16le(44100, 1, &samples);
        let a = decode_bytes(wav.clone(), Some("wav")).unwrap();
        let b = decode_bytes(wav, Some("wav")).unwrap();
        assert_eq!(a.channels, b.channels);
        assert_eq!(a.rate, b.rate);
        assert_eq!(a.samples, b.samples);
    }

    #[test]
    fn decode_without_extension_hint_still_works() {
        // symphonia should probe the container even without an extension hint.
        let samples: Vec<i16> = (0..120).map(|i| (i * 50) as i16).collect();
        let wav = wav_16le_mono(44100, &samples);
        let dec = decode_bytes(wav, None).unwrap();
        assert_eq!(dec.channels, 1);
        assert_eq!(dec.samples.len(), 120);
    }

    #[test]
    fn zero_length_data_chunk_returns_empty_error() {
        // valid header but no PCM frames -> "empty audio" error
        let wav = wav_16le_mono(44100, &[]);
        let err = decode_bytes(wav, Some("wav"));
        assert!(err.is_err());
    }

    #[test]
    fn preview_demo_fixture_decodes_and_mixes_to_nonzero_audio() {
        use crate::mixer::{Bus, Command, Mixer, SampleData};
        use std::sync::Arc;
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../samples/preview-demo/preview.wav");
        let bytes = std::fs::read(path).expect("preview-demo fixture is committed");
        let dec = decode_bytes(bytes, Some("wav")).expect("fixture decodes");
        assert_eq!(dec.channels, 1);
        assert_eq!(dec.rate, 8000);
        assert!(!dec.samples.is_empty());
        let sample = Arc::new(SampleData { pcm: dec.samples.into(), channels: dec.channels, rate: dec.rate });
        let mut mixer = Mixer::new(48000, 2, 16);
        mixer.apply(Command::Play { sample, gain: 0.85, pan: 0.0, pitch: 1.0, key: 0, at_frame: 0, bus: Bus::Bg });
        let mut out = vec![0.0f32; 4096];
        mixer.mix(&mut out);
        assert!(out.iter().any(|&s| s != 0.0), "the #PREVIEW fixture must produce audible PCM through the decode->mix path");
    }
}
