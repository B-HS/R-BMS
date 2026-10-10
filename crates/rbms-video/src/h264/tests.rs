//! The decoder against a pack of real movies, when one is named.
//!
//! `RBMS_SKIN_PACK` names a skin pack outside the repository. Every movie under it is decoded
//! front to back and counted, and the ones this was written against are held to the frame counts
//! their own tables give.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use super::*;

/// The environment variable that names a skin pack to decode the movies of.
const PACK_VARIABLE: &str = "RBMS_SKIN_PACK";

/// The movies of the pack this was written against, by their path inside it, and how many frames
/// each has.
const KNOWN_MOVIES: [(&str, u32); 8] = [
    ("Decide/bg/movie/sample.mp4", 600),
    ("Decide/bg/movie/sample2.mp4", 601),
    ("Select/bg/movie/BGmovie01.mp4", 921),
    ("Select/bg/movie/BGmovie02.mp4", 750),
    ("Play/parts/common/BGA/movie/#default.mp4", 2564),
    ("Play/parts/common/BGA/movie/cyber.mp4", 4229),
    ("Play/parts/common/BGA/movie/NOSTALGIC.mp4", 7415),
    ("Play/parts/common/BGA/movie/travel.mp4", 3604),
];

/// How many frames of each movie are timed with their pixels written.
const TIMED_FRAMES: u32 = 300;

/// Every `.mp4` under `folder`, in name order.
fn movies_under(folder: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    let mut entries: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            movies_under(&path, found);
        } else if path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("mp4")) {
            found.push(path);
        }
    }
}

/// Decodes the first [`TIMED_FRAMES`] frames of `decoder` with their pixels, answering the mean and
/// the longest time one frame took and how many frames that was.
fn time_frames(decoder: &mut Mp4H264Decoder) -> (Duration, Duration, u32) {
    let mut pixels = Vec::new();
    let (mut total, mut longest, mut frames) = (Duration::ZERO, Duration::ZERO, 0u32);
    while frames < TIMED_FRAMES {
        let started = Instant::now();
        let decoded = decoder.next_frame(Some(&mut pixels)).expect("the movie decodes");
        let took = started.elapsed();
        if decoded.is_none() {
            break;
        }
        total += took;
        longest = longest.max(took);
        frames += 1;
    }
    (total / frames.max(1), longest, frames)
}

#[test]
fn every_movie_of_an_external_pack_decodes_to_the_frames_its_tables_count() {
    let Some(pack) = std::env::var_os(PACK_VARIABLE).map(PathBuf::from) else {
        eprintln!("skipped: {PACK_VARIABLE} is not set");
        return;
    };
    let mut movies = Vec::new();
    movies_under(&pack, &mut movies);
    assert!(!movies.is_empty(), "{} holds no movie", pack.display());

    for path in &movies {
        let inside = path.strip_prefix(&pack).unwrap_or(path).to_string_lossy().replace('\\', "/");
        let mut decoder = Mp4H264Decoder::open(path).unwrap_or_else(|error| panic!("{inside} could not be opened: {error}"));
        let info = decoder.info();

        let (mean, longest, timed) = time_frames(&mut decoder);
        decoder.rewind().expect("the movie rewinds");

        let started = Instant::now();
        let mut times = Vec::new();
        while let Some(at_us) = decoder.next_frame(None).unwrap_or_else(|error| panic!("{inside} stopped decoding: {error}")) {
            times.push(at_us);
        }
        let whole = started.elapsed();
        let refused = decoder.lost_samples();
        eprintln!(
            "{inside}: {}x{} {:.3} fps {:.2}s, {} frames out of {} counted, {refused} samples refused, color {:?}; \
             decode + convert of {timed} frames mean {mean:?} max {longest:?}; decode of all without pixels {whole:?}",
            info.width,
            info.height,
            f64::from(info.frame_count) * crate::MICROS_PER_SECOND as f64 / info.duration_us as f64,
            info.duration_us as f64 / 1e6,
            times.len(),
            info.frame_count,
            decoder.movie.color,
        );
        assert_eq!(refused, 0, "{inside}: the decoder refused samples");
        assert_eq!(times.len() as u32, info.frame_count, "{inside}: frames out and frames counted differ");
        assert!(times.windows(2).all(|pair| pair[0] <= pair[1]), "{inside}: frames came out with times out of order");
        if let Some((_, frames)) = KNOWN_MOVIES.iter().find(|(known, _)| *known == inside) {
            assert_eq!(info.frame_count, *frames, "{inside}: not the frame count measured for this file");
        }

        decoder.rewind().expect("the movie rewinds");
        let mut first = Vec::new();
        assert_eq!(decoder.next_frame(Some(&mut first)).expect("the first frame decodes again"), Some(times[0]));
        assert_eq!(first.len(), info.frame_bytes());

        let keys = decoder.movie.keys.len();
        let middle_us = info.duration_us / 2;
        assert!(decoder.seek(middle_us).expect("the movie seeks"), "{inside}: no place to start from lies between the start and the middle");
        let started_us = decoder.next_time_us().unwrap_or_else(|| panic!("{inside}: nothing follows the place the decoder went to"));
        assert!((1..=middle_us).contains(&started_us), "{inside}: the decoder went to {started_us} for {middle_us}");
        let mut from_the_start_there = Vec::new();
        assert_eq!(decoder.next_frame(Some(&mut from_the_start_there)).expect("the frame decodes"), Some(started_us));
        let mut rest = vec![started_us];
        while let Some(at_us) = decoder.next_frame(None).unwrap_or_else(|error| panic!("{inside} stopped decoding after a seek: {error}")) {
            rest.push(at_us);
        }
        assert_eq!(decoder.lost_samples(), 0, "{inside}: the decoder refused samples after a seek");
        assert!(times.ends_with(&rest), "{inside}: the frames after a seek are not the frames of a whole pass from {started_us} on, each at its own time");
        assert_eq!(decoder.movie.keys.len(), keys, "{inside}: a sample the file marks as a start was found to be none");

        decoder.rewind().expect("the movie rewinds");
        while decoder.next_time_us().is_some_and(|next_us| next_us < started_us) {
            decoder.next_frame(None).expect("the movie decodes");
        }
        let mut walked_to = Vec::new();
        assert_eq!(decoder.next_frame(Some(&mut walked_to)).expect("the frame decodes"), Some(started_us));
        assert!(walked_to == from_the_start_there, "{inside}: the frame at {started_us} is not the same decoded from there as decoded from the start");
        eprintln!("{inside}: {keys} places to start from; a seek to {middle_us} starts at {started_us} and leaves {} frames", rest.len());
    }
}

mod made;
