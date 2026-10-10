//! Getting H.264 samples out of the shape a movie file stores them in and into the shape a decoder
//! reads.
//!
//! A movie file keeps each sample as a run of NAL units, every one preceded by its length, and
//! keeps the stream's parameter sets apart from the samples, in the track's decoder configuration
//! record. A decoder reads a byte stream instead: each NAL unit preceded by a start code, with the
//! parameter sets in the stream ahead of the first picture that needs them. [`AnnexB`] makes the
//! second out of the first.
//!
//! The parameter sets are also where a stream says how large its pictures are and what colour
//! they are in ([`SequenceInfo`]).

use crate::VideoError;
use crate::color::ColorHint;

/// The three bytes that start a NAL unit in a byte stream.
pub const START_CODE: [u8; 3] = [0, 0, 1];

/// Bytes of a decoder configuration record before its first parameter set count.
const CONFIG_FIXED_BYTES: usize = 5;

/// Which bits of the record's fifth byte say how many bytes a NAL length takes, less one.
const LENGTH_SIZE_MASK: u8 = 0x03;

/// Which bits of the byte that counts sequence parameter sets are the count.
const SPS_COUNT_MASK: u8 = 0x1f;

/// Bytes of length each parameter set of the record is preceded by.
const SET_LENGTH_BYTES: usize = 2;

/// Which bits of a NAL unit's first byte are its type.
const NAL_TYPE_MASK: u8 = 0x1f;

/// The NAL unit type of a sequence parameter set.
const NAL_SPS: u8 = 7;

/// The NAL unit type of a picture parameter set.
const NAL_PPS: u8 = 8;

/// The NAL unit type of a slice of an IDR picture.
const NAL_IDR: u8 = 5;

/// The NAL unit types that carry a slice of a picture, which is what the parameter sets have to be
/// ahead of.
const FIRST_PICTURE_KINDS: std::ops::RangeInclusive<u8> = 1..=5;

/// What a movie file's decoder configuration record carries: how long a NAL length is, and the
/// parameter sets every sample of the track is decoded with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvcConfig {
    /// Bytes each NAL unit's length takes in a sample: one to four.
    pub length_size: usize,
    pub sps: Vec<Vec<u8>>,
    pub pps: Vec<Vec<u8>>,
}

impl AvcConfig {
    /// Reads a decoder configuration record (`AVCDecoderConfigurationRecord`).
    ///
    /// A record whose counts and lengths run past its end is an error; nothing is read beyond the
    /// bytes it was given.
    pub fn parse(record: &[u8]) -> Result<AvcConfig, VideoError> {
        let short = || VideoError::Malformed("the decoder configuration is cut short".into());
        let length_size = usize::from(record.get(CONFIG_FIXED_BYTES - 1).ok_or_else(short)? & LENGTH_SIZE_MASK) + 1;
        let sps_count = usize::from(record.get(CONFIG_FIXED_BYTES).ok_or_else(short)? & SPS_COUNT_MASK);
        let (sps, after_sps) = parameter_sets(record, CONFIG_FIXED_BYTES + 1, sps_count).ok_or_else(short)?;
        let pps_count = usize::from(*record.get(after_sps).ok_or_else(short)?);
        let (pps, _) = parameter_sets(record, after_sps + 1, pps_count).ok_or_else(short)?;
        Ok(AvcConfig { length_size, sps, pps })
    }
}

/// The `count` parameter sets that start at `at` in a decoder configuration record, each behind its
/// length, and where the byte after the last of them is.
fn parameter_sets(record: &[u8], mut at: usize, count: usize) -> Option<(Vec<Vec<u8>>, usize)> {
    let mut found = Vec::with_capacity(count);
    for _ in 0..count {
        let length = record.get(at..at.checked_add(SET_LENGTH_BYTES)?)?;
        let length = usize::from(u16::from_be_bytes([length[0], length[1]]));
        at += SET_LENGTH_BYTES;
        found.push(record.get(at..at.checked_add(length)?)?.to_vec());
        at += length;
    }
    Some((found, at))
}

