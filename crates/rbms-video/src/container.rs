//! Walking the boxes of an ISO base media file before anything else is allowed to read them.
//!
//! A movie file is a tree of boxes, each saying how long it is. A parser that believes those
//! lengths can be made to loop for ever by a box of no length, or to ask for more memory than the
//! machine has by a box longer than its file. So nothing here hands a file to a box parser as it
//! is: the tree is walked once with every length checked against the box it sits in, the one track
//! this crate decodes is picked out of it, and a small movie header holding only that track's
//! tables is written out again ([`rebuild_moov`]). What a parser reads afterwards is that rebuilt
//! header, whose every length this module wrote itself.
//!
//! The walk allocates nothing that grows with a length a file states: it only ever takes slices of
//! bytes it was given.

use std::io::{Read, Seek, SeekFrom};

use crate::VideoError;
use crate::color::ColorHint;

/// The four characters a box is named by.
pub type FourCc = [u8; 4];

/// Bytes a box header takes: its length and its name.
pub const BOX_HEADER_BYTES: usize = 8;

/// Bytes of a box header that are its 32-bit length, ahead of its name.
const LENGTH_BYTES: usize = size_of::<u32>();

/// Bytes a box header takes when its length is written as 64 bits after the name.
const LARGE_HEADER_BYTES: usize = 16;

/// The 32-bit length that says the real one follows the name as 64 bits.
const LENGTH_FOLLOWS: u32 = 1;

/// The 32-bit length that says the box runs to the end of whatever holds it.
const LENGTH_TO_END: u32 = 0;

/// The most bytes of movie header that are read at all. The header of a two hour film at sixty
/// frames a second is a few megabytes of tables.
pub const MOVIE_HEADER_LIMIT: u64 = 64 * 1024 * 1024;

/// The most top-level boxes that are stepped over looking for the movie header.
const TOP_LEVEL_BOX_LIMIT: usize = 65_536;

/// Bytes of version and flags a full box starts with.
const FULL_BOX_PREFIX: usize = 4;

/// Bytes of the entry count a table box carries after its version and flags.
const ENTRY_COUNT_BYTES: usize = size_of::<u32>();

/// Where a handler box's payload keeps its handler type: after its version and flags and one
/// reserved word.
const HANDLER_TYPE_AT: usize = 8;

/// The least a handler box's payload can be: everything up to and including its reserved words.
const HANDLER_MIN_BYTES: usize = 24;

/// Bytes of a sample description box's payload before its first entry: version, flags and the entry
/// count.
const DESCRIPTION_PREFIX: usize = 8;

/// Bytes of fixed fields a visual sample entry starts with, before the boxes nested in it.
const VISUAL_ENTRY_BYTES: usize = 78;

/// Bytes of a decoder configuration record before its first parameter set count.
const AVC_CONFIG_FIXED: usize = 5;

/// Which bits of the byte that counts sequence parameter sets are the count.
const SPS_COUNT_MASK: u8 = 0x1f;

/// Bytes of length each parameter set of a decoder configuration record is preceded by.
const PARAMETER_SET_LENGTH_BYTES: usize = 2;

/// Bytes of a colour box's payload that carry its three code points after its four character type.
const COLOUR_CODES_AT: usize = 4;

/// Where the matrix code point is among those three: after the primaries and the transfer codes.
const COLOUR_MATRIX_AT: usize = COLOUR_CODES_AT + 2 * size_of::<u16>();

/// Bytes a colour box of the `nclx` kind holds: its type, three 16-bit code points and a flag byte.
const COLOUR_NCLX_BYTES: usize = 11;

/// The bit of an `nclx` colour box's last byte that says the samples use the full range.
const COLOUR_FULL_RANGE_BIT: u8 = 0x80;

const MOOV: FourCc = *b"moov";
const MVHD: FourCc = *b"mvhd";
const TRAK: FourCc = *b"trak";
const TKHD: FourCc = *b"tkhd";
const MDIA: FourCc = *b"mdia";
const MDHD: FourCc = *b"mdhd";
const HDLR: FourCc = *b"hdlr";
const MINF: FourCc = *b"minf";
const DINF: FourCc = *b"dinf";
const DREF: FourCc = *b"dref";
const STBL: FourCc = *b"stbl";
const STSD: FourCc = *b"stsd";
const STTS: FourCc = *b"stts";
const CTTS: FourCc = *b"ctts";
const STSS: FourCc = *b"stss";
const STSC: FourCc = *b"stsc";
const STSZ: FourCc = *b"stsz";
const STCO: FourCc = *b"stco";
const CO64: FourCc = *b"co64";
const AVC1: FourCc = *b"avc1";
const AVC3: FourCc = *b"avc3";
const AVCC: FourCc = *b"avcC";
const COLR: FourCc = *b"colr";
const NCLX: FourCc = *b"nclx";
const VIDEO_HANDLER: FourCc = *b"vide";

