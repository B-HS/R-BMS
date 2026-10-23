//! The pictures of the bars near the cursor: which bars are read, how a picture becomes a square,
//! that the worker only works through the newest list it was given, that the squares used longest
//! ago are let go of, and that a frame shows a square on each bar whose picture has been read.

use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::time::Duration;

use rbms_model::Mode;
use rbms_render::skin_render::frame::BarKind;
use rbms_render::{CpuCanvas, Renderer};

use super::{BARS_ABOVE, BARS_BELOW, CACHE_LIMIT, Pictures, Reply, SongThumbs, THUMB_SIDE, make_squares, near, square_of};
use crate::{BarChart, BarImages, DecodedImage, SelectBar};

/// Longest a test waits for the worker to answer one picture.
const ANSWER_WAIT: Duration = Duration::from_secs(20);

/// The size of the canvas the squares are registered with.
const CANVAS_SIDE: u32 = 16;

/// A scratch folder for the pictures one test writes, removed when the test ends.
struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new(tag: &str) -> Scratch {
        let root = std::env::temp_dir().join(format!("rbms-select-thumbs-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("the scratch folder is writable");
        Scratch { root }
    }

    /// Writes a picture of one flat colour and answers where it is.
    fn picture(&self, name: &str, size: (u32, u32), rgba: [u8; 4]) -> PathBuf {
        let file = self.root.join(name);
        image::RgbaImage::from_pixel(size.0, size.1, image::Rgba(rgba)).save(&file).expect("the picture is written");
        file
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// A chart's bar naming `stagefile` and `banner`.
fn chart_bar(stagefile: Option<&Path>, banner: Option<&Path>) -> SelectBar {
    SelectBar {
        chart: Some(BarChart {
            index: 0,
            subtitle: String::new(),
            mode: Mode::BEAT_7K,
            level_text: String::new(),
            level: 0,
            difficulty: 0,
            features: 0,
            added_at: None,
            favorite: false,
            best: None,
            images: BarImages { stagefile: stagefile.map(Path::to_path_buf), banner: banner.map(Path::to_path_buf), backbmp: None },
        }),
        ..SelectBar::new(BarKind::Song { exists: true }, "CHART")
    }
}

/// The files of a bar that names only `file`, as its stage file.
fn stagefile_only(file: &str) -> Pictures {
    Pictures { stagefile: Some(PathBuf::from(file)), banner: None }
}

#[test]
fn the_bars_read_are_the_nearest_first_round_the_ends_of_the_list_and_each_once() {
    assert!(near(0, 0).is_empty(), "an empty list reads nothing");
    assert_eq!(near(1, 0), [0]);
    assert_eq!(near(3, 1), [1, 2, 0], "a short list names each bar once");
    assert_eq!(&near(100, 0)[..5], [0, 1, 99, 2, 98], "the cursor, then one below and one above, going round the ends");
    let many = near(1000, 500);
    assert_eq!(many.len(), BARS_ABOVE + 1 + BARS_BELOW);
    assert_eq!((many.iter().min(), many.iter().max()), (Some(&(500 - BARS_ABOVE)), Some(&(500 + BARS_BELOW))));
}

#[test]
fn a_picture_becomes_its_middle_square_at_the_size_of_a_bars_picture() {
    let (width, height) = (6, 2);
    let mut rgba = Vec::new();
    for _ in 0..height {
        for x in 0..width {
            rgba.extend_from_slice(&if (2..4).contains(&x) { [0, 255, 0, 255] } else { [255, 0, 0, 255] });
        }
    }
    let square = square_of(DecodedImage::for_test(rgba, width, height)).expect("a picture with pixels makes a square");
    assert_eq!(square.len(), (THUMB_SIDE * THUMB_SIDE * 4) as usize);
    assert!(square.chunks(4).all(|pixel| pixel == [0, 255, 0, 255]), "only the middle of a wide picture is kept");
    assert_eq!(square_of(DecodedImage::for_test(Vec::new(), 0, 0)), None, "a picture with no pixels makes none");
}

#[test]
fn the_worker_works_through_the_newest_list_it_was_given_and_skips_the_ones_it_passed() {
    let scratch = Scratch::new("newest");
    let first = scratch.picture("first.png", (4, 4), [255, 0, 0, 255]);
    let second = scratch.picture("second.png", (2, 8), [0, 0, 255, 255]);
    let (jobs, incoming) = channel();
    let (outgoing, replies) = channel();
    jobs.send(vec![stagefile_only("passed-one.png"), stagefile_only("passed-two.png")]).expect("the job is sent");
    let newest = vec![
        Pictures { stagefile: Some(scratch.root.join("missing.png")), banner: Some(second.clone()) },
        stagefile_only(first.to_str().expect("the path is text")),
    ];
    jobs.send(newest.clone()).expect("the job is sent");
    drop(jobs);
    make_squares(&incoming, &outgoing);
    let answered: Vec<Reply> = replies.try_iter().collect();
    assert_eq!(answered.iter().map(|reply| &reply.pictures).collect::<Vec<_>>(), newest.iter().collect::<Vec<_>>(), "the passed list was not read");
    assert!(answered.iter().all(|reply| reply.square.is_some()), "a stage file that is not there falls back to the banner");
}

#[test]
fn the_squares_used_longest_ago_are_let_go_of() {
    let mut thumbs = SongThumbs::default();
    for number in 0..=CACHE_LIMIT {
        thumbs.keep(Reply { pictures: stagefile_only(&format!("{number}.png")), square: None });
    }
    assert_eq!(thumbs.kept_count(), CACHE_LIMIT);
    assert!(thumbs.kept.iter().all(|kept| kept.pictures != stagefile_only("0.png")), "the first answer is the one let go of");
    thumbs.keep(Reply { pictures: stagefile_only("1.png"), square: None });
    assert_eq!(thumbs.kept_count(), CACHE_LIMIT, "an answer already kept is not kept twice");
}

#[test]
fn a_frame_shows_a_square_on_every_bar_near_the_cursor_whose_picture_has_been_read() {
    let scratch = Scratch::new("frame");
    let stage = scratch.picture("stage.png", (6, 3), [255, 0, 0, 255]);
    let banner = scratch.picture("banner.png", (8, 2), [0, 0, 255, 255]);
    let broken = scratch.root.join("broken.png");
    std::fs::write(&broken, b"not a picture").expect("the broken file is written");
    let bars = vec![
        chart_bar(Some(&stage), None),
        chart_bar(None, None),
        chart_bar(Some(&broken), Some(&banner)),
        SelectBar::new(BarKind::Folder, "FOLDER"),
        chart_bar(Some(&broken), None),
    ];
    let mut canvas = CpuCanvas::new(CANVAS_SIDE, CANVAS_SIDE);
    let mut thumbs = SongThumbs::default();

    assert!(thumbs.frame(&mut canvas, &bars, 0).is_empty(), "nothing is shown before it has been read");
    thumbs.settle(ANSWER_WAIT);
    let shown = thumbs.frame(&mut canvas, &bars, 0).to_vec();
    assert_eq!(shown.iter().map(|(bar, _)| *bar).collect::<Vec<_>>(), [0, 2], "only the bars with a picture that could be read show one");
    for (_, texture) in &shown {
        assert_eq!(canvas.texture_size(*texture), Some((THUMB_SIDE, THUMB_SIDE)));
    }
    assert_ne!(shown[0].1, shown[1].1, "two bars hold two textures");
    assert_eq!(thumbs.frame(&mut canvas, &bars, 0), shown.as_slice(), "a frame with nothing changed keeps the same textures");
    assert!(thumbs.asked.is_empty(), "nothing is asked for again once it has been read");

    let none = vec![chart_bar(None, None); 3];
    let mut idle = SongThumbs::default();
    assert!(idle.frame(&mut canvas, &none, 0).is_empty());
    assert!(idle.worker.is_none(), "a list naming no pictures starts no worker");
}
