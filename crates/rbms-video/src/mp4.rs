//! Reading where an MP4 file keeps the samples of its H.264 track.
//!
//! The file's movie header is walked and cut down to the one video track first
//! ([`crate::container`]), and only that rebuilt header is handed to `re_mp4`, which turns its
//! boxes into tables. The tables are then expanded here into one entry per sample -- where it is
//! in the file, how long it is and when it is shown -- with every sum checked: a table that
//! contradicts another, counts more samples than a file could hold or points past the end of the
//! file is an error, never an index out of range.
//!
//! Edit lists are not applied. A track is played from its first sample to its last, each for the
//! time the table gives it.
//!
//! The sync sample table is read for where decoding can start other than at the first sample
//! ([`Movie::keys`]), which is what lets a clock that jumps be caught up with from the picture
//! before where it landed and not from the start of the movie.

use std::io::{Cursor, Read, Seek};

use re_mp4::{MoovBox, ReadBox, StblBox, StsdBoxContent};

use crate::bitstream::{AvcConfig, SequenceInfo};
use crate::color::{ColorHint, ColorSpec};
use crate::container::{BOX_HEADER_BYTES, first_avc_track, read_movie_header, rebuild_moov};
use crate::{MICROS_PER_SECOND, VideoError, VideoInfo};

/// The most samples a track is read with: nineteen hours of video at sixty frames a second.
const SAMPLE_LIMIT: u32 = 1 << 22;

/// The most samples room is made for before any of them has been found to lie inside the file.
const SAMPLE_RESERVE_LIMIT: usize = 1 << 16;

/// The most bytes one sample may be. The decoder itself refuses a compressed frame past a
/// megabyte; this only keeps a table from asking for a buffer the size of the file.
const SAMPLE_BYTES_LIMIT: u64 = 16 * 1024 * 1024;

/// The longest edge a frame may have, which is what the decoder's highest level allows.
const FRAME_EDGE_LIMIT: u32 = 8192;

/// One sample of the track.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Sample {
    /// Where the sample starts in the file.
    pub(crate) offset: u64,
    pub(crate) bytes: usize,
}

/// Everything about a movie file that decoding its video track needs.
#[derive(Debug, Clone)]
pub(crate) struct Movie {
    pub(crate) info: VideoInfo,
    pub(crate) config: AvcConfig,
    pub(crate) color: ColorSpec,
    /// The samples in the order they are stored and decoded.
    pub(crate) samples: Vec<Sample>,
    /// When each frame is shown, in microseconds from the start of the movie, in the order the
    /// frames are shown: the n-th frame out of the decoder is shown at the n-th of these.
    pub(crate) show_us: Vec<i64>,
    /// The samples the file says decoding can start from, in order, the first sample always among
    /// them. Sample `k` of these is also the `k`-th frame shown: every sample before it is shown
    /// before it and every sample after it after, so a decoder started there is at `show_us[k]`.
    pub(crate) keys: Vec<usize>,
}

/// Table ticks as microseconds, or `None` when that does not fit.
fn ticks_to_us(ticks: i128, timescale: u32) -> Option<i64> {
    i64::try_from(ticks.checked_mul(i128::from(MICROS_PER_SECOND))? / i128::from(timescale)).ok()
}

/// A table of `(count, value)` runs read one sample at a time: the value of each sample in turn,
/// which is the value of the run it falls in.
struct Runs<'a, T> {
    runs: &'a [(u32, T)],
    at: usize,
    left: u32,
}