/// One box inside another: its name and everything after its header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoxRef<'a> {
    pub kind: FourCc,
    pub payload: &'a [u8],
}

/// The boxes laid end to end in `bytes`, each checked to fit in what is left of it.
///
/// The first box that does not fit -- shorter than its own header, or longer than the bytes that
/// remain -- is answered as an error and ends the walk, so a caller that stops at the first error
/// never reads past a length it could not trust.
pub fn boxes(bytes: &[u8]) -> Boxes<'_> {
    Boxes { rest: bytes, failed: false }
}

/// The iterator behind [`boxes`].
#[derive(Debug, Clone)]
pub struct Boxes<'a> {
    rest: &'a [u8],
    failed: bool,
}

/// The four bytes at `at`, read as a big-endian number.
fn be_u32(bytes: &[u8], at: usize) -> Option<u32> {
    let field: [u8; size_of::<u32>()] = bytes.get(at..at.checked_add(size_of::<u32>())?)?.try_into().ok()?;
    Some(u32::from_be_bytes(field))
}

/// The eight bytes at `at`, read as a big-endian number.
fn be_u64(bytes: &[u8], at: usize) -> Option<u64> {
    let field: [u8; size_of::<u64>()] = bytes.get(at..at.checked_add(size_of::<u64>())?)?.try_into().ok()?;
    Some(u64::from_be_bytes(field))
}

/// The two bytes at `at`, read as a big-endian number.
fn be_u16(bytes: &[u8], at: usize) -> Option<u16> {
    let field: [u8; size_of::<u16>()] = bytes.get(at..at.checked_add(size_of::<u16>())?)?.try_into().ok()?;
    Some(u16::from_be_bytes(field))
}

/// The header at the front of `bytes`: the box's name, how many bytes its header takes, and how
/// many the whole box takes when it says. `None` for the length means the box runs to the end of
/// whatever holds it.
fn header(bytes: &[u8]) -> Result<(FourCc, usize, Option<u64>), VideoError> {
    let short = || VideoError::Malformed("a box header is cut short".into());
    let length = be_u32(bytes, 0).ok_or_else(short)?;
    let kind: FourCc = bytes.get(LENGTH_BYTES..BOX_HEADER_BYTES).and_then(|name| name.try_into().ok()).ok_or_else(short)?;
    match length {
        LENGTH_TO_END => Ok((kind, BOX_HEADER_BYTES, None)),
        LENGTH_FOLLOWS => {
            let long = be_u64(bytes, BOX_HEADER_BYTES).ok_or_else(short)?;
            if long < LARGE_HEADER_BYTES as u64 {
                return Err(VideoError::Malformed("a box is shorter than its own header".into()));
            }
            Ok((kind, LARGE_HEADER_BYTES, Some(long)))
        }
        length if (length as usize) < BOX_HEADER_BYTES => Err(VideoError::Malformed("a box is shorter than its own header".into())),
        length => Ok((kind, BOX_HEADER_BYTES, Some(u64::from(length)))),
    }
}

impl<'a> Iterator for Boxes<'a> {
    type Item = Result<BoxRef<'a>, VideoError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.failed || self.rest.is_empty() {
            return None;
        }
        let parsed = header(self.rest).and_then(|(kind, header_bytes, length)| {
            let length = length.unwrap_or(self.rest.len() as u64);
            let length = usize::try_from(length).ok().filter(|length| *length <= self.rest.len());
            let length = length.ok_or_else(|| VideoError::Malformed(format!("box {:?} is longer than what holds it", name(kind))))?;
            Ok((BoxRef { kind, payload: &self.rest[header_bytes..length] }, length))
        });
        match parsed {
            Ok((found, length)) => {
                self.rest = &self.rest[length..];
                Some(Ok(found))
            }
            Err(error) => {
                self.failed = true;
                Some(Err(error))
            }
        }
    }
}

