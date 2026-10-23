//! The small pictures a skin's wheel shows on its bars: one square cut from each chart's stage
//! file, or from its banner when it has none, for the bars near the cursor.
//!
//! The reference's wheel has no picture on a bar; a document written for this player places one
//! (`songlist.thumb`) and the browser supplies it here. A list can be every chart of the library, so
//! only the bars around the cursor are ever read, and a picture is kept as a square of
//! [`THUMB_SIDE`] pixels a side, never at the size of its file.
//!
//! Reading happens on one worker thread, which the frame loop never waits for, and the worker always
//! works through the newest list of bars it was given: bars the wheel has turned past are not read.
//! It cuts the middle square out of each picture, scales it down and sends the square; the decode it
//! came from is dropped there. The frame keeps the [`CACHE_LIMIT`] squares used last, and the
//! renderer holds at most [`TEXTURE_POOL`] of them at once, which is one for every bar the browser
//! reads at a time.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};

use image::imageops::{self, FilterType};
use rbms_render::{Renderer, TextureId};

use super::images::read_picture;
use crate::{BarImages, DecodedImage, SelectBar};

/// The side of a bar's picture, in pixels.
const THUMB_SIDE: u32 = 128;

/// How many bars above the cursor, and how many below it, have their pictures read. Together with
/// the bar under the cursor this is more than a wheel of fifteen slots shows.
const BARS_ABOVE: usize = 11;
const BARS_BELOW: usize = 12;

/// How many pictures the renderer holds at once: one for every bar that is read.
const TEXTURE_POOL: usize = BARS_ABOVE + 1 + BARS_BELOW;

/// How many squares are kept, the ones used last, so that turning the wheel back and forth over a
/// few bars reads nothing twice.
const CACHE_LIMIT: usize = 48;

/// What the pool's textures are registered under, followed by the number of the texture.
const TEXTURE_KEY_PREFIX: &str = "rbms.player.select.thumb.";

/// The two files a bar's picture may come from, in the order they are tried.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Pictures {
    stagefile: Option<PathBuf>,
    banner: Option<PathBuf>,
}

impl Pictures {
    /// The files a chart names, or `None` for a chart that names neither.
    fn of(images: &BarImages) -> Option<Pictures> {
        (images.stagefile.is_some() || images.banner.is_some()).then(|| Pictures { stagefile: images.stagefile.clone(), banner: images.banner.clone() })
    }

    /// Whether these are the files `images` names.
    fn names(&self, images: &BarImages) -> bool {
        self.stagefile == images.stagefile && self.banner == images.banner
    }

    /// The files in the order they are tried.
    fn files(&self) -> impl Iterator<Item = &Path> {
        self.stagefile.iter().chain(&self.banner).map(PathBuf::as_path)
    }
}

/// One square the worker made, or `None` in `square` when neither file is a picture.
struct Reply {
    pictures: Pictures,
    square: Option<Vec<u8>>,
}

/// The two ends the frame loop holds of the worker thread.
struct Worker {
    jobs: Sender<Vec<Pictures>>,
    replies: Receiver<Reply>,
}

/// One kept answer of the worker.
struct Kept {
    pictures: Pictures,
    /// This answer's own number, which tells a texture holding it from one holding another.
    id: u64,
    /// The square, or `None` when the bar has no picture that could be read.
    square: Option<Vec<u8>>,
}

/// One texture of the pool.
struct PoolTexture {
    key: String,
    /// The answer it holds and the texture it is held in.
    held: Option<(u64, TextureId)>,
    /// The frame it was last shown on.
    shown_on: u64,
}

/// The pictures of the bars near the cursor, read off the frame loop.
#[derive(Default)]
pub(super) struct SongThumbs {
    /// The worker thread, started by the first frame that wants a picture read.
    worker: Option<Worker>,
    /// The worker's answers, the one used last at the back.
    kept: VecDeque<Kept>,
    answers: u64,
    /// The pictures the worker was last asked for, in the order it reads them.
    asked: Vec<Pictures>,
    pool: Vec<PoolTexture>,
    frames: u64,
    /// The bars this frame shows a picture on, each with its texture.
    shown: Vec<(usize, TextureId)>,
}

impl SongThumbs {
    /// The pictures of the bars near `cursor` in `bars` that have been read, each registered with
    /// `r` and named by the bar's place in `bars`; and asks the worker for the ones that have not.
    ///
    /// A bar the worker has not answered for yet shows no picture, and neither does one whose files
    /// are not pictures.
    pub(super) fn frame<R: Renderer>(&mut self, r: &mut R, bars: &[SelectBar], cursor: usize) -> &[(usize, TextureId)] {
        self.poll();
        self.frames += 1;
        self.shown.clear();
        let mut wanted = Vec::new();
        for bar in near(bars.len(), cursor) {
            let Some(images) = bars.get(bar).and_then(|bar| bar.chart.as_ref()).map(|chart| &chart.images) else {
                continue;
            };
            let Some(at) = self.kept.iter().position(|kept| kept.pictures.names(images)) else {
                wanted.extend(Pictures::of(images));
                continue;
            };
            let Some(kept) = self.kept.remove(at) else {
                continue;
            };
            let texture = kept.square.as_deref().and_then(|square| self.texture(r, kept.id, square));
            self.kept.push_back(kept);
            if let Some(texture) = texture {
                self.shown.push((bar, texture));
            }
        }
        self.ask(wanted);
        &self.shown
    }

