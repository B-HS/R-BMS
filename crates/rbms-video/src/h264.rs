//! H.264 in an MP4 container (`.mp4`, `.m4v`).
//!
//! `re_mp4` reads the sample table, which gives the video track's size, the parameter sets in its
//! `avcC` configuration and, per sample, the byte range and the composition time. The decoder wants
//! Annex B, so each sample's length-prefixed units are rewritten with start codes and the parameter
//! sets are prepended to every sync sample, which is what lets a stream whose configuration
//! changes mid-file keep decoding.
//!
//! Pictures come out one access unit at a time, in decode order, stamped with that sample's own
//! composition time. For the baseline streams a chart ships the two orders are the same; a stream
//! with B-pictures would hand a picture on slightly out of order rather than reordering it here,
//! because reordering means holding a whole group of pictures in memory.

use crate::yuv::{PlaneGeometry, yuv420_to_rgba};
use crate::{DecodedFrame, FrameSink, MICROS_PER_SECOND, VideoError, VideoGeometry};
use re_mp4::{Mp4, StsdBoxContent, Track, TrackKind};
use std::sync::Arc;

/// The four bytes that introduce a NAL unit in an Annex B stream.
const START_CODE: [u8; 4] = [0x00, 0x00, 0x00, 0x01];

/// Mask of `lengthSizeMinusOne` within its `avcC` byte (ISO/IEC 14496-15 §5.2.4.1).
const LENGTH_SIZE_MASK: u8 = 0x03;

/// How many luma samples one chroma sample covers along each axis in 4:2:0.
const CHROMA_SUBSAMPLING: usize = 2;

/// The video track of an MP4, with the `avcC` configuration that decodes its samples.
struct VideoTrack<'a> {
    track: &'a Track,
    /// Bytes of the length prefix in front of each unit of a sample.
    length_size: usize,
    /// The parameter sets, already in Annex B form, prepended to every sync sample.
    parameter_sets: Vec<u8>,
}

/// Find the H.264 video track and read its configuration.
fn video_track<'a>(mp4: &'a Mp4) -> Result<VideoTrack<'a>, VideoError> {
    let track = mp4
        .tracks()
        .values()
        .find(|track| track.kind == Some(TrackKind::Video))
        .ok_or_else(|| VideoError::Unsupported("an MP4 with no video track".to_owned()))?;
    let StsdBoxContent::Avc1(avc1) = &track.trak(mp4).mdia.minf.stbl.stsd.contents else {
        return Err(VideoError::Unsupported("an MP4 whose video is not H.264".to_owned()));
    };
    let mut parameter_sets = Vec::new();
    for unit in avc1.avcc.sequence_parameter_sets.iter().chain(avc1.avcc.picture_parameter_sets.iter()) {
        parameter_sets.extend_from_slice(&START_CODE);
        parameter_sets.extend_from_slice(&unit.bytes);
    }
    if parameter_sets.is_empty() {
        return Err(VideoError::Decode("an H.264 track with no parameter sets".to_owned()));
    }
    let length_size = usize::from(avc1.avcc.length_size_minus_one & LENGTH_SIZE_MASK) + 1;
    Ok(VideoTrack { track, length_size, parameter_sets })
}

/// Rewrite one sample's length-prefixed NAL units as Annex B, into `annex_b`.
///
/// Returns `false` when a prefix claims more bytes than the sample holds, which is the one way a
/// damaged sample table could walk off the end of the file.
fn append_annex_b(sample: &[u8], length_size: usize, annex_b: &mut Vec<u8>) -> bool {
    let mut at = 0usize;
    while at < sample.len() {
        let Some(prefix) = sample.get(at..at + length_size) else {
            return false;
        };
        let length = prefix.iter().fold(0usize, |length, byte| length << u8::BITS | usize::from(*byte));
        at += length_size;
        let Some(unit) = sample.get(at..at + length) else {
            return false;
        };
        annex_b.extend_from_slice(&START_CODE);
        annex_b.extend_from_slice(unit);
        at += length;
    }
    true
}