/// A box name as text, for a message.
fn name(kind: FourCc) -> String {
    String::from_utf8_lossy(&kind).into_owned()
}

/// The payload of the first box named `kind` directly inside `bytes`, with every box before it
/// checked on the way.
fn child(bytes: &[u8], kind: FourCc) -> Result<Option<&[u8]>, VideoError> {
    for found in boxes(bytes) {
        let found = found?;
        if found.kind == kind {
            return Ok(Some(found.payload));
        }
    }
    Ok(None)
}

/// [`child`], for a box the file cannot do without.
fn required(bytes: &[u8], kind: FourCc) -> Result<&[u8], VideoError> {
    child(bytes, kind)?.ok_or_else(|| VideoError::Malformed(format!("box {:?} is missing", name(kind))))
}

/// Finds the movie header of the file behind `reader`, which is `file_bytes` long, and reads its
/// payload.
///
/// Only headers are read on the way there: the media data is stepped over, wherever it sits. A box
/// that says it is longer than the file, a header that is cut short and a movie header past
/// [`MOVIE_HEADER_LIMIT`] are each an error rather than something to allocate for.
pub fn read_movie_header<R: Read + Seek>(reader: &mut R, file_bytes: u64) -> Result<Vec<u8>, VideoError> {
    let mut at = 0u64;
    for _ in 0..TOP_LEVEL_BOX_LIMIT {
        if at >= file_bytes {
            break;
        }
        let mut front = [0u8; LARGE_HEADER_BYTES];
        let wanted = usize::try_from(file_bytes - at).map_or(LARGE_HEADER_BYTES, |left| left.min(LARGE_HEADER_BYTES));
        reader.seek(SeekFrom::Start(at)).map_err(VideoError::io)?;
        reader.read_exact(&mut front[..wanted]).map_err(VideoError::io)?;
        let (kind, header_bytes, length) = header(&front[..wanted])?;
        let length = length.unwrap_or(file_bytes - at);
        let end = at.checked_add(length).filter(|end| *end <= file_bytes);
        let end = end.ok_or_else(|| VideoError::Malformed(format!("box {:?} is longer than the file", name(kind))))?;
        if kind == MOOV {
            let payload_bytes = length - header_bytes as u64;
            if payload_bytes > MOVIE_HEADER_LIMIT {
                return Err(VideoError::Malformed(format!("the movie header is {payload_bytes} bytes, past the {MOVIE_HEADER_LIMIT} read")));
            }
            let mut payload = vec![0u8; payload_bytes as usize];
            reader.seek(SeekFrom::Start(at + header_bytes as u64)).map_err(VideoError::io)?;
            reader.read_exact(&mut payload).map_err(VideoError::io)?;
            return Ok(payload);
        }
        at = end;
    }
    Err(VideoError::Malformed("the file has no movie header".into()))
}

/// The boxes of the one video track a movie is decoded from, each as its payload.
#[derive(Debug, Clone, Copy)]
pub struct AvcTrack<'a> {
    pub movie_header: &'a [u8],
    pub track_header: &'a [u8],
    pub media_header: &'a [u8],
    pub handler: &'a [u8],
    pub time_to_sample: &'a [u8],
    pub composition_offsets: Option<&'a [u8]>,
    pub sync_samples: Option<&'a [u8]>,
    pub sample_to_chunk: &'a [u8],
    pub sample_sizes: &'a [u8],
    pub chunk_offsets: Option<&'a [u8]>,
    pub chunk_offsets_64: Option<&'a [u8]>,
    /// The fixed fields of the track's visual sample entry.
    pub entry: &'a [u8],
    /// The decoder configuration record nested in that entry.
    pub avc_config: &'a [u8],
    /// What the entry's colour box says about the samples, when it has one.
    pub color: ColorHint,
}

