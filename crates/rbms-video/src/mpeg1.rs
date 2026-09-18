//! MPEG-1 / MPEG-2 video, in a program stream (`.mpg`, `.mpeg`) or on its own (`.m1v`, `.m2v`).
//!
//! A program stream is unwrapped first: `mpeg-ps` walks the packs and `mpeg-pes` hands back the
//! payload of each packet, and the payloads of the video stream are concatenated back into the
//! elementary stream the video decoder wants. An elementary stream skips that step.
//!
//! Those two crates read the ISO/IEC 13818-1 pack and packet headers. A `.mpg` holding MPEG-1
//! video is usually an ISO/IEC 11172-1 system stream instead, whose pack header and packet header
//! are laid out differently, and they refuse it. So a stream they will not take is walked here
//! instead, by the packet start codes, which reads both generations. Only the payload bytes matter
//! either way: the clock comes from the sequence header's frame rate, not from the presentation
//! stamps in the packet headers.
//!
//! The decoder has one entry point and it reconstructs the whole sequence in a single call, so the
//! sequence's pictures are all in memory before the first one is handed on. Timestamps come from
//! the sequence header's frame rate and the display index rather than from the program stream's
//! own presentation stamps, which is the same frame-0-plus-offset clock the rest of the crate uses.

use crate::yuv::{PlaneGeometry, yuv420_to_rgba};
use crate::{DecodedFrame, FrameSink, MICROS_PER_SECOND, VideoError, VideoGeometry};
use mpeg_pes::StreamId;
use oxideav_mpeg12video::decode_video_sequence;
use std::sync::Arc;

/// `pack_start_code`, the four bytes a program stream begins with (ISO/IEC 13818-1 §2.5.3.3).
const PACK_START_CODE: [u8; 4] = [0x00, 0x00, 0x01, 0xBA];

/// `sequence_header_code`, the four bytes a video elementary stream's sequence layer begins with
/// (ISO/IEC 13818-2 §6.2.2.1).
const SEQUENCE_HEADER_CODE: [u8; 4] = [0x00, 0x00, 0x01, 0xB3];

/// First `stream_id` of the sixteen video streams a program stream may carry (§2.4.3.7 Table 2-18).
const VIDEO_STREAM_ID_FIRST: u8 = 0xE0;

/// Last `stream_id` of those sixteen.
const VIDEO_STREAM_ID_LAST: u8 = 0xEF;

/// Bytes of `sequence_header()` that must follow the start code before `frame_rate_code` is
/// readable: three bytes of `horizontal_size` and `vertical_size` and the byte holding
/// `aspect_ratio_information` and `frame_rate_code`.
const SEQUENCE_HEADER_MIN_BYTES: usize = 8;

/// Offset from the start code to the first of the three size bytes.
const SEQUENCE_SIZE_OFFSET: usize = 4;

/// Offset from the start code to the byte holding `aspect_ratio_information` and `frame_rate_code`.
const SEQUENCE_RATE_OFFSET: usize = 7;

/// Bits each of `horizontal_size` and `vertical_size` occupies (§6.2.2.1).
const SIZE_BITS: u32 = 12;

/// Mask of the low four bits, which is `frame_rate_code` within its byte.
const LOW_NIBBLE: u8 = 0x0F;

/// Table 6-4 `frame_rate_value` as `(numerator, denominator)`, indexed by `frame_rate_code`.
/// Code 0 is forbidden and codes 9..15 are reserved; both are read as 25 fps, the rate every other
/// entry of the table is a neighbour of, so a stream with a damaged header still plays at a sane
/// speed instead of not at all.
const FRAME_RATES: [(i64, i64); 16] = [
    (25, 1),
    (24000, 1001),
    (24, 1),
    (25, 1),
    (30000, 1001),
    (30, 1),
    (50, 1),
    (60000, 1001),
    (60, 1),
    (25, 1),
    (25, 1),
    (25, 1),
    (25, 1),
    (25, 1),
    (25, 1),
    (25, 1),
];

/// `MPEG_program_end_code`, which ends a program stream.
const PROGRAM_END_STREAM_ID: u8 = 0xB9;

/// `pack_start_code`'s `stream_id` byte, which introduces a pack rather than a packet.
const PACK_STREAM_ID: u8 = 0xBA;

