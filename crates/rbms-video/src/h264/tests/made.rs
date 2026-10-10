//! The decoder against movies made here: a few flat grey frames encoded with the encoder that
//! ships beside the decoder, wrapped in a movie file written a box at a time. Nothing is read from
//! outside the test, so the same file can also be cut short, overstated and scribbled on.

use std::io::Cursor;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use openh264::encoder::{Encoder, EncoderConfig, RateControlMode};
use openh264::formats::YUVBuffer;

use super::super::*;
use crate::VideoPlayer;
use crate::container::{copy_box, write_box};
use crate::mp4::Movie;

/// The size of the made movie's frames.
const WIDTH: usize = 64;
const HEIGHT: usize = 48;

/// How many frames it has.
const FRAMES: usize = 6;

/// The frame after the first that the encoder is made to start afresh at, so the made movie has a
/// place other than its start that decoding can begin from.
const SECOND_START: usize = 3;

/// Table ticks a second, and ticks each frame is on show.
const TIMESCALE: u32 = 1000;
const FRAME_TICKS: u32 = 100;

/// Microseconds each frame is on show.
const FRAME_US: i64 = 100_000;

/// The luma of the first frame and how much brighter each frame after it is.
const FIRST_LUMA: u8 = 40;
const LUMA_STEP: u8 = 30;

/// The value of a chroma sample that carries no colour.
const NEUTRAL_CHROMA: u8 = 128;

/// How far a decoded grey may sit from the grey that was encoded: the encoder is lossy.
const GREY_TOLERANCE: i32 = 8;

/// Bytes of the NAL length the made movie's samples use.
const LENGTH_BYTES: usize = 4;

/// Which bits of a NAL unit's first byte are its type, and the types of the two parameter sets.
const NAL_TYPE_MASK: u8 = 0x1f;
const NAL_SPS: u8 = 7;
const NAL_PPS: u8 = 8;

/// How long a test waits on the player's thread.
const PATIENCE: Duration = Duration::from_secs(30);

/// The grey the encoder was given for frame `index`.
fn luma_of(index: usize) -> u8 {
    FIRST_LUMA + LUMA_STEP * index as u8
}

/// The RGB grey a studio range luma converts to.
fn grey_of(luma: u8) -> i32 {
    ((i32::from(luma) - 16) * 255 + 109) / 219
}

/// The NAL units of an encoded access unit, without their start codes.
fn split_units(stream: &[u8]) -> Vec<&[u8]> {
    let mut starts = Vec::new();
    let mut at = 0;
    while at + 3 <= stream.len() {
        if stream[at..at + 3] == [0, 0, 1] {
            starts.push(at + 3);
            at += 3;
        } else {
            at += 1;
        }
    }
    let ends = starts.iter().skip(1).map(|next| next - 3).chain([stream.len()]);
    starts.iter().zip(ends).map(|(start, end)| stream[*start..end].strip_suffix(&[0]).unwrap_or(&stream[*start..end])).collect()
}

/// An encoded stream as a movie file keeps it: the parameter sets apart, and one sample a frame.
#[derive(Debug, Clone)]
struct Encoded {
    sps: Vec<u8>,
    pps: Vec<u8>,
    samples: Vec<Vec<u8>>,
}

/// [`FRAMES`] flat grey frames, each brighter than the one before, encoded once for every test. The
/// first frame and frame [`SECOND_START`] lean on no frame before them.
fn encoded() -> &'static Encoded {
    static ENCODED: OnceLock<Encoded> = OnceLock::new();
    ENCODED.get_or_init(|| {
        let config = EncoderConfig::new().skip_frames(false).rate_control_mode(RateControlMode::Off);
        let mut encoder = Encoder::with_api_config(OpenH264API::from_source(), config).expect("an encoder is made");
        let (mut sps, mut pps, mut samples) = (Vec::new(), Vec::new(), Vec::new());
        for index in 0..FRAMES {
            if index == SECOND_START {
                encoder.force_intra_frame();
            }
            let mut planes = vec![luma_of(index); WIDTH * HEIGHT];
            planes.resize(WIDTH * HEIGHT * 3 / 2, NEUTRAL_CHROMA);
            let stream = encoder.encode(&YUVBuffer::from_vec(planes, WIDTH, HEIGHT)).expect("a frame encodes").to_vec();
            let mut sample = Vec::new();
            for unit in split_units(&stream) {
                match unit[0] & NAL_TYPE_MASK {
                    NAL_SPS => sps = unit.to_vec(),
                    NAL_PPS => pps = unit.to_vec(),
                    _ => {
                        sample.extend_from_slice(&(unit.len() as u32).to_be_bytes());
                        sample.extend_from_slice(unit);
                    }
                }
            }
            assert!(!sample.is_empty(), "frame {index} encoded to no slice");
            samples.push(sample);
        }
        assert!(!sps.is_empty() && !pps.is_empty(), "the encoder wrote no parameter sets");
        Encoded { sps, pps, samples }
    })
}