/// Whether a decoder configuration record's parameter sets all lie inside it.
///
/// The record counts its sets and gives each a length, and nothing but this says the counts and
/// lengths agree with how long the record is.
fn avc_config_is_whole(config: &[u8]) -> bool {
    let sets = |mut at: usize, count: usize| {
        for _ in 0..count {
            let length = usize::from(be_u16(config, at)?);
            at = at.checked_add(PARAMETER_SET_LENGTH_BYTES)?.checked_add(length)?;
        }
        (at <= config.len()).then_some(at)
    };
    let Some(sps_count) = config.get(AVC_CONFIG_FIXED) else {
        return false;
    };
    let Some(after_sps) = sets(AVC_CONFIG_FIXED + 1, usize::from(sps_count & SPS_COUNT_MASK)) else {
        return false;
    };
    let Some(pps_count) = config.get(after_sps) else {
        return false;
    };
    sets(after_sps + 1, usize::from(*pps_count)).is_some()
}

/// What a colour box says, when it is of the kind that carries code points.
fn color_hint(colour: &[u8]) -> ColorHint {
    if colour.len() < COLOUR_NCLX_BYTES || colour[..COLOUR_CODES_AT] != NCLX {
        return ColorHint::default();
    }
    let matrix = be_u16(colour, COLOUR_MATRIX_AT).map(u32::from);
    let full_range = colour[COLOUR_NCLX_BYTES - 1] & COLOUR_FULL_RANGE_BIT != 0;
    ColorHint { matrix_coefficients: matrix, full_range: Some(full_range) }
}

/// An H.264 sample entry: its fixed fields, its decoder configuration record and its colour.
type AvcEntry<'a> = (&'a [u8], &'a [u8], ColorHint);

/// The sample entry of a track's description box, when it is an H.264 one.
fn avc_entry(description: &[u8]) -> Result<Option<AvcEntry<'_>>, VideoError> {
    let Some(entries) = description.get(DESCRIPTION_PREFIX..) else {
        return Err(VideoError::Malformed("a sample description is cut short".into()));
    };
    let Some(first) = boxes(entries).next() else {
        return Ok(None);
    };
    let first = first?;
    if first.kind != AVC1 && first.kind != AVC3 {
        return Ok(None);
    }
    let Some((fixed, nested)) = first.payload.split_at_checked(VISUAL_ENTRY_BYTES) else {
        return Err(VideoError::Malformed("a visual sample entry is cut short".into()));
    };
    let config = required(nested, AVCC)?;
    if !avc_config_is_whole(config) {
        return Err(VideoError::Malformed("the decoder configuration's parameter sets run past its end".into()));
    }
    let color = child(nested, COLR)?.map(color_hint).unwrap_or_default();
    Ok(Some((fixed, config, color)))
}

/// The track of one `trak` box, when it is a video track of H.264 samples.
fn avc_track<'a>(movie_header: &'a [u8], track: &'a [u8]) -> Result<Option<AvcTrack<'a>>, VideoError> {
    let media = required(track, MDIA)?;
    let handler = required(media, HDLR)?;
    if handler.len() < HANDLER_MIN_BYTES || handler[HANDLER_TYPE_AT..HANDLER_TYPE_AT + VIDEO_HANDLER.len()] != VIDEO_HANDLER {
        return Ok(None);
    }
    let tables = required(required(media, MINF)?, STBL)?;
    let Some((entry, avc_config, color)) = avc_entry(required(tables, STSD)?)? else {
        return Ok(None);
    };
    let (chunk_offsets, chunk_offsets_64) = (child(tables, STCO)?, child(tables, CO64)?);
    if chunk_offsets.is_none() && chunk_offsets_64.is_none() {
        return Err(VideoError::Malformed("the track has no chunk offsets".into()));
    }
    Ok(Some(AvcTrack {
        movie_header,
        track_header: required(track, TKHD)?,
        media_header: required(media, MDHD)?,
        handler,
        time_to_sample: required(tables, STTS)?,
        composition_offsets: child(tables, CTTS)?,
        sync_samples: child(tables, STSS)?,
        sample_to_chunk: required(tables, STSC)?,
        sample_sizes: required(tables, STSZ)?,
        chunk_offsets,
        chunk_offsets_64,
        entry,
        avc_config,
        color,
    }))
}

/// The first video track of H.264 samples in a movie header's payload.
///
/// Every box on the way to it is checked to fit in the box that holds it. Sound, timecode and
/// subtitle tracks are stepped over without being looked into, as is everything that is not one of
/// the tables a decode needs: user data, edit lists, metadata.
pub fn first_avc_track(movie: &[u8]) -> Result<AvcTrack<'_>, VideoError> {
    let movie_header = required(movie, MVHD)?;
    for found in boxes(movie) {
        let found = found?;
        if found.kind != TRAK {
            continue;
        }
        if let Some(track) = avc_track(movie_header, found.payload)? {
            return Ok(track);
        }
    }
    Err(VideoError::Unsupported("the file has no H.264 video track".into()))
}