/// The three bytes every start code begins with.
const PACKET_START_PREFIX: [u8; 3] = [0x00, 0x00, 0x01];

/// Bytes of a start code, counting the `stream_id` byte.
const START_CODE_LEN: usize = 4;

/// Bytes of a packet's header before its own payload or optional header: the start code and the
/// 16-bit `PES_packet_length`.
const PACKET_PREFIX_LEN: usize = 6;

/// Bytes of an ISO/IEC 11172-1 pack header, start code included.
const MPEG1_PACK_LEN: usize = 12;

/// Bytes of an ISO/IEC 13818-1 pack header before its stuffing, start code included.
const MPEG2_PACK_LEN: usize = 14;

/// Mask of `pack_stuffing_length` in the last byte of a 13818-1 pack header.
const PACK_STUFFING_MASK: u8 = 0x07;

/// Mask of the two bits that tell the two pack-header generations apart.
const PACK_GENERATION_MASK: u8 = 0xC0;

/// Value of those two bits in a 13818-1 pack header (the `01` prefix of its `system_clock_reference`).
const MPEG2_PACK_GENERATION: u8 = 0x40;

/// Mask of the two bits that tell the two packet-header generations apart.
const PACKET_GENERATION_MASK: u8 = 0xC0;

/// Value of those two bits in a 13818-1 packet header (its `10` prefix).
const MPEG2_PACKET_GENERATION: u8 = 0x80;

/// Bytes of a 13818-1 packet's fixed optional header: two flag bytes and `PES_header_data_length`.
const MPEG2_PACKET_HEADER_FIXED: usize = 3;

/// The stuffing byte an 11172-1 packet header may repeat before its own fields.
const MPEG1_STUFFING_BYTE: u8 = 0xFF;

/// Mask of the two bits that mark an 11172-1 packet header's `STD_buffer_scale` field.
const MPEG1_STD_BUFFER_MASK: u8 = 0xC0;

/// Value of those bits when that field is present.
const MPEG1_STD_BUFFER: u8 = 0x40;

/// Bytes that field occupies.
const MPEG1_STD_BUFFER_LEN: usize = 2;

/// Mask of the four bits that mark an 11172-1 packet header's timestamp fields.
const MPEG1_TIMESTAMP_MASK: u8 = 0xF0;

/// Value of those bits when the header carries a presentation time stamp alone.
const MPEG1_PTS_ONLY: u8 = 0x20;

/// Value of those bits when it carries a presentation and a decoding time stamp.
const MPEG1_PTS_AND_DTS: u8 = 0x30;

/// Bytes one such timestamp occupies.
const MPEG1_TIMESTAMP_LEN: usize = 5;

/// The single byte that stands in for the timestamps when a header carries neither.
const MPEG1_NO_TIMESTAMP: u8 = 0x0F;

/// Where a start code sits in `bytes`, if it is there at all.
fn find_start_code(bytes: &[u8], code: [u8; 4]) -> Option<usize> {
    bytes.windows(code.len()).position(|window| window == code)
}

/// Whether `stream_id` names one of the sixteen video streams a program stream may carry.
fn is_video_stream(stream_id: u8) -> bool {
    (VIDEO_STREAM_ID_FIRST..=VIDEO_STREAM_ID_LAST).contains(&stream_id)
}

/// How many bytes of a packet body, taken from just after `PES_packet_length`, are header rather
/// than elementary stream. `None` when the body is too short to hold the header it claims.
fn packet_header_len(body: &[u8]) -> Option<usize> {
    let first = *body.first()?;
    if first & PACKET_GENERATION_MASK == MPEG2_PACKET_GENERATION {
        return Some(MPEG2_PACKET_HEADER_FIXED + usize::from(*body.get(MPEG2_PACKET_HEADER_FIXED - 1)?));
    }
    let mut at = 0usize;
    while body.get(at).copied() == Some(MPEG1_STUFFING_BYTE) {
        at += 1;
    }
    if body.get(at)? & MPEG1_STD_BUFFER_MASK == MPEG1_STD_BUFFER {
        at += MPEG1_STD_BUFFER_LEN;
    }
    match body.get(at)? {
        marker if marker & MPEG1_TIMESTAMP_MASK == MPEG1_PTS_ONLY => Some(at + MPEG1_TIMESTAMP_LEN),
        marker if marker & MPEG1_TIMESTAMP_MASK == MPEG1_PTS_AND_DTS => Some(at + MPEG1_TIMESTAMP_LEN * 2),
        marker if *marker == MPEG1_NO_TIMESTAMP => Some(at + 1),
        _ => None,
    }
}