/// The tables of a movie file about to be written, each open to being made wrong.
#[derive(Debug, Clone)]
struct Recipe {
    /// Whether the movie header is written ahead of the media data rather than behind it.
    header_first: bool,
    timescale: u32,
    handler: [u8; 4],
    entry_kind: [u8; 4],
    /// The decoder configuration record.
    avc_config: Vec<u8>,
    /// The colour box nested in the sample entry, when there is one.
    colour: Option<Vec<u8>>,
    /// Runs of `(samples, ticks each)`.
    durations: Vec<(u32, u32)>,
    /// Runs of `(samples, composition offset)`.
    offsets: Option<Vec<(u32, i32)>>,
    /// The numbers of the samples decoding can start from, counted from one, when the file says.
    starts: Option<Vec<u32>>,
    /// Runs of `(first chunk, samples in each chunk)`.
    chunks: Vec<(u32, u32)>,
    /// The size every sample has, or zero to say them one by one.
    fixed_size: u32,
    sample_count: u32,
    sizes: Vec<u32>,
    /// Where each chunk starts, measured from the start of the media data.
    chunk_starts: Vec<u32>,
    /// A box to nest in the movie header ahead of the track, as its whole bytes.
    stray: Vec<u8>,
}

impl Recipe {
    /// The tables that describe `encoded` truthfully: one chunk, every frame as long as the next.
    fn of(encoded: &Encoded) -> Recipe {
        let mut avc_config = vec![1, encoded.sps[1], encoded.sps[2], encoded.sps[3], 0xfc | (LENGTH_BYTES as u8 - 1), 0xe1];
        avc_config.extend_from_slice(&(encoded.sps.len() as u16).to_be_bytes());
        avc_config.extend_from_slice(&encoded.sps);
        avc_config.push(1);
        avc_config.extend_from_slice(&(encoded.pps.len() as u16).to_be_bytes());
        avc_config.extend_from_slice(&encoded.pps);
        let count = encoded.samples.len() as u32;
        Recipe {
            header_first: false,
            timescale: TIMESCALE,
            handler: *b"vide",
            entry_kind: *b"avc1",
            avc_config,
            colour: None,
            durations: vec![(count, FRAME_TICKS)],
            offsets: None,
            starts: Some(vec![1, SECOND_START as u32 + 1]),
            chunks: vec![(1, count)],
            fixed_size: 0,
            sample_count: count,
            sizes: encoded.samples.iter().map(|sample| sample.len() as u32).collect(),
            chunk_starts: vec![0],
            stray: Vec::new(),
        }
    }