impl<'a, T: Copy> Runs<'a, T> {
    fn new(runs: &'a [(u32, T)]) -> Runs<'a, T> {
        Runs { runs, at: 0, left: runs.first().map_or(0, |run| run.0) }
    }

    /// The value of the next sample, or `None` once the table has run out.
    fn next(&mut self) -> Option<T> {
        while self.left == 0 {
            self.at += 1;
            self.left = self.runs.get(self.at)?.0;
        }
        self.left -= 1;
        Some(self.runs.get(self.at)?.1)
    }
}

/// Where every sample of a track is and how long it is, from its three placement tables.
fn place_samples(tables: &StblBox, file_bytes: u64) -> Result<Vec<Sample>, VideoError> {
    let inconsistent = |what: &str| VideoError::Malformed(format!("the sample tables disagree: {what}"));
    let sizes = &tables.stsz;
    let count = if sizes.sample_size == 0 { u32::try_from(sizes.sample_sizes.len()).unwrap_or(u32::MAX) } else { sizes.sample_count };
    if count > SAMPLE_LIMIT {
        return Err(VideoError::Unsupported(format!("the track has {count} samples, past the {SAMPLE_LIMIT} read")));
    }
    let chunk_offsets: Vec<u64> = match (&tables.co64, &tables.stco) {
        (Some(wide), _) => wide.entries.clone(),
        (None, Some(narrow)) => narrow.entries.iter().map(|offset| u64::from(*offset)).collect(),
        (None, None) => return Err(inconsistent("there are no chunk offsets")),
    };

    let mut samples = Vec::with_capacity((count as usize).min(SAMPLE_RESERVE_LIMIT));
    let mut runs = tables.stsc.entries.iter().peekable();
    let mut per_chunk = 0u32;
    for (chunk, start) in chunk_offsets.iter().enumerate() {
        let chunk_number = u32::try_from(chunk + 1).map_err(|_| inconsistent("too many chunks"))?;
        while let Some(run) = runs.next_if(|run| run.first_chunk <= chunk_number) {
            per_chunk = run.samples_per_chunk;
        }
        let mut offset = *start;
        for _ in 0..per_chunk {
            if samples.len() == count as usize {
                break;
            }
            let bytes = if sizes.sample_size == 0 { sizes.sample_sizes[samples.len()] } else { sizes.sample_size };
            let bytes = u64::from(bytes);
            let end = offset.checked_add(bytes).ok_or_else(|| inconsistent("a sample's end overflows"))?;
            if bytes > SAMPLE_BYTES_LIMIT {
                return Err(VideoError::Unsupported(format!("a sample is {bytes} bytes, past the {SAMPLE_BYTES_LIMIT} read")));
            }
            if end > file_bytes {
                return Err(VideoError::Malformed("the file ends before its samples do".into()));
            }
            samples.push(Sample { offset, bytes: bytes as usize });
            offset = end;
        }
        if samples.len() == count as usize {
            break;
        }
    }
    if samples.len() != count as usize {
        return Err(inconsistent("the chunks hold fewer samples than the size table counts"));
    }
    Ok(samples)
}

/// The samples among `marked` -- sample numbers counted from one, as a sync sample table lists
/// them -- that a decoder can be started from with the times after them still right: the ones shown
/// after every sample stored before them and before every sample stored after them. `shown` is
/// when each sample is shown, in the order the samples are stored. The first sample is always
/// answered, marked or not: it is where a movie starts.
///
/// A track with no such table is answered its first sample alone. The format reads a missing
/// table as every sample being one to start from, which a file that merely left the table out
/// would be believed about at the cost of every frame up to its next real one.
fn key_samples(shown: &[i128], marked: &[u32]) -> Vec<usize> {
    let mut marked: Vec<usize> = marked.iter().filter_map(|number| (*number as usize).checked_sub(1)).filter(|sample| *sample < shown.len()).collect();
    marked.sort_unstable();
    marked.dedup();

    let mut after_all_before = Vec::with_capacity(marked.len());
    let mut candidates = marked.iter().copied().peekable();
    let mut latest = i128::MIN;
    for (sample, at) in shown.iter().enumerate() {
        if candidates.next_if_eq(&sample).is_some() && *at > latest {
            after_all_before.push(sample);
        }
        latest = latest.max(*at);
    }

    let mut keys = Vec::with_capacity(after_all_before.len() + 1);
    let mut candidates = after_all_before.iter().rev().copied().peekable();
    let mut earliest = i128::MAX;
    for (sample, at) in shown.iter().enumerate().rev() {
        if candidates.next_if_eq(&sample).is_some() && *at < earliest {
            keys.push(sample);
        }
        earliest = earliest.min(*at);
    }
    if keys.last() != Some(&0) {
        keys.push(0);
    }
    keys.reverse();
    keys
}

/// When the frames of a track are shown and where decoding can start.
struct Timing {
    /// When each frame is shown, in show order, in microseconds.
    show_us: Vec<i64>,
    /// How long the track lasts, in microseconds.
    duration_us: i64,
    keys: Vec<usize>,
}

/// When each frame of a track is shown, how long the track lasts and which samples decoding can
/// start from.
///
/// A sample's decode time is the sum of the durations before it and its show time is that plus its
/// composition offset. The earliest show time is taken as zero, as players take it, so a stream
/// whose first frame is offset by its reordering still starts at the start.
fn show_times(tables: &StblBox, count: usize, timescale: u32) -> Result<Timing, VideoError> {
    let inconsistent = |what: &str| VideoError::Malformed(format!("the timing tables disagree: {what}"));
    if timescale == 0 {
        return Err(inconsistent("the timescale is zero"));
    }
    let durations: Vec<(u32, u32)> = tables.stts.entries.iter().map(|run| (run.sample_count, run.sample_delta)).collect();
    let offsets: Option<Vec<(u32, i32)>> = tables.ctts.as_ref().map(|table| table.entries.iter().map(|run| (run.sample_count, run.sample_offset)).collect());
    let mut durations = Runs::new(&durations);
    let mut offsets = offsets.as_deref().map(Runs::new);

    let mut shown: Vec<i128> = Vec::with_capacity(count);
    let mut decode_at = 0i128;
    for _ in 0..count {
        let duration = durations.next().ok_or_else(|| inconsistent("the duration table is shorter than the track"))?;
        let offset = match offsets.as_mut() {
            Some(offsets) => offsets.next().ok_or_else(|| inconsistent("the composition table is shorter than the track"))?,
            None => 0,
        };
        shown.push(decode_at + i128::from(offset));
        decode_at += i128::from(duration);
    }
    let keys = key_samples(&shown, tables.stss.as_ref().map_or(&[], |table| table.entries.as_slice()));
    let first = shown.iter().copied().min().unwrap_or(0);
    shown.sort_unstable();
    let too_long = || inconsistent("a time does not fit");
    let show_us = shown.iter().map(|ticks| ticks_to_us(ticks - first, timescale)).collect::<Option<Vec<i64>>>().ok_or_else(too_long)?;
    let duration_us = ticks_to_us(decode_at, timescale).ok_or_else(too_long)?;
    Ok(Timing { show_us, duration_us, keys })
}

impl Movie {
    /// Reads the header of the movie file behind `reader`, which is `file_bytes` long.
    pub(crate) fn read<R: Read + Seek>(reader: &mut R, file_bytes: u64) -> Result<Movie, VideoError> {
        let header = read_movie_header(reader, file_bytes)?;
        let track = first_avc_track(&header)?;
        let rebuilt = rebuild_moov(&track);
        let mut cursor = Cursor::new(rebuilt.as_slice());
        cursor.set_position(BOX_HEADER_BYTES as u64);
        let moov = MoovBox::read_box(&mut cursor, rebuilt.len() as u64).map_err(|error| VideoError::Malformed(error.to_string()))?;
        let media = &moov.traks.first().ok_or_else(|| VideoError::Malformed("the rebuilt header lost its track".into()))?.mdia;
        let tables = &media.minf.stbl;
        let StsdBoxContent::Avc1(entry) = &tables.stsd.contents else {
            return Err(VideoError::Unsupported("the video track is not H.264".into()));
        };

        let config = AvcConfig::parse(track.avc_config)?;
        let sequence = config.sps.first().and_then(|set| SequenceInfo::parse(set));
        if sequence.is_some_and(|sequence| !sequence.progressive) {
            return Err(VideoError::Unsupported("the video is interlaced, which is not decoded".into()));
        }
        let (width, height) = sequence.map_or((u32::from(entry.width), u32::from(entry.height)), |sequence| (sequence.width, sequence.height));
        if width == 0 || height == 0 || width.max(height) > FRAME_EDGE_LIMIT {
            return Err(VideoError::Unsupported(format!("a frame is {width}x{height}, which is not decoded")));
        }

        let samples = place_samples(tables, file_bytes)?;
        if samples.is_empty() {
            return Err(VideoError::Malformed("the video track has no samples".into()));
        }
        let Timing { show_us, duration_us, keys } = show_times(tables, samples.len(), media.mdhd.timescale)?;
        if duration_us <= 0 {
            return Err(VideoError::Malformed("the video track has no length".into()));
        }
        let hint = sequence.map_or(ColorHint::default(), |sequence| sequence.color).or(track.color);
        let frame_count = u32::try_from(samples.len()).unwrap_or(u32::MAX);
        Ok(Movie { info: VideoInfo { width, height, frame_count, duration_us }, config, color: ColorSpec::resolve(hint, height), samples, show_us, keys })
    }
}