/// Walk a program stream by its start codes, concatenating the payloads of the video stream.
///
/// This is the path for streams `mpeg-ps` refuses, which is every ISO/IEC 11172-1 system stream.
fn scan_video_payloads(bytes: &[u8]) -> Vec<u8> {
    let mut elementary = Vec::new();
    let mut at = 0usize;
    while let Some(found) = bytes.get(at..).and_then(|rest| rest.windows(PACKET_START_PREFIX.len()).position(|w| w == PACKET_START_PREFIX)) {
        let start = at + found;
        let Some(&stream_id) = bytes.get(start + PACKET_START_PREFIX.len()) else {
            break;
        };
        if stream_id == PROGRAM_END_STREAM_ID {
            break;
        }
        if stream_id == PACK_STREAM_ID {
            let generation = bytes.get(start + START_CODE_LEN).copied().unwrap_or_default();
            at = match generation & PACK_GENERATION_MASK == MPEG2_PACK_GENERATION {
                true => start + MPEG2_PACK_LEN + usize::from(bytes.get(start + MPEG2_PACK_LEN - 1).copied().unwrap_or_default() & PACK_STUFFING_MASK),
                false => start + MPEG1_PACK_LEN,
            };
            continue;
        }
        let Some(length) = bytes.get(start + START_CODE_LEN..start + PACKET_PREFIX_LEN) else {
            break;
        };
        let length = usize::from(u16::from_be_bytes([length[0], length[1]]));
        let body_start = start + PACKET_PREFIX_LEN;
        let body_end = (body_start + length).min(bytes.len());
        let body = &bytes[body_start..body_end];
        if is_video_stream(stream_id)
            && let Some(header) = packet_header_len(body)
            && header <= body.len()
        {
            elementary.extend_from_slice(&body[header..]);
        }
        at = body_end.max(start + PACKET_PREFIX_LEN);
    }
    elementary
}

/// Pull the video elementary stream out of a program stream, or hand back an elementary stream
/// unchanged.
fn elementary_stream(bytes: &[u8]) -> Result<Vec<u8>, VideoError> {
    if !bytes.starts_with(&PACK_START_CODE) {
        return Ok(bytes.to_vec());
    }
    let mut elementary = Vec::new();
    if let Ok((packs, _)) = mpeg_ps::program_stream::parse_all_packs(bytes) {
        for pack in packs {
            for packet in pack.pes_packets {
                let StreamId(id) = packet.stream_id;
                if is_video_stream(id) {
                    elementary.extend_from_slice(packet.payload);
                }
            }
        }
    }
    if elementary.is_empty() {
        elementary = scan_video_payloads(bytes);
    }
    if elementary.is_empty() {
        return Err(VideoError::Unsupported("a program stream with no video stream".to_owned()));
    }
    Ok(elementary)
}

/// The sequence header's geometry and the microseconds one picture is displayed for.
#[derive(Debug)]
struct SequenceHeader {
    geometry: VideoGeometry,
    frame_us: i64,
}

/// Read the leading `sequence_header()` of an elementary stream.
fn sequence_header(elementary: &[u8]) -> Result<SequenceHeader, VideoError> {
    let at = find_start_code(elementary, SEQUENCE_HEADER_CODE).ok_or_else(|| VideoError::Decode("no sequence header".to_owned()))?;
    let header = elementary.get(at..at + SEQUENCE_HEADER_MIN_BYTES).ok_or_else(|| VideoError::Decode("truncated sequence header".to_owned()))?;
    let size = u32::from(header[SEQUENCE_SIZE_OFFSET]) << 16 | u32::from(header[SEQUENCE_SIZE_OFFSET + 1]) << 8 | u32::from(header[SEQUENCE_SIZE_OFFSET + 2]);
    let width = size >> SIZE_BITS;
    let height = size & ((1 << SIZE_BITS) - 1);
    if width == 0 || height == 0 {
        return Err(VideoError::Decode("a sequence header with no picture size".to_owned()));
    }
    let (numerator, denominator) = FRAME_RATES[usize::from(header[SEQUENCE_RATE_OFFSET] & LOW_NIBBLE)];
    Ok(SequenceHeader { geometry: VideoGeometry { width, height }, frame_us: MICROS_PER_SECOND * denominator / numerator })
}