/// The NAL units of `sample`, each behind a length of `length_size` bytes, as far as the sample is
/// whole: one that ends in the middle of a unit or of a length is cut there. A unit of no length is
/// passed over.
fn units_of(sample: &[u8], length_size: usize) -> impl Iterator<Item = &[u8]> {
    let mut rest = sample;
    std::iter::from_fn(move || {
        loop {
            let (length, after) = rest.split_at_checked(length_size)?;
            let length = length.iter().fold(0usize, |length, byte| (length << BYTE_BITS) | usize::from(*byte));
            let (unit, after) = after.split_at_checked(length)?;
            rest = after;
            if !unit.is_empty() {
                return Some(unit);
            }
        }
    })
}

/// The kind of a NAL unit that is not empty.
fn kind_of(unit: &[u8]) -> u8 {
    unit[0] & NAL_TYPE_MASK
}

/// Rewrites a track's samples as a byte stream a decoder reads.
#[derive(Debug, Clone)]
pub struct AnnexB {
    config: AvcConfig,
    /// Whether the next sample is the first the decoder will see: at the start, and again after
    /// every [`AnnexB::rewind`].
    needs_parameter_sets: bool,
}

impl AnnexB {
    /// A converter for the samples of a track with this configuration, about to convert its first.
    pub fn new(config: AvcConfig) -> AnnexB {
        AnnexB { config, needs_parameter_sets: true }
    }

    /// Whether `sample` holds a picture a decoder that has seen nothing before it can start from:
    /// an IDR picture, which no picture after it reaches back past.
    pub fn starts_afresh(&self, sample: &[u8]) -> bool {
        units_of(sample, self.config.length_size).any(|unit| kind_of(unit) == NAL_IDR)
    }

    /// Notes that the decoder is about to be fed from the first sample again, so the parameter sets
    /// go ahead of the next sample converted.
    pub fn rewind(&mut self) {
        self.needs_parameter_sets = true;
    }

    /// Writes `sample` into `out` as a byte stream, replacing what `out` held.
    ///
    /// Each length a sample states is replaced by a start code. The first sample, and the first
    /// after a rewind, is also given the track's parameter sets -- the kinds it does not bring
    /// itself: a stream that carries its parameter sets in its samples is not handed a second copy.
    /// The sequence sets go first of all and the picture sets go just ahead of the sample's first
    /// slice, so a picture set never precedes the sequence set it names, whichever of the two the
    /// sample brought.
    ///
    /// A sample cut short is converted as far as it is whole. A NAL unit of no length is skipped.
    pub fn convert<'a>(&'a mut self, sample: &'a [u8], out: &mut Vec<u8>) {
        out.clear();
        let units: Vec<&[u8]> = units_of(sample, self.config.length_size).collect();
        let first_picture = units.iter().position(|unit| FIRST_PICTURE_KINDS.contains(&kind_of(unit))).unwrap_or(units.len());
        let (ahead, pictures) = units.split_at(first_picture);
        let needed = std::mem::take(&mut self.needs_parameter_sets);
        let is_missing = |wanted: u8| needed && !units.iter().any(|unit| kind_of(unit) == wanted);
        let sequence_sets: &[Vec<u8>] = if is_missing(NAL_SPS) { &self.config.sps } else { &[] };
        let picture_sets: &[Vec<u8>] = if is_missing(NAL_PPS) { &self.config.pps } else { &[] };
        let sets = |sets: &'a [Vec<u8>]| sets.iter().map(Vec::as_slice);
        for unit in sets(sequence_sets).chain(ahead.iter().copied()).chain(sets(picture_sets)).chain(pictures.iter().copied()) {
            if !unit.is_empty() {
                out.extend_from_slice(&START_CODE);
                out.extend_from_slice(unit);
            }
        }
    }
}

/// The profiles whose sequence parameter sets carry the chroma format and scaling lists
/// (H.264 7.3.2.1.1).
const HIGH_PROFILES: [u32; 13] = [100, 110, 122, 244, 44, 83, 86, 118, 128, 138, 139, 134, 135];