    /// The movie header, with its chunk offsets measured from `media_at`.
    fn header(&self, media_at: u32) -> Vec<u8> {
        let words = |out: &mut Vec<u8>, words: &[u32]| words.iter().for_each(|word| out.extend_from_slice(&word.to_be_bytes()));
        let table = |out: &mut Vec<u8>, kind: &[u8; 4], rows: usize, fill: &dyn Fn(&mut Vec<u8>)| {
            write_box(out, *kind, |out| {
                words(out, &[0, rows as u32]);
                fill(out);
            });
        };
        let mut out = Vec::new();
        write_box(&mut out, *b"moov", |out| {
            copy_box(out, *b"mvhd", &[&[0; 12][..], &self.timescale.to_be_bytes(), &[0; 84]].concat());
            out.extend_from_slice(&self.stray);
            write_box(out, *b"trak", |out| {
                copy_box(out, *b"tkhd", &[&[0; 12][..], &1u32.to_be_bytes(), &[0; 68]].concat());
                write_box(out, *b"mdia", |out| {
                    copy_box(out, *b"mdhd", &[&[0; 12][..], &self.timescale.to_be_bytes(), &[0; 8]].concat());
                    copy_box(out, *b"hdlr", &[&[0; 8][..], &self.handler, &[0; 13]].concat());
                    write_box(out, *b"minf", |out| {
                        write_box(out, *b"stbl", |out| {
                            write_box(out, *b"stsd", |out| {
                                words(out, &[0, 1]);
                                write_box(out, self.entry_kind, |out| {
                                    out.extend_from_slice(&[0; 24]);
                                    out.extend_from_slice(&(WIDTH as u16).to_be_bytes());
                                    out.extend_from_slice(&(HEIGHT as u16).to_be_bytes());
                                    out.extend_from_slice(&[0; 50]);
                                    if let Some(colour) = &self.colour {
                                        copy_box(out, *b"colr", colour);
                                    }
                                    copy_box(out, *b"avcC", &self.avc_config);
                                });
                            });
                            table(out, b"stts", self.durations.len(), &|out| self.durations.iter().for_each(|(count, ticks)| words(out, &[*count, *ticks])));
                            if let Some(offsets) = &self.offsets {
                                table(out, b"ctts", offsets.len(), &|out| offsets.iter().for_each(|(count, offset)| words(out, &[*count, *offset as u32])));
                            }
                            if let Some(starts) = &self.starts {
                                table(out, b"stss", starts.len(), &|out| words(out, starts));
                            }
                            table(out, b"stsc", self.chunks.len(), &|out| self.chunks.iter().for_each(|(first, samples)| words(out, &[*first, *samples, 1])));
                            write_box(out, *b"stsz", |out| {
                                words(out, &[0, self.fixed_size, self.sample_count]);
                                words(out, &self.sizes);
                            });
                            table(out, b"stco", self.chunk_starts.len(), &|out| {
                                self.chunk_starts.iter().for_each(|start| words(out, &[media_at.wrapping_add(*start)]))
                            });
                        });
                    });
                });
            });
        });
        out
    }

    /// The whole file: a file type box, the media data and the movie header, in the order asked.
    fn file(&self, encoded: &Encoded) -> Vec<u8> {
        let mut file_type = Vec::new();
        copy_box(&mut file_type, *b"ftyp", b"isom\0\0\0\0isom");
        let mut media = Vec::new();
        copy_box(&mut media, *b"mdat", &encoded.samples.concat());
        let header_bytes = self.header(0).len();
        let before_media = file_type.len() + if self.header_first { header_bytes } else { 0 };
        let header = self.header((before_media + crate::container::BOX_HEADER_BYTES) as u32);
        if self.header_first { [file_type, header, media].concat() } else { [file_type, media, header].concat() }
    }
}

/// The made movie as its tables say it truthfully.
fn made_file() -> Vec<u8> {
    Recipe::of(encoded()).file(encoded())
}

/// Reads the header of a movie file held in memory.
fn read(file: &[u8]) -> Result<Movie, VideoError> {
    Movie::read(&mut Cursor::new(file), file.len() as u64)
}

/// A file of this test's own in the system's temporary folder, removed when it is dropped.
struct TempMovie(PathBuf);

impl TempMovie {
    fn write(bytes: &[u8]) -> TempMovie {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let name = format!("rbms-video-{}-{}.mp4", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed));
        let path = std::env::temp_dir().join(name);
        std::fs::write(&path, bytes).expect("the temporary folder is writable");
        TempMovie(path)
    }
}