/// Read the stream's size without decoding a picture.
pub(crate) fn probe(bytes: &[u8]) -> Result<VideoGeometry, VideoError> {
    Ok(sequence_header(&elementary_stream(bytes)?)?.geometry)
}

/// Decode the whole sequence and hand every picture on in display order.
pub(crate) fn decode(bytes: &[u8], _geometry: VideoGeometry, sink: &mut FrameSink<'_>) -> Result<(), VideoError> {
    let elementary = elementary_stream(bytes)?;
    let header = sequence_header(&elementary)?;
    let pictures = decode_video_sequence(&elementary).map_err(|err| VideoError::Decode(format!("{err}")))?;
    for (index, picture) in pictures.into_iter().enumerate() {
        let frame = picture.frame;
        let (chroma_width, chroma_height) = frame.visible_chroma_dims();
        let geometry = PlaneGeometry { width: frame.width, height: frame.height, chroma_width, chroma_height };
        let y = frame.y.packed_rect(frame.width, frame.height);
        let u = frame.cb.packed_rect(chroma_width, chroma_height);
        let v = frame.cr.packed_rect(chroma_width, chroma_height);
        let Some(rgba) = yuv420_to_rgba(geometry, &y, &u, &v) else {
            return Err(VideoError::Decode("a picture whose planes do not match its size".to_owned()));
        };
        let at = index as i64;
        let decoded = DecodedFrame {
            start_us: at * header.frame_us,
            end_us: (at + 1) * header.frame_us,
            rgba: Arc::from(rgba),
            width: frame.width as u32,
            height: frame.height as u32,
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

    /// The bytes of a `sequence_header()` for `width` x `height` at `frame_rate_code`, which is
    /// everything the reader above looks at.
    fn header_bytes(width: u32, height: u32, frame_rate_code: u8) -> Vec<u8> {
        let size = width << SIZE_BITS | height;
        let mut bytes = SEQUENCE_HEADER_CODE.to_vec();
        bytes.push((size >> 16) as u8);
        bytes.push((size >> 8) as u8);
        bytes.push(size as u8);
        bytes.push(frame_rate_code);
        bytes
    }

    #[test]
    fn a_sequence_header_gives_the_size_and_the_frame_interval() {
        let header = sequence_header(&header_bytes(320, 240, 3)).expect("read a 25 fps header");
        assert_eq!(header.geometry, VideoGeometry { width: 320, height: 240 });
        assert_eq!(header.frame_us, 40_000);
        let ntsc = sequence_header(&header_bytes(720, 480, 4)).expect("read a 30000/1001 header");
        assert_eq!(ntsc.geometry, VideoGeometry { width: 720, height: 480 });
        assert_eq!(ntsc.frame_us, 33_366);
    }

    #[test]
    fn a_reserved_frame_rate_code_falls_back_rather_than_dividing_by_zero() {
        let header = sequence_header(&header_bytes(320, 240, 0)).expect("read a header with the forbidden code");
        assert_eq!(header.frame_us, 40_000);
        let reserved = sequence_header(&header_bytes(320, 240, 15)).expect("read a header with a reserved code");
        assert_eq!(reserved.frame_us, 40_000);
    }

    #[test]
    fn a_stream_with_no_sequence_header_is_refused() {
        let error = sequence_header(&[0u8; 32]).expect_err("there is no sequence header");
        assert!(matches!(error, VideoError::Decode(_)), "got {error:?}");
    }

    #[test]
    fn an_elementary_stream_passes_through_the_demux_unchanged() {
        let bytes = header_bytes(48, 32, 3);
        assert_eq!(elementary_stream(&bytes).expect("pass an elementary stream through"), bytes);
    }
}