/// Turn a time in the track's own time base into microseconds of stream time.
fn micros(units: i64, timescale: u64) -> i64 {
    match timescale {
        0 => 0,
        timescale => units.saturating_mul(MICROS_PER_SECOND) / timescale as i64,
    }
}

/// Read the video track's size without decoding a picture.
pub(crate) fn probe(bytes: &[u8]) -> Result<VideoGeometry, VideoError> {
    let mp4 = Mp4::read_bytes(bytes).map_err(|err| VideoError::Decode(format!("mp4: {err}")))?;
    let video = video_track(&mp4)?;
    if video.track.width == 0 || video.track.height == 0 {
        return Err(VideoError::Decode("an H.264 track with no picture size".to_owned()));
    }
    Ok(VideoGeometry { width: u32::from(video.track.width), height: u32::from(video.track.height) })
}

/// Decode every sample of the video track and hand each picture on.
pub(crate) fn decode(bytes: &[u8], _geometry: VideoGeometry, sink: &mut FrameSink<'_>) -> Result<(), VideoError> {
    let mp4 = Mp4::read_bytes(bytes).map_err(|err| VideoError::Decode(format!("mp4: {err}")))?;
    let video = video_track(&mp4)?;
    let mut decoder = rusty_h264_decoder::Decoder::default();
    let mut annex_b = Vec::new();
    for sample in &video.track.samples {
        let Some(payload) = bytes.get(sample.byte_range()) else {
            return Err(VideoError::Decode("a sample outside the file".to_owned()));
        };
        annex_b.clear();
        if sample.is_sync {
            annex_b.extend_from_slice(&video.parameter_sets);
        }
        if !append_annex_b(payload, video.length_size, &mut annex_b) {
            return Err(VideoError::Decode("a sample whose unit lengths do not fit it".to_owned()));
        }
        let picture = decoder.decode(&annex_b).map_err(|err| VideoError::Decode(format!("h264: {err:?}")))?;
        let Some(picture) = picture else {
            continue;
        };
        let geometry = PlaneGeometry {
            width: picture.width,
            height: picture.height,
            chroma_width: picture.width.div_ceil(CHROMA_SUBSAMPLING),
            chroma_height: picture.height.div_ceil(CHROMA_SUBSAMPLING),
        };
        let Some(rgba) = yuv420_to_rgba(geometry, &picture.y, &picture.u, &picture.v) else {
            return Err(VideoError::Decode("a picture whose planes do not match its size".to_owned()));
        };
        let start_us = micros(sample.composition_timestamp, sample.timescale);
        let decoded = DecodedFrame {
            start_us,
            end_us: micros(sample.composition_timestamp.saturating_add(sample.duration as i64), sample.timescale),
            rgba: Arc::from(rgba),
            width: picture.width as u32,
            height: picture.height as u32,
        };
        if !sink(decoded) {
            return Ok(());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_prefixed_units_become_start_code_units() {
        let sample = [0, 0, 0, 2, 0x65, 0x11, 0, 0, 0, 1, 0x41];
        let mut annex_b = Vec::new();
        assert!(append_annex_b(&sample, 4, &mut annex_b));
        assert_eq!(annex_b, vec![0, 0, 0, 1, 0x65, 0x11, 0, 0, 0, 1, 0x41]);
    }

    #[test]
    fn a_prefix_longer_than_the_sample_is_refused() {
        let mut annex_b = Vec::new();
        assert!(!append_annex_b(&[0, 0, 0, 9, 0x65], 4, &mut annex_b));
        assert!(!append_annex_b(&[0, 0], 4, &mut annex_b));
    }

    #[test]
    fn a_time_base_turns_into_microseconds() {
        assert_eq!(micros(600, 12_800), 46_875);
        assert_eq!(micros(0, 12_800), 0);
        assert_eq!(micros(5, 0), 0);
    }
}