/// The chroma format a profile that does not state one has: 4:2:0.
const CHROMA_420: u32 = 1;

/// The chroma format with no chroma at all.
const CHROMA_NONE: u32 = 0;

/// The chroma format whose planes are full size, which is the only one with twelve scaling lists.
const CHROMA_444: u32 = 3;

/// Scaling lists a sequence parameter set may carry for every chroma format but 4:4:4, and for it.
const SCALING_LISTS: usize = 8;
const SCALING_LISTS_444: usize = 12;

/// How many of those lists are four by four; the rest are eight by eight.
const SMALL_SCALING_LISTS: usize = 6;
const SMALL_SCALING_LIST_ENTRIES: usize = 16;
const LARGE_SCALING_LIST_ENTRIES: usize = 64;

/// The scale a scaling list starts from, and the modulus its deltas wrap at.
const SCALING_LIST_START: i32 = 8;
const SCALING_LIST_MODULUS: i32 = 256;

/// The steps a scaling list may take from one scale to the next (`delta_scale`, H.264 7.4.2.1.1.1).
const SCALING_DELTA_RANGE: std::ops::RangeInclusive<i32> = -128..=127;

/// The most reference frame offsets a picture order count cycle may list.
const POC_CYCLE_LIMIT: u32 = 255;

/// Luma samples across and down one macroblock.
const MACROBLOCK: u32 = 16;

/// The aspect ratio code that says the ratio follows as two 16-bit numbers.
const EXTENDED_SAR: u32 = 255;

/// The largest count of leading zeros an Exp-Golomb code of 32 bits can have.
const GOLOMB_ZERO_LIMIT: u32 = 31;

/// The byte an encoder puts after two zero bytes so a payload never looks like a start code, and
/// how many zero bytes in a row it is put after.
const EMULATION_PREVENTION: u8 = 3;
const EMULATION_ZERO_RUN: usize = 2;

/// How many unsigned Exp-Golomb codes there are to each size of a signed one: the code of the
/// number above zero and, after it, the code of the number below.
const GOLOMB_CODES_PER_MAGNITUDE: u32 = 2;

/// Fields in a frame, which is how many rows of a frame one row of a field coded picture stands for.
const FIELDS_PER_FRAME: u32 = 2;

/// Luma samples one chroma sample spans along an axis its chroma format halves.
const CHROMA_SUBSAMPLING: u32 = 2;

/// Bits in a byte, and the place of a byte's first bit counted from its last.
const BYTE_BITS: usize = 8;
const BYTE_TOP_BIT: usize = BYTE_BITS - 1;

/// Bits of a sequence parameter set's profile, and of the constraint flags and level after it.
const PROFILE_BITS: u32 = 8;
const CONSTRAINTS_AND_LEVEL_BITS: u32 = 16;

/// Bits of an aspect ratio code, and of the two numbers that follow the one that says they do.
const ASPECT_RATIO_CODE_BITS: u32 = 8;
const EXTENDED_SAR_BITS: u32 = 32;

/// Bits of the video format code, of the colour primaries and transfer codes together, and of the
/// matrix code after them.
const VIDEO_FORMAT_BITS: u32 = 3;
const PRIMARIES_AND_TRANSFER_BITS: u32 = 16;
const MATRIX_BITS: u32 = 8;

/// What a sequence parameter set says about the pictures of its stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SequenceInfo {
    /// The size of a picture once it is cropped, in luma samples.
    pub width: u32,
    pub height: u32,
    /// Whether every picture is a whole frame rather than a field of one.
    pub progressive: bool,
    /// What the set's usability information says about colour, as far as it says anything.
    pub color: ColorHint,
}

/// A parameter set's payload read a bit at a time, most significant bit first.
struct Bits {
    bytes: Vec<u8>,
    at: usize,
}