    /// Hands the worker the pictures the frame wants and has not got, in the order it should read
    /// them, when one of them is not among those it was last asked for. A list that has only lost
    /// pictures since is left to the worker as it is, so a picture it is reading is not read twice.
    fn ask(&mut self, wanted: Vec<Pictures>) {
        if wanted.iter().all(|pictures| self.asked.contains(pictures)) {
            return;
        }
        let worker = self.worker.get_or_insert_with(spawn_worker);
        if worker.jobs.send(wanted.clone()).is_err() {
            self.worker = None;
            return;
        }
        self.asked = wanted;
    }

    /// Takes in whatever the worker has answered since the last frame.
    fn poll(&mut self) {
        loop {
            let Some(worker) = &self.worker else {
                return;
            };
            match worker.replies.try_recv() {
                Ok(reply) => self.keep(reply),
                Err(TryRecvError::Empty) => return,
                Err(TryRecvError::Disconnected) => {
                    self.worker = None;
                    return;
                }
            }
        }
    }

    /// Keeps one answer, letting go of the one used longest ago when there are too many.
    fn keep(&mut self, reply: Reply) {
        self.asked.retain(|pictures| *pictures != reply.pictures);
        if self.kept.iter().any(|kept| kept.pictures == reply.pictures) {
            return;
        }
        self.answers += 1;
        self.kept.push_back(Kept { pictures: reply.pictures, id: self.answers, square: reply.square });
        while self.kept.len() > CACHE_LIMIT {
            self.kept.pop_front();
        }
    }

    /// The texture holding answer `id`, registering `square` into the pool's texture that was shown
    /// longest ago when no texture holds it. `None` when every texture is taken by this frame or the
    /// renderer will not take the square.
    fn texture<R: Renderer>(&mut self, r: &mut R, id: u64, square: &[u8]) -> Option<TextureId> {
        let size = Some((THUMB_SIDE, THUMB_SIDE));
        if self.pool.is_empty() {
            self.pool = (0..TEXTURE_POOL).map(|number| PoolTexture { key: format!("{TEXTURE_KEY_PREFIX}{number}"), held: None, shown_on: 0 }).collect();
        }
        let frame = self.frames;
        if let Some(held) = self.pool.iter_mut().find(|held| held.held.is_some_and(|(holds, texture)| holds == id && r.texture_size(texture) == size)) {
            held.shown_on = frame;
            return held.held.map(|(_, texture)| texture);
        }
        let free = self.pool.iter_mut().filter(|held| held.shown_on != frame).min_by_key(|held| held.shown_on)?;
        let texture = r.register_texture(&free.key, square, THUMB_SIDE, THUMB_SIDE);
        free.shown_on = frame;
        free.held = (r.texture_size(texture) == size).then_some((id, texture));
        free.held.map(|(_, texture)| texture)
    }

    /// Waits on the worker until it has answered every picture it was asked for.
    #[cfg(test)]
    pub(super) fn settle(&mut self, wait: std::time::Duration) {
        while !self.asked.is_empty() {
            let reply = self.worker.as_ref().expect("the worker is running").replies.recv_timeout(wait).expect("the worker answered in time");
            self.keep(reply);
        }
    }

    /// How many answers are kept.
    #[cfg(test)]
    pub(super) fn kept_count(&self) -> usize {
        self.kept.len()
    }
}

/// The bars whose pictures are read with the cursor on `cursor` of a list of `len` bars, nearest
/// first, going round the ends of the list as the wheel does and naming each bar once.
fn near(len: usize, cursor: usize) -> Vec<usize> {
    if len == 0 {
        return Vec::new();
    }
    let mut bars = Vec::with_capacity(TEXTURE_POOL);
    bars.push(cursor % len);
    for step in 1..=BARS_ABOVE.max(BARS_BELOW) {
        let below = (step <= BARS_BELOW).then(|| (cursor + step) % len);
        let above = (step <= BARS_ABOVE).then(|| (cursor + len - step % len) % len);
        for bar in below.into_iter().chain(above) {
            if !bars.contains(&bar) {
                bars.push(bar);
            }
        }
    }
    bars
}

/// Starts the thread that makes the squares, which runs until the browser that started it is gone.
fn spawn_worker() -> Worker {
    let (jobs, incoming) = channel();
    let (outgoing, replies) = channel();
    std::thread::spawn(move || make_squares(&incoming, &outgoing));
    Worker { jobs, replies }
}

/// The worker's loop: works through the newest list it was given, one picture at a time, looking for
/// a newer list between every two.
fn make_squares(jobs: &Receiver<Vec<Pictures>>, replies: &Sender<Reply>) {
    let mut pending: VecDeque<Pictures> = VecDeque::new();
    loop {
        if pending.is_empty() {
            match jobs.recv() {
                Ok(job) => pending = job.into(),
                Err(_) => return,
            }
        }
        while let Ok(job) = jobs.try_recv() {
            pending = job.into();
        }
        let Some(pictures) = pending.pop_front() else {
            continue;
        };
        let square = pictures.files().find_map(read_picture).and_then(square_of);
        if replies.send(Reply { pictures, square }).is_err() {
            return;
        }
    }
}

/// The middle square of a picture scaled to [`THUMB_SIDE`] pixels a side, as RGBA8, or `None` for a
/// picture with no pixels.
fn square_of(picture: DecodedImage) -> Option<Vec<u8>> {
    let picture = image::RgbaImage::from_raw(picture.width, picture.height, picture.rgba)?;
    let side = picture.width().min(picture.height());
    if side == 0 {
        return None;
    }
    let middle = imageops::crop_imm(&picture, (picture.width() - side) / 2, (picture.height() - side) / 2, side, side).to_image();
    Some(imageops::resize(&middle, THUMB_SIDE, THUMB_SIDE, FilterType::Triangle).into_raw())
}

#[cfg(test)]
#[path = "thumbs_tests.rs"]
mod tests;