/// Writes one box into `out`: its header, then whatever `fill` writes as its payload.
pub(crate) fn write_box(out: &mut Vec<u8>, kind: FourCc, fill: impl FnOnce(&mut Vec<u8>)) {
    let start = out.len();
    out.extend_from_slice(&[0; LENGTH_BYTES]);
    out.extend_from_slice(&kind);
    fill(out);
    let length = u32::try_from(out.len() - start).unwrap_or(u32::MAX);
    out[start..start + LENGTH_BYTES].copy_from_slice(&length.to_be_bytes());
}

/// Writes one box whose payload is `payload` as it stands.
pub(crate) fn copy_box(out: &mut Vec<u8>, kind: FourCc, payload: &[u8]) {
    write_box(out, kind, |out| out.extend_from_slice(payload));
}

/// A whole `moov` box holding nothing but `track`: the boxes a table parser reads, each with a
/// length written here.
///
/// The sample entry is written as `avc1` whichever of the two H.264 entry names the file used, and
/// carries its decoder configuration record alone. The data reference a media information box has
/// to have is written empty.
pub fn rebuild_moov(track: &AvcTrack<'_>) -> Vec<u8> {
    let mut out = Vec::new();
    write_box(&mut out, MOOV, |out| {
        copy_box(out, MVHD, track.movie_header);
        write_box(out, TRAK, |out| {
            copy_box(out, TKHD, track.track_header);
            write_box(out, MDIA, |out| {
                copy_box(out, MDHD, track.media_header);
                copy_box(out, HDLR, track.handler);
                write_box(out, MINF, |out| {
                    write_box(out, DINF, |out| copy_box(out, DREF, &[0; FULL_BOX_PREFIX + ENTRY_COUNT_BYTES]));
                    write_box(out, STBL, |out| {
                        write_box(out, STSD, |out| {
                            out.extend_from_slice(&[0; FULL_BOX_PREFIX]);
                            out.extend_from_slice(&1u32.to_be_bytes());
                            write_box(out, AVC1, |out| {
                                out.extend_from_slice(track.entry);
                                copy_box(out, AVCC, track.avc_config);
                            });
                        });
                        copy_box(out, STTS, track.time_to_sample);
                        for (kind, table) in [(CTTS, track.composition_offsets), (STSS, track.sync_samples)] {
                            if let Some(table) = table {
                                copy_box(out, kind, table);
                            }
                        }
                        copy_box(out, STSC, track.sample_to_chunk);
                        copy_box(out, STSZ, track.sample_sizes);
                        for (kind, table) in [(STCO, track.chunk_offsets), (CO64, track.chunk_offsets_64)] {
                            if let Some(table) = table {
                                copy_box(out, kind, table);
                            }
                        }
                    });
                });
            });
        });
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// A box of `kind` around `payload`.
    fn boxed(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        copy_box(&mut out, *kind, payload);
        out
    }

    #[test]
    fn boxes_are_walked_in_order_with_their_payloads() {
        let bytes = [boxed(b"abcd", &[1, 2, 3]), boxed(b"wxyz", &[])].concat();
        let found: Vec<BoxRef<'_>> = boxes(&bytes).collect::<Result<_, _>>().expect("two well formed boxes");
        assert_eq!(found, vec![BoxRef { kind: *b"abcd", payload: &[1, 2, 3] }, BoxRef { kind: *b"wxyz", payload: &[] }]);
    }

    #[test]
    fn a_box_of_no_length_runs_to_the_end_and_never_repeats() {
        let bytes = [0, 0, 0, 0, b'f', b'r', b'e', b'e', 9, 9];
        let found: Vec<BoxRef<'_>> = boxes(&bytes).collect::<Result<_, _>>().expect("a box that runs to the end");
        assert_eq!(found, vec![BoxRef { kind: *b"free", payload: &[9, 9] }]);
    }

    #[test]
    fn a_box_longer_than_what_holds_it_ends_the_walk_with_an_error() {
        let mut bytes = boxed(b"abcd", &[1, 2, 3]);
        bytes[..4].copy_from_slice(&u32::MAX.to_be_bytes());
        let mut walk = boxes(&bytes);
        assert!(matches!(walk.next(), Some(Err(VideoError::Malformed(_)))));
        assert!(walk.next().is_none(), "the walk went on past a length it could not trust");
    }

    #[test]
    fn a_box_shorter_than_its_header_is_refused() {
        for length in [2u32, 7] {
            let mut bytes = boxed(b"abcd", &[1, 2, 3]);
            bytes[..4].copy_from_slice(&length.to_be_bytes());
            assert!(matches!(boxes(&bytes).next(), Some(Err(VideoError::Malformed(_)))), "a length of {length} was walked");
        }
    }

    #[test]
    fn a_sixty_four_bit_length_is_held_to_the_same_bounds() {
        let large = |length: u64| [&1u32.to_be_bytes()[..], b"wide", &length.to_be_bytes()[..], &[5, 6]].concat();
        let whole = large(18);
        assert_eq!(boxes(&whole).next().and_then(Result::ok), Some(BoxRef { kind: *b"wide", payload: &[5, 6] }));
        for absurd in [u64::MAX, 1 << 40, 15, 0] {
            assert!(matches!(boxes(&large(absurd)).next(), Some(Err(VideoError::Malformed(_)))), "a 64-bit length of {absurd} was walked");
        }
    }

    #[test]
    fn the_movie_header_is_found_behind_the_media_data_without_reading_it() {
        let file = [boxed(b"ftyp", b"isom"), boxed(b"mdat", &[0; 64]), boxed(b"moov", &[7, 7, 7])].concat();
        let payload = read_movie_header(&mut Cursor::new(&file), file.len() as u64).expect("the header is there");
        assert_eq!(payload, vec![7, 7, 7]);
    }

    #[test]
    fn a_movie_header_past_the_end_of_its_file_is_refused_without_allocating_for_it() {
        let mut file = [boxed(b"ftyp", b"isom"), boxed(b"moov", &[7, 7, 7])].concat();
        let moov = file.len() - 11;
        file[moov..moov + 4].copy_from_slice(&0x7fff_ffffu32.to_be_bytes());
        assert!(matches!(read_movie_header(&mut Cursor::new(&file), file.len() as u64), Err(VideoError::Malformed(_))));

        let huge = [&1u32.to_be_bytes()[..], b"moov", &(1u64 << 50).to_be_bytes()[..]].concat();
        assert!(matches!(read_movie_header(&mut Cursor::new(&huge), huge.len() as u64), Err(VideoError::Malformed(_))));
    }

    #[test]
    fn a_file_with_no_movie_header_and_a_file_that_is_not_boxes_are_errors() {
        let no_header = boxed(b"ftyp", b"isom");
        assert!(read_movie_header(&mut Cursor::new(&no_header), no_header.len() as u64).is_err());
        let noise: Vec<u8> = (0..4096u32).map(|index| (index * 31 % 251) as u8).collect();
        assert!(read_movie_header(&mut Cursor::new(&noise), noise.len() as u64).is_err());
        assert!(read_movie_header(&mut Cursor::new(&[] as &[u8]), 0).is_err());
    }

    #[test]
    fn parameter_sets_that_run_past_their_record_are_noticed() {
        let whole = [1, 100, 0, 40, 0xff, 0xe1, 0, 2, 0x67, 1, 1, 0, 1, 0x68];
        assert!(avc_config_is_whole(&whole));
        let mut long_sps = whole;
        long_sps[7] = 200;
        assert!(!avc_config_is_whole(&long_sps));
        let mut many_pps = whole;
        many_pps[10] = 9;
        assert!(!avc_config_is_whole(&many_pps));
        assert!(!avc_config_is_whole(&whole[..5]));
        assert!(!avc_config_is_whole(&[]));
    }

    #[test]
    fn a_colour_box_is_read_only_when_it_carries_code_points() {
        let nclx = [&b"nclx"[..], &[0, 1, 0, 1, 0, 6, 0x80]].concat();
        assert_eq!(color_hint(&nclx), ColorHint { matrix_coefficients: Some(6), full_range: Some(true) });
        assert_eq!(color_hint(b"prof\0\0\0\0\0\0\0\0"), ColorHint::default());
        assert_eq!(color_hint(b"nclx"), ColorHint::default());
    }
}