impl Bits {
    /// The payload of a NAL unit with its emulation prevention bytes taken out.
    fn of_nal(unit: &[u8]) -> Bits {
        let mut bytes = Vec::with_capacity(unit.len());
        let mut zeros = 0usize;
        for &byte in unit {
            if zeros >= EMULATION_ZERO_RUN && byte == EMULATION_PREVENTION {
                zeros = 0;
                continue;
            }
            zeros = if byte == 0 { zeros + 1 } else { 0 };
            bytes.push(byte);
        }
        Bits { bytes, at: 0 }
    }

    fn bit(&mut self) -> Option<u32> {
        let byte = *self.bytes.get(self.at / BYTE_BITS)?;
        let bit = (byte >> (BYTE_TOP_BIT - self.at % BYTE_BITS)) & 1;
        self.at += 1;
        Some(u32::from(bit))
    }

    fn flag(&mut self) -> Option<bool> {
        self.bit().map(|bit| bit != 0)
    }

    fn bits(&mut self, count: u32) -> Option<u32> {
        (0..count).try_fold(0u32, |value, _| Some((value << 1) | self.bit()?))
    }

    /// An unsigned Exp-Golomb code.
    fn ue(&mut self) -> Option<u32> {
        let mut zeros = 0u32;
        while self.bit()? == 0 {
            zeros += 1;
            if zeros > GOLOMB_ZERO_LIMIT {
                return None;
            }
        }
        Some(((1u64 << zeros) - 1 + u64::from(self.bits(zeros)?)) as u32)
    }

    /// A signed Exp-Golomb code.
    fn se(&mut self) -> Option<i32> {
        let code = self.ue()?;
        let magnitude = code.div_ceil(GOLOMB_CODES_PER_MAGNITUDE) as i32;
        Some(if code % GOLOMB_CODES_PER_MAGNITUDE == 1 { magnitude } else { -magnitude })
    }

    /// Steps over one scaling list of `entries` entries, or answers `None` when it is cut short or
    /// takes a step no stream can.
    fn skip_scaling_list(&mut self, entries: usize) -> Option<()> {
        let (mut last, mut next) = (SCALING_LIST_START, SCALING_LIST_START);
        for _ in 0..entries {
            if next != 0 {
                let step = self.se().filter(|step| SCALING_DELTA_RANGE.contains(step))?;
                next = (last + step).rem_euclid(SCALING_LIST_MODULUS);
            }
            if next != 0 {
                last = next;
            }
        }
        Some(())
    }
}

impl SequenceInfo {
    /// Reads a sequence parameter set NAL unit, header byte included, or `None` when it is cut
    /// short or says something no stream can.
    pub fn parse(unit: &[u8]) -> Option<SequenceInfo> {
        if unit.first()? & NAL_TYPE_MASK != NAL_SPS {
            return None;
        }
        let mut bits = Bits::of_nal(&unit[1..]);
        let profile = bits.bits(PROFILE_BITS)?;
        bits.bits(CONSTRAINTS_AND_LEVEL_BITS)?;
        bits.ue()?;
        let mut chroma_format = CHROMA_420;
        if HIGH_PROFILES.contains(&profile) {
            chroma_format = bits.ue()?;
            if chroma_format == CHROMA_444 {
                bits.flag()?;
            }
            bits.ue()?;
            bits.ue()?;
            bits.flag()?;
            if bits.flag()? {
                let lists = if chroma_format == CHROMA_444 { SCALING_LISTS_444 } else { SCALING_LISTS };
                for list in 0..lists {
                    if bits.flag()? {
                        bits.skip_scaling_list(if list < SMALL_SCALING_LISTS { SMALL_SCALING_LIST_ENTRIES } else { LARGE_SCALING_LIST_ENTRIES })?;
                    }
                }
            }
        }
        bits.ue()?;
        match bits.ue()? {
            0 => {
                bits.ue()?;
            }
            1 => {
                bits.flag()?;
                bits.se()?;
                bits.se()?;
                let cycle = bits.ue()?;
                if cycle > POC_CYCLE_LIMIT {
                    return None;
                }
                for _ in 0..cycle {
                    bits.se()?;
                }
            }
            _ => {}
        }
        bits.ue()?;
        bits.flag()?;
        let width_in_macroblocks = bits.ue()?.checked_add(1)?;
        let height_in_map_units = bits.ue()?.checked_add(1)?;
        let progressive = bits.flag()?;
        if !progressive {
            bits.flag()?;
        }
        bits.flag()?;
        let (mut crop_left, mut crop_right, mut crop_top, mut crop_bottom) = (0u32, 0u32, 0u32, 0u32);
        if bits.flag()? {
            crop_left = bits.ue()?;
            crop_right = bits.ue()?;
            crop_top = bits.ue()?;
            crop_bottom = bits.ue()?;
        }

        let field_factor = if progressive { 1 } else { FIELDS_PER_FRAME };
        let (crop_unit_x, crop_unit_y) = match chroma_format {
            CHROMA_NONE | CHROMA_444 => (1, field_factor),
            CHROMA_420 => (CHROMA_SUBSAMPLING, CHROMA_SUBSAMPLING * field_factor),
            _ => (CHROMA_SUBSAMPLING, field_factor),
        };
        let coded_width = width_in_macroblocks.checked_mul(MACROBLOCK)?;
        let coded_height = height_in_map_units.checked_mul(MACROBLOCK)?.checked_mul(field_factor)?;
        let width = coded_width.checked_sub(crop_left.checked_add(crop_right)?.checked_mul(crop_unit_x)?)?;
        let height = coded_height.checked_sub(crop_top.checked_add(crop_bottom)?.checked_mul(crop_unit_y)?)?;
        if width == 0 || height == 0 {
            return None;
        }
        Some(SequenceInfo { width, height, progressive, color: usability_color(&mut bits).unwrap_or_default() })
    }
}