impl Drop for TempMovie {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Opens a movie file held in memory and decodes all of it, answering each frame's time and its
/// first pixel. `Err` when it will not open or stops decoding; never a panic.
fn decode_all(file: &[u8]) -> Result<Vec<(i64, [u8; 4])>, VideoError> {
    let on_disk = TempMovie::write(file);
    let mut decoder = Mp4H264Decoder::open(&on_disk.0)?;
    let mut frames = Vec::new();
    let mut pixels = Vec::new();
    while let Some(at_us) = decoder.next_frame(Some(&mut pixels))? {
        frames.push((at_us, [pixels[0], pixels[1], pixels[2], pixels[3]]));
        assert!(frames.len() <= 4 * FRAMES, "a movie of {FRAMES} frames went on past {}", 4 * FRAMES);
    }
    Ok(frames)
}

#[test]
fn a_made_movie_says_its_size_its_length_and_where_its_samples_are() {
    let file = made_file();
    let movie = read(&file).expect("the made movie reads");
    assert_eq!(movie.info, VideoInfo { width: WIDTH as u32, height: HEIGHT as u32, frame_count: FRAMES as u32, duration_us: FRAMES as i64 * FRAME_US });
    assert_eq!(movie.show_us, (0..FRAMES as i64).map(|frame| frame * FRAME_US).collect::<Vec<_>>());
    assert_eq!(movie.config.length_size, LENGTH_BYTES);
    assert_eq!((movie.config.sps.len(), movie.config.pps.len()), (1, 1));
    for (sample, wanted) in movie.samples.iter().zip(&encoded().samples) {
        assert_eq!(&file[sample.offset as usize..][..sample.bytes], wanted.as_slice(), "a sample is not where the tables put it");
    }
    assert_eq!(movie.color, ColorSpec::resolve(Default::default(), HEIGHT as u32), "a small unlabelled movie is read as standard definition");
}

#[test]
fn a_made_movie_decodes_to_its_frames_in_order_wherever_its_header_is() {
    for header_first in [false, true] {
        let recipe = Recipe { header_first, ..Recipe::of(encoded()) };
        let frames = decode_all(&recipe.file(encoded())).expect("the made movie decodes");
        assert_eq!(frames.len(), FRAMES);
        for (index, (at_us, pixel)) in frames.iter().enumerate() {
            assert_eq!(*at_us, index as i64 * FRAME_US);
            let wanted = grey_of(luma_of(index));
            for channel in &pixel[..3] {
                assert!((i32::from(*channel) - wanted).abs() <= GREY_TOLERANCE, "frame {index} came out {pixel:?}, wanted a grey of {wanted}");
            }
            assert_eq!(pixel[3], u8::MAX);
        }
    }
}

#[test]
fn a_rewound_movie_decodes_the_same_frames_again() {
    let on_disk = TempMovie::write(&made_file());
    let mut decoder = Mp4H264Decoder::open(&on_disk.0).expect("the made movie opens");
    let pass = |decoder: &mut Mp4H264Decoder| {
        let mut frames = Vec::new();
        let mut pixels = Vec::new();
        loop {
            let due = decoder.next_time_us();
            let Some(at_us) = decoder.next_frame(Some(&mut pixels)).expect("the made movie decodes") else {
                assert_eq!(due, None, "a time was promised for a frame that never came");
                break;
            };
            assert_eq!(due, Some(at_us), "the frame came at another time than it was promised for");
            assert_eq!(pixels.len(), decoder.info().frame_bytes());
            frames.push((at_us, pixels.clone()));
        }
        frames
    };
    let first = pass(&mut decoder);
    assert_eq!(first.len(), FRAMES);
    assert!(matches!(decoder.next_frame(None), Ok(None)), "a movie at its end had another frame");
    decoder.rewind().expect("the movie rewinds");
    assert_eq!(pass(&mut decoder), first, "the second pass is not the first");

    decoder.rewind().expect("the movie rewinds");
    assert_eq!(decoder.next_frame(None).expect("a frame decodes without its pixels"), Some(0));
    decoder.rewind().expect("the movie rewinds from its middle");
    assert_eq!(pass(&mut decoder), first, "a pass begun after a rewind from the middle is not the first");
    assert_eq!(decoder.lost_samples(), 0);
}

/// Decodes what is left of `decoder`'s pass, answering which frame each one out is -- by its grey --
/// and when it is shown.
fn rest_of_pass(decoder: &mut Mp4H264Decoder) -> Vec<(usize, i64)> {
    let mut frames = Vec::new();
    let mut pixels = Vec::new();
    while let Some(at_us) = decoder.next_frame(Some(&mut pixels)).expect("the made movie decodes") {
        let grey = i32::from(pixels[0]);
        let index = (0..FRAMES).find(|index| (grey - grey_of(luma_of(*index))).abs() <= GREY_TOLERANCE);
        frames.push((index.unwrap_or_else(|| panic!("a frame came out a grey of {grey}, which no frame was made")), at_us));
        assert!(frames.len() <= FRAMES, "a movie of {FRAMES} frames went on past them");
    }
    frames
}

/// Each of the frames `from..FRAMES` at its own time.
fn frames_from(from: usize) -> Vec<(usize, i64)> {
    (from..FRAMES).map(|index| (index, index as i64 * FRAME_US)).collect()
}

#[test]
fn the_samples_the_file_marks_are_where_decoding_can_start() {
    let movie = read(&made_file()).expect("the made movie reads");
    assert_eq!(movie.keys, [0, SECOND_START]);
    let stream = AnnexB::new(movie.config.clone());
    for (index, sample) in encoded().samples.iter().enumerate() {
        assert_eq!(stream.starts_afresh(sample), movie.keys.contains(&index), "sample {index}");
    }

    let unmarked = Recipe { starts: None, ..Recipe::of(encoded()) };
    assert_eq!(read(&unmarked.file(encoded())).expect("the made movie reads").keys, [0], "a file that marks nothing was believed to mark everything");
    let badly_marked = Recipe { starts: Some(vec![0, 5, 5, u32::MAX, 2, 99]), ..Recipe::of(encoded()) };
    assert_eq!(read(&badly_marked.file(encoded())).expect("the made movie reads").keys, [0, 1, 4]);
    let reordered = Recipe { starts: Some(vec![1, 2, 3, 4, 5, 6]), offsets: Some(vec![(1, 0), (1, 300), (2, 0), (2, 100)]), ..Recipe::of(encoded()) };
    assert_eq!(
        read(&reordered.file(encoded())).expect("the made movie reads").keys,
        [0, 4, 5],
        "a sample shown after one stored behind it was taken for a place to start"
    );
}

#[test]
fn a_decoder_goes_to_the_start_before_a_time_and_its_frames_keep_their_own_times() {
    let on_disk = TempMovie::write(&made_file());
    let mut decoder = Mp4H264Decoder::open(&on_disk.0).expect("the made movie opens");
    let at = |frame: usize| frame as i64 * FRAME_US;

    assert!(!decoder.seek(at(2)).expect("the movie seeks"), "a decoder at the start was moved for a time the start is the start before");
    assert!(decoder.seek(at(4) + 1).expect("the movie seeks"), "a decoder before a later start was not moved to it");
    assert_eq!(decoder.next_time_us(), Some(at(SECOND_START)));
    assert_eq!(rest_of_pass(&mut decoder), frames_from(SECOND_START));

    assert!(decoder.seek(at(5)).expect("the movie seeks"), "a decoder at its end was not moved");
    assert_eq!(decoder.next_frame(None).expect("a frame decodes"), Some(at(SECOND_START)));
    assert!(!decoder.seek(at(5)).expect("the movie seeks"), "a decoder between the start and the time was moved");
    assert!(decoder.seek(at(1)).expect("the movie seeks"), "a decoder past the time was not moved back");
    assert_eq!(rest_of_pass(&mut decoder), frames_from(0));

    decoder.rewind().expect("the movie rewinds");
    assert_eq!(decoder.next_frame(None).expect("a frame decodes"), Some(0));
    assert!(decoder.seek(at(FRAMES) + at(FRAMES)).expect("the movie seeks"), "a time past the end is not reached from the last start");
    assert_eq!(rest_of_pass(&mut decoder), frames_from(SECOND_START));
    assert_eq!(decoder.lost_samples(), 0);
}

#[test]
fn a_sample_marked_as_a_start_that_is_none_is_not_started_from() {
    let lying = Recipe { starts: Some(vec![1, 2, 3, SECOND_START as u32 + 1, 6]), ..Recipe::of(encoded()) };
    let on_disk = TempMovie::write(&lying.file(encoded()));
    let mut decoder = Mp4H264Decoder::open(&on_disk.0).expect("the made movie opens");
    assert_eq!(decoder.movie.keys, [0, 1, 2, SECOND_START, 5]);

    assert!(!decoder.seek(2 * FRAME_US).expect("the movie seeks"), "a decoder at the start was moved to a sample that leans on the ones before it");
    assert_eq!(decoder.movie.keys, [0, SECOND_START, 5], "the samples found to be no start are still listed as starts");
    assert_eq!(rest_of_pass(&mut decoder), frames_from(0));

    assert!(decoder.seek(5 * FRAME_US).expect("the movie seeks"));
    assert_eq!(decoder.movie.keys, [0, SECOND_START]);
    assert_eq!(rest_of_pass(&mut decoder), frames_from(SECOND_START), "the start before a sample that was no start was not gone to");
    assert_eq!(decoder.lost_samples(), 0, "a sample that is no start was fed to a decoder that had seen nothing");
}

#[test]
fn the_frames_after_a_damaged_stretch_come_out_at_their_own_times() {
    let file = Recipe { header_first: true, ..Recipe::of(encoded()) }.file(encoded());
    let media_at = file.len() - encoded().samples.concat().len();
    let damaged_sample = 1;
    let sample_at = media_at + encoded().samples[..damaged_sample].iter().map(Vec::len).sum::<usize>();
    let mut damaged = file;
    for (index, byte) in damaged[sample_at + LENGTH_BYTES + 1..sample_at + encoded().samples[damaged_sample].len()].iter_mut().enumerate() {
        *byte = (index * 31 % 251) as u8;
    }

    let on_disk = TempMovie::write(&damaged);
    let mut decoder = Mp4H264Decoder::open(&on_disk.0).expect("the damaged movie opens");
    let frames = rest_of_pass(&mut decoder);
    assert!(decoder.lost_samples() > 0, "the damage went unnoticed, so this says nothing about what follows it");
    for (index, at_us) in &frames {
        assert_eq!(*at_us, *index as i64 * FRAME_US, "frame {index} came out at the time of another: {frames:?}");
    }
    assert!(frames.ends_with(&frames_from(SECOND_START)), "the frames after the damage did not all come out: {frames:?}");

    let mut player = VideoPlayer::spawn(crate::open(&on_disk.0).expect("the damaged movie opens")).expect("a thread starts");
    player.advance_blocking(0, PATIENCE);
    assert_eq!(player.frame().map(|frame| frame.time_us), Some(0));
    player.advance_blocking(SECOND_START as i64 * FRAME_US - 1, PATIENCE);
    assert_eq!(player.frame().map(|frame| frame.time_us), Some(0), "something was shown in the time of the frames that were lost");
    player.advance_blocking(SECOND_START as i64 * FRAME_US, PATIENCE);
    assert_eq!(player.frame().map(|frame| frame.time_us), Some(SECOND_START as i64 * FRAME_US));
    assert!(player.lost_samples() > 0, "a player was not told of the damage its decoder found");
    assert_eq!(player.failure(), None);
}

#[test]
fn a_made_movie_plays_round_and_round() {
    let on_disk = TempMovie::write(&made_file());
    let mut player = VideoPlayer::spawn(crate::open(&on_disk.0).expect("the made movie opens")).expect("a thread starts");
    let pass_us = FRAMES as i64 * FRAME_US;
    for (time_us, frame) in [(0, 0), (FRAME_US + 1, 1), (pass_us - 1, FRAMES - 1), (pass_us, 0), (2 * pass_us + 3 * FRAME_US, 3)] {
        player.advance_blocking(time_us, PATIENCE);
        let shown = player.frame().unwrap_or_else(|| panic!("nothing is on show at {time_us}"));
        assert_eq!(shown.time_us % pass_us, frame as i64 * FRAME_US, "the wrong frame is on show at {time_us}");
        assert!((i32::from(shown.rgba[0]) - grey_of(luma_of(frame))).abs() <= GREY_TOLERANCE, "frame {frame} is not on show at {time_us}");
    }
    assert_eq!(player.failure(), None);
}

#[test]
fn frames_stored_out_of_show_order_are_timed_in_show_order() {
    let reordered = Recipe { offsets: Some(vec![(1, 100), (1, 300), (2, 0), (2, 100)]), ..Recipe::of(encoded()) };
    let movie = read(&reordered.file(encoded())).expect("the made movie reads");
    assert_eq!(movie.show_us, (0..FRAMES as i64).map(|frame| frame * FRAME_US).collect::<Vec<_>>(), "the first frame shown is not at zero");
    let stretched = Recipe { durations: vec![(2, 100), (4, 50)], ..Recipe::of(encoded()) };
    let movie = read(&stretched.file(encoded())).expect("the made movie reads");
    assert_eq!(movie.show_us, [0, 100_000, 200_000, 250_000, 300_000, 350_000]);
    assert_eq!(movie.info.duration_us, 400_000);
}

#[test]
fn samples_split_over_chunks_are_found_where_each_chunk_starts() {
    let sizes: Vec<u32> = encoded().samples.iter().map(|sample| sample.len() as u32).collect();
    let after = |samples: usize| sizes[..samples].iter().sum::<u32>();
    let split = Recipe { chunks: vec![(1, 1), (2, 2), (3, 3)], chunk_starts: vec![0, after(1), after(3)], ..Recipe::of(encoded()) };
    assert_eq!(decode_all(&split.file(encoded())).expect("the split movie decodes").len(), FRAMES);
}

#[test]
fn what_the_file_says_about_its_colour_is_kept_when_the_stream_says_nothing() {
    let labelled = Recipe { colour: Some([&b"nclx"[..], &[0, 1, 0, 1, 0, 1, 0x80]].concat()), ..Recipe::of(encoded()) };
    let movie = read(&labelled.file(encoded())).expect("the made movie reads");
    assert_eq!(movie.color.matrix, crate::color::ColorMatrix::Bt709);
    assert_eq!(movie.color.range, crate::color::ColorRange::Full);
}

#[test]
fn the_other_name_of_an_h264_entry_is_read_and_other_tracks_are_not() {
    let other_name = Recipe { entry_kind: *b"avc3", ..Recipe::of(encoded()) };
    assert_eq!(decode_all(&other_name.file(encoded())).expect("an avc3 entry decodes").len(), FRAMES);
    let sound = Recipe { handler: *b"soun", ..Recipe::of(encoded()) };
    assert!(matches!(read(&sound.file(encoded())), Err(VideoError::Unsupported(_))));
    let other_codec = Recipe { entry_kind: *b"hvc1", ..Recipe::of(encoded()) };
    assert!(matches!(read(&other_codec.file(encoded())), Err(VideoError::Unsupported(_))));
}

#[test]
fn tables_that_contradict_each_other_or_the_file_are_errors() {
    let true_recipe = Recipe::of(encoded());
    let wrong: Vec<(&str, Recipe)> = vec![
        ("no duration runs", Recipe { durations: Vec::new(), ..true_recipe.clone() }),
        ("fewer durations than samples", Recipe { durations: vec![(2, FRAME_TICKS)], ..true_recipe.clone() }),
        ("fewer offsets than samples", Recipe { offsets: Some(vec![(1, 0)]), ..true_recipe.clone() }),
        ("no chunk runs", Recipe { chunks: Vec::new(), ..true_recipe.clone() }),
        ("fewer samples in chunks than sizes", Recipe { chunks: vec![(1, 2)], ..true_recipe.clone() }),
        ("no chunks", Recipe { chunk_starts: Vec::new(), ..true_recipe.clone() }),
        ("a chunk past the end of the file", Recipe { chunk_starts: vec![u32::MAX / 2], ..true_recipe.clone() }),
        ("a timescale of zero", Recipe { timescale: 0, ..true_recipe.clone() }),
        ("durations of zero", Recipe { durations: vec![(FRAMES as u32, 0)], ..true_recipe.clone() }),
        ("no samples", Recipe { sample_count: 0, sizes: Vec::new(), ..true_recipe.clone() }),
        ("a sample longer than the file", Recipe { sizes: vec![u32::MAX; FRAMES], ..true_recipe.clone() }),
        ("a fixed size no sample has room for", Recipe { fixed_size: 1 << 20, sizes: Vec::new(), ..true_recipe.clone() }),
        ("more samples than a file could hold", Recipe { fixed_size: 1, sample_count: u32::MAX, sizes: Vec::new(), ..true_recipe.clone() }),
        ("more samples than the size table lists", Recipe { sample_count: u32::MAX, ..true_recipe.clone() }),
        ("a parameter set past the end of its record", Recipe { avc_config: vec![1, 100, 0, 30, 0xff, 0xe1, 0xff, 0xff, 0x67], ..true_recipe.clone() }),
        ("an empty decoder configuration", Recipe { avc_config: Vec::new(), ..true_recipe.clone() }),
    ];
    for (what, recipe) in wrong {
        assert!(read(&recipe.file(encoded())).is_err(), "{what}: the movie was read");
    }
    let run_of_everything = Recipe { chunks: vec![(1, u32::MAX)], ..true_recipe };
    assert_eq!(read(&run_of_everything.file(encoded())).expect("a chunk run longer than the track is cut at the track's end").samples.len(), FRAMES);
}

#[test]
fn boxes_with_absurd_lengths_inside_the_header_are_errors_and_never_a_wait() {
    let length_of = |length: u32, kind: &[u8; 4]| [&length.to_be_bytes()[..], kind].concat();
    let wide = |length: u64, kind: &[u8; 4]| [&1u32.to_be_bytes()[..], kind, &length.to_be_bytes()[..]].concat();
    let strays: Vec<(&str, Vec<u8>, bool)> = vec![
        ("an unknown box of a sane length is stepped over", [length_of(12, b"junk"), vec![1, 2, 3, 4]].concat(), true),
        ("a box longer than the header", length_of(u32::MAX, b"junk"), false),
        ("a box shorter than its own header", length_of(4, b"junk"), false),
        ("a 64-bit length of nearly everything", wide(u64::MAX, b"junk"), false),
        ("a 64-bit length shorter than its header", wide(9, b"junk"), false),
        ("a track box that is cut short", length_of(8, b"trak"), false),
        ("user data with a child longer than it", [length_of(16, b"udta"), length_of(u32::MAX, b"meta")].concat(), true),
    ];
    for (what, stray, reads) in strays {
        let file = Recipe { stray, ..Recipe::of(encoded()) }.file(encoded());
        assert_eq!(read(&file).is_ok(), reads, "{what}");
    }
    let to_the_end = Recipe { stray: length_of(0, b"junk"), ..Recipe::of(encoded()) }.file(encoded());
    assert!(read(&to_the_end).is_err(), "a box of no length swallows the track behind it, and the header has no track left");
}

#[test]
fn a_file_cut_short_anywhere_is_an_error_and_never_a_panic() {
    for header_first in [false, true] {
        let file = Recipe { header_first, ..Recipe::of(encoded()) }.file(encoded());
        for end in 0..file.len() {
            assert!(read(&file[..end]).is_err(), "a file cut to {end} of {} bytes was read", file.len());
        }
        assert!(read(&file).is_ok());
    }
}

#[test]
fn a_header_with_any_byte_scribbled_on_reads_or_fails_and_never_panics() {
    let recipe = Recipe { header_first: true, ..Recipe::of(encoded()) };
    let file = recipe.file(encoded());
    let header_end = file.len() - encoded().samples.concat().len() - crate::container::BOX_HEADER_BYTES;
    let mut read_anyway = 0usize;
    for at in 0..header_end {
        for scribble in [0x00u8, 0xff, 0x80] {
            let mut damaged = file.clone();
            if damaged[at] == scribble {
                continue;
            }
            damaged[at] = scribble;
            if let Ok(movie) = read(&damaged) {
                read_anyway += 1;
                assert!(
                    movie.samples.iter().all(|sample| sample.offset + sample.bytes as u64 <= damaged.len() as u64),
                    "byte {at}: a sample lies outside the file"
                );
                assert!(movie.info.width > 0 && movie.info.height > 0 && movie.info.duration_us > 0, "byte {at}: {:?}", movie.info);
            }
        }
    }
    assert!(read_anyway > 0, "no scribble left a header that still read, so nothing after the read was exercised");
}

#[test]
fn damaged_media_data_is_skipped_or_refused_without_a_panic() {
    let file = Recipe { header_first: true, ..Recipe::of(encoded()) }.file(encoded());
    let media_at = file.len() - encoded().samples.concat().len();
    let first_sample = encoded().samples[0].len();

    let mut noise_after_the_first = file.clone();
    for (index, byte) in noise_after_the_first[media_at + first_sample..].iter_mut().enumerate() {
        *byte = (index * 31 % 251) as u8;
    }
    if let Ok(frames) = decode_all(&noise_after_the_first) {
        assert!(frames.len() <= FRAMES);
    }

    let mut lengths_overstated = file.clone();
    for sample in 0..FRAMES {
        let at = media_at + encoded().samples[..sample].iter().map(Vec::len).sum::<usize>();
        lengths_overstated[at..at + LENGTH_BYTES].copy_from_slice(&u32::MAX.to_be_bytes());
    }
    let frames = decode_all(&lengths_overstated).expect("samples that convert to nothing are fed as nothing");
    assert!(frames.is_empty(), "frames came out of samples whose every length runs past them");

    let mut all_noise = file;
    all_noise[media_at..].iter_mut().enumerate().for_each(|(index, byte)| *byte = (index * 7 % 256) as u8);
    if let Ok(frames) = decode_all(&all_noise) {
        assert!(frames.len() <= FRAMES);
    }
}

#[test]
fn a_file_that_is_no_movie_is_refused() {
    for bytes in [&b""[..], b"not a movie at all", &[0u8; 64], &[0xffu8; 64]] {
        let on_disk = TempMovie::write(bytes);
        assert!(crate::open(&on_disk.0).is_err());
    }
}