/// What the usability information at the end of a sequence parameter set says about colour. A set
/// with none, or one that ends before it gets there, says nothing.
fn usability_color(bits: &mut Bits) -> Option<ColorHint> {
    if !bits.flag()? {
        return None;
    }
    if bits.flag()? && bits.bits(ASPECT_RATIO_CODE_BITS)? == EXTENDED_SAR {
        bits.bits(EXTENDED_SAR_BITS)?;
    }
    if bits.flag()? {
        bits.flag()?;
    }
    if !bits.flag()? {
        return None;
    }
    bits.bits(VIDEO_FORMAT_BITS)?;
    let full_range = bits.flag()?;
    let mut matrix_coefficients = None;
    if bits.flag()? {
        bits.bits(PRIMARIES_AND_TRANSFER_BITS)?;
        matrix_coefficients = Some(bits.bits(MATRIX_BITS)?);
    }
    Some(ColorHint { matrix_coefficients, full_range: Some(full_range) })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A record with one sequence and one picture parameter set and four byte lengths.
    const RECORD: [u8; 14] = [1, 100, 0, 40, 0xff, 0xe1, 0, 2, 0x67, 0xaa, 1, 0, 1, 0x68];

    fn config() -> AvcConfig {
        AvcConfig::parse(&RECORD).expect("the fixture record is whole")
    }

    /// One sample holding `units`, each behind a four byte length.
    fn sample(units: &[&[u8]]) -> Vec<u8> {
        units.iter().flat_map(|unit| [&(unit.len() as u32).to_be_bytes()[..], unit].concat()).collect()
    }

    #[test]
    fn a_record_gives_up_its_length_size_and_parameter_sets() {
        assert_eq!(config(), AvcConfig { length_size: 4, sps: vec![vec![0x67, 0xaa]], pps: vec![vec![0x68]] });
        let mut two_byte = RECORD;
        two_byte[4] = 0xfd;
        assert_eq!(AvcConfig::parse(&two_byte).expect("whole").length_size, 2);
    }

    #[test]
    fn a_record_cut_short_or_overstating_itself_is_an_error() {
        for end in 0..RECORD.len() {
            assert!(AvcConfig::parse(&RECORD[..end]).is_err(), "a record cut to {end} bytes was read");
        }
        let mut long_set = RECORD;
        long_set[6] = 0xff;
        assert!(AvcConfig::parse(&long_set).is_err());
    }

    #[test]
    fn lengths_become_start_codes() {
        let mut converter = AnnexB::new(config());
        let mut out = Vec::new();
        converter.convert(&sample(&[&[0x65, 1, 2]]), &mut out);
        converter.convert(&sample(&[&[0x41, 9], &[0x06, 5, 5, 5]]), &mut out);
        assert_eq!(out, [0, 0, 1, 0x41, 9, 0, 0, 1, 0x06, 5, 5, 5]);
    }

    #[test]
    fn the_parameter_sets_go_ahead_of_the_first_sample_and_of_no_other() {
        let mut converter = AnnexB::new(config());
        let mut out = Vec::new();
        converter.convert(&sample(&[&[0x65, 1]]), &mut out);
        assert_eq!(out, [0, 0, 1, 0x67, 0xaa, 0, 0, 1, 0x68, 0, 0, 1, 0x65, 1]);
        converter.convert(&sample(&[&[0x65, 2]]), &mut out);
        assert_eq!(out, [0, 0, 1, 0x65, 2], "a later sample was given the parameter sets again, key frame or not");
    }

    #[test]
    fn the_parameter_sets_go_ahead_of_the_first_sample_after_a_rewind() {
        let mut converter = AnnexB::new(config());
        let mut out = Vec::new();
        converter.convert(&sample(&[&[0x65, 1]]), &mut out);
        converter.convert(&sample(&[&[0x41, 2]]), &mut out);
        converter.rewind();
        converter.convert(&sample(&[&[0x65, 1]]), &mut out);
        assert_eq!(out, [0, 0, 1, 0x67, 0xaa, 0, 0, 1, 0x68, 0, 0, 1, 0x65, 1]);
        converter.convert(&sample(&[&[0x41, 2]]), &mut out);
        assert_eq!(out, [0, 0, 1, 0x41, 2]);
    }

    #[test]
    fn a_sample_that_brings_its_own_parameter_sets_is_not_given_them_twice() {
        let mut converter = AnnexB::new(config());
        let mut out = Vec::new();
        converter.convert(&sample(&[&[0x67, 0xbb], &[0x68, 0xcc], &[0x65, 1]]), &mut out);
        assert_eq!(out, [0, 0, 1, 0x67, 0xbb, 0, 0, 1, 0x68, 0xcc, 0, 0, 1, 0x65, 1]);

        converter.rewind();
        converter.convert(&sample(&[&[0x67, 0xbb], &[0x65, 1]]), &mut out);
        assert_eq!(out, [0, 0, 1, 0x67, 0xbb, 0, 0, 1, 0x68, 0, 0, 1, 0x65, 1], "the picture set the sample lacks goes after its sequence set");

        converter.rewind();
        converter.convert(&sample(&[&[0x09, 0xf0], &[0x68, 0xcc], &[0x65, 1]]), &mut out);
        assert_eq!(out, [0, 0, 1, 0x67, 0xaa, 0, 0, 1, 0x09, 0xf0, 0, 0, 1, 0x68, 0xcc, 0, 0, 1, 0x65, 1], "the sequence set the sample lacks goes first");
    }

    #[test]
    fn a_sample_cut_short_is_converted_as_far_as_it_is_whole() {
        let mut converter = AnnexB::new(config());
        let mut out = Vec::new();
        converter.convert(&sample(&[&[0x65, 1]]), &mut out);

        let whole = sample(&[&[0x41, 1, 2], &[0x41, 3, 4]]);
        converter.convert(&whole[..whole.len() - 1], &mut out);
        assert_eq!(out, [0, 0, 1, 0x41, 1, 2]);

        converter.convert(&[0, 0], &mut out);
        assert!(out.is_empty(), "a length cut short was read as a unit");

        converter.convert(&[0xff, 0xff, 0xff, 0xff, 1, 2, 3], &mut out);
        assert!(out.is_empty(), "a length past the end of the sample was believed");
    }

    #[test]
    fn a_sample_starts_afresh_only_when_it_holds_an_idr_picture() {
        let converter = AnnexB::new(config());
        assert!(converter.starts_afresh(&sample(&[&[0x06, 5], &[0x65, 1, 2]])));
        assert!(!converter.starts_afresh(&sample(&[&[0x06, 5], &[0x41, 1, 2]])), "a picture that leans on the ones before it was taken for a start");
        assert!(!converter.starts_afresh(&[]));
        let whole = sample(&[&[0x41, 1], &[0x65, 1, 2]]);
        assert!(!converter.starts_afresh(&whole[..whole.len() - 1]), "a picture cut short was taken for a start");
    }

    #[test]
    fn empty_units_and_empty_samples_write_nothing() {
        let mut converter = AnnexB::new(AvcConfig { length_size: 1, sps: Vec::new(), pps: Vec::new() });
        let mut out = vec![1, 2, 3];
        converter.convert(&[], &mut out);
        assert!(out.is_empty());
        converter.convert(&[0, 2, 0x41, 7, 0], &mut out);
        assert_eq!(out, [0, 0, 1, 0x41, 7]);
    }

    /// Writes bits most significant first, for building parameter sets by hand.
    #[derive(Default)]
    struct BitWriter {
        bits: Vec<bool>,
    }

    impl BitWriter {
        fn put(&mut self, value: u32, count: u32) -> &mut Self {
            self.bits.extend((0..count).rev().map(|bit| (value >> bit) & 1 == 1));
            self
        }

        fn ue(&mut self, value: u32) -> &mut Self {
            let code = value + 1;
            let width = 32 - code.leading_zeros();
            self.put(0, width - 1).put(code, width)
        }

        fn se(&mut self, value: i32) -> &mut Self {
            self.ue(if value > 0 { (value as u32) * 2 - 1 } else { value.unsigned_abs() * 2 })
        }

        /// The bits as a NAL unit of type `header`, with the stop bit and the emulation prevention
        /// bytes a real one has.
        fn nal(&self, header: u8) -> Vec<u8> {
            let mut bits = self.bits.clone();
            bits.push(true);
            while !bits.len().is_multiple_of(BYTE_BITS) {
                bits.push(false);
            }
            let payload: Vec<u8> = bits.chunks(BYTE_BITS).map(|byte| byte.iter().fold(0u8, |value, bit| (value << 1) | u8::from(*bit))).collect();
            let mut out = vec![header];
            let mut zeros = 0;
            for byte in payload {
                if zeros >= EMULATION_ZERO_RUN && byte <= EMULATION_PREVENTION {
                    out.push(EMULATION_PREVENTION);
                    zeros = 0;
                }
                zeros = if byte == 0 { zeros + 1 } else { 0 };
                out.push(byte);
            }
            out
        }
    }

    /// A High profile set for a 1920 by 1080 progressive stream: 120 by 68 macroblocks with four
    /// chroma rows cropped off the bottom.
    fn high_1080(writer: &mut BitWriter) {
        writer.put(100, 8).put(0, 8).put(40, 8).ue(0);
        writer.ue(1).ue(0).ue(0).put(0, 1).put(0, 1);
        writer.ue(0).ue(0).ue(4);
        writer.ue(4).put(0, 1).ue(119).ue(67).put(1, 1).put(1, 1);
        writer.put(1, 1).ue(0).ue(0).ue(0).ue(4);
    }

    #[test]
    fn a_cropped_high_profile_set_says_its_picture_size() {
        let mut writer = BitWriter::default();
        high_1080(&mut writer);
        writer.put(0, 1);
        let info = SequenceInfo::parse(&writer.nal(0x67)).expect("the set is whole");
        assert_eq!((info.width, info.height, info.progressive), (1920, 1080, true));
        assert_eq!(info.color, ColorHint::default());
    }

    #[test]
    fn the_usability_information_says_the_matrix_and_the_range() {
        let mut writer = BitWriter::default();
        high_1080(&mut writer);
        writer.put(1, 1);
        writer.put(1, 1).put(255, 8).put(1, 16).put(1, 16);
        writer.put(0, 1);
        writer.put(1, 1).put(5, 3).put(1, 1).put(1, 1).put(1, 8).put(1, 8).put(6, 8);
        let info = SequenceInfo::parse(&writer.nal(0x67)).expect("the set is whole");
        assert_eq!((info.width, info.height), (1920, 1080));
        assert_eq!(info.color, ColorHint { matrix_coefficients: Some(6), full_range: Some(true) });
    }

    #[test]
    fn a_baseline_set_with_scaling_and_offsets_elsewhere_is_read_too() {
        let mut writer = BitWriter::default();
        writer.put(66, 8).put(0, 8).put(30, 8).ue(0);
        writer.ue(0).ue(1).put(0, 1).se(-3).se(2).ue(2).se(1).se(-1);
        writer.ue(1).put(0, 1).ue(19).ue(14).put(1, 1).put(1, 1).put(0, 1).put(0, 1);
        let info = SequenceInfo::parse(&writer.nal(0x27)).expect("the set is whole");
        assert_eq!((info.width, info.height, info.progressive), (320, 240, true));

        let mut scaled = BitWriter::default();
        scaled.put(100, 8).put(0, 8).put(31, 8).ue(0);
        scaled.ue(1).ue(0).ue(0).put(0, 1).put(1, 1);
        scaled.put(1, 1);
        (0..SMALL_SCALING_LIST_ENTRIES).for_each(|_| {
            scaled.se(1);
        });
        scaled.put(1, 1).se(-8);
        (2..SCALING_LISTS).for_each(|_| {
            scaled.put(0, 1);
        });
        scaled.ue(0).ue(2).ue(1).put(0, 1).ue(79).ue(44).put(1, 1).put(1, 1).put(0, 1).put(0, 1);
        let info = SequenceInfo::parse(&scaled.nal(0x67)).expect("the set is whole");
        assert_eq!((info.width, info.height), (1280, 720));
    }

    #[test]
    fn a_scaling_list_step_no_stream_can_have_is_not_read_and_does_not_overflow() {
        for step in [i32::MAX, SCALING_DELTA_RANGE.end() + 1, SCALING_DELTA_RANGE.start() - 1] {
            let mut scaled = BitWriter::default();
            scaled.put(100, 8).put(0, 8).put(31, 8).ue(0);
            scaled.ue(1).ue(0).ue(0).put(0, 1).put(1, 1);
            scaled.put(1, 1).se(step);
            assert_eq!(SequenceInfo::parse(&scaled.nal(0x67)), None, "a set whose scaling list steps by {step} was read");
        }
    }

    #[test]
    fn a_field_coded_set_doubles_its_height_and_says_it_is_not_progressive() {
        let mut writer = BitWriter::default();
        writer.put(77, 8).put(0, 8).put(30, 8).ue(0);
        writer.ue(0).ue(2).ue(1).put(0, 1).ue(44).ue(17).put(0, 1).put(0, 1).put(1, 1).put(0, 1).put(0, 1);
        let info = SequenceInfo::parse(&writer.nal(0x67)).expect("the set is whole");
        assert_eq!((info.width, info.height, info.progressive), (720, 576, false));
    }

    #[test]
    fn a_set_cut_short_or_of_the_wrong_kind_is_not_read() {
        let mut writer = BitWriter::default();
        high_1080(&mut writer);
        writer.put(0, 1);
        let whole = writer.nal(0x67);
        for end in 0..whole.len() - 1 {
            assert_eq!(SequenceInfo::parse(&whole[..end]), None, "a set cut to {end} bytes was read");
        }
        assert_eq!(SequenceInfo::parse(&writer.nal(0x68)), None);
        assert_eq!(SequenceInfo::parse(&[0x67, 0xff, 0xff, 0xff, 0, 0, 0, 0, 0, 0, 0, 0]), None, "a run of zeros was read as a number");
    }

    #[test]
    fn emulation_prevention_bytes_are_not_read_as_payload() {
        let mut bits = Bits::of_nal(&[0, 0, 3, 1, 0, 0, 3]);
        assert_eq!(bits.bits(32), Some(256));
        assert_eq!(bits.bits(8), Some(0));
        assert_eq!(bits.bit(), None);
    }
}
