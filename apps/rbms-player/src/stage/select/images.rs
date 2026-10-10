//! The pictures a skin's browser shows for the chart under the cursor: its stage file, its banner
//! and a back image, which a document draws by reference rather than ships.
//!
//! The reference reads the stage file and the banner of every bar of a list on a thread of its own,
//! as soon as the list is made (`BarManager.BarContentsLoaderThread`), and the moment the cursor
//! lands on a bar it shows whichever of that bar's two pictures have been read by then and nothing
//! for the ones that have not (`MusicSelector.loadSelectedSongImages`). There is no pause between
//! the cursor moving and the pictures changing: the bar that was left gives its pictures up at once.
//! The back image is never read while browsing at all. What a browser's document draws for it is
//! whatever the chart that was loaded last left behind (`BMSResource.setBMSFile`), and nothing
//! before the first chart has been loaded.
//!
//! This keeps those rules and reads less. A list here can be every chart of the library, and
//! holding two decoded pictures for each of them is not affordable, so only the pictures the frame
//! can show are read: the two of the chart under the cursor and the back image of the chart loaded
//! last. Decoding happens on one worker thread, which the frame loop never waits for. A picture
//! the cursor has already left is not decoded when a newer request reached the worker first, so
//! turning the wheel through a long list costs one decode at a time rather than one per bar.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};

use rbms_render::{ReferenceImages, Renderer, TextureId};

use crate::stage::Canvas;
use crate::{DecodedImage, decode_bga_image};

/// How many pictures a frame can refer to.
const SLOTS: usize = 3;

/// Which picture a slot holds.
const STAGEFILE: usize = 0;
const BANNER: usize = 1;
const BACKBMP: usize = 2;

/// The names the three pictures are registered with a draw target under, by slot. One name each for
/// as long as the application runs, so a new picture replaces the pixels of the one before it.
const TEXTURE_KEYS: [&str; SLOTS] = ["rbms.player.select.stagefile", "rbms.player.select.banner", "rbms.player.select.backbmp"];

/// One request to the worker: what each slot named in it should hold from now on.
struct Job {
    /// Which request this is. A picture is taken in only by the slot whose newest request it
    /// answers.
    id: u64,
    /// The slots this request changes, each with the file to read into it, or `None` for a slot
    /// that is to hold nothing.
    slots: Vec<(usize, Option<PathBuf>)>,
}

/// One picture the worker read, or `None` in `image` for a file that is not there or is not a
/// picture.
struct Reply {
    id: u64,
    slot: usize,
    image: Option<DecodedImage>,
}

/// The two ends the frame loop holds of the worker thread.
struct Worker {
    jobs: Sender<Job>,
    replies: Receiver<Reply>,
}

/// What a slot holds.
#[derive(Default)]
enum Picture {
    /// Nothing was asked for.
    #[default]
    Unasked,
    /// A file was asked for and the worker has not answered yet.
    Waiting,
    /// The file is not there or is not a picture.
    Unreadable,
    Read(DecodedImage),
}

impl Picture {
    /// The picture, when one has been read.
    fn image(&self) -> Option<&DecodedImage> {
        if let Picture::Read(image) = self { Some(image) } else { None }
    }
}

/// A picture as a draw target holds it.
#[derive(Clone, Copy)]
struct Uploaded {
    /// The decode the pixels came from.
    generation: u64,
    texture: TextureId,
    /// Whether the target took the picture. One it refused -- a picture larger than it can hold --
    /// is not offered to it again.
    taken: bool,
}

/// The files a frame should show, each `None` for a picture there is none of.
#[derive(Clone, Copy, Default)]
pub(super) struct Wanted<'a> {
    /// The stage file of the chart under the cursor.
    pub(super) stagefile: Option<&'a Path>,
    /// The banner of the chart under the cursor.
    pub(super) banner: Option<&'a Path>,
    /// The back image of the chart loaded last.
    pub(super) backbmp: Option<&'a Path>,
}

/// The pictures of the chart under the cursor, read off the frame loop and kept until the cursor
/// moves on.
#[derive(Default)]
pub(super) struct SongImages {
    /// The worker thread, started by the first request that names a file.
    worker: Option<Worker>,
    /// The file each slot was last asked to hold.
    wanted: [Option<PathBuf>; SLOTS],
    /// The request each slot was last asked by.
    asked: [u64; SLOTS],
    requests: u64,
    held: [Picture; SLOTS],
    uploaded: [Option<Uploaded>; SLOTS],
}

impl SongImages {
    /// Says which files the frame should show from now on.
    ///
    /// A slot whose file has not changed keeps the picture it holds. A slot whose file has changed
    /// gives its picture up at once and takes the new one in when the worker has read it.
    pub(super) fn want(&mut self, wanted: Wanted<'_>) {
        let asked_for = [(STAGEFILE, wanted.stagefile), (BANNER, wanted.banner), (BACKBMP, wanted.backbmp)];
        let changed: Vec<(usize, Option<PathBuf>)> = asked_for
            .into_iter()
            .filter(|(slot, file)| self.wanted[*slot].as_deref() != *file)
            .map(|(slot, file)| (slot, file.map(Path::to_path_buf)))
            .collect();
        if changed.is_empty() {
            return;
        }
        self.requests += 1;
        for (slot, file) in &changed {
            self.wanted[*slot].clone_from(file);
            self.asked[*slot] = self.requests;
            self.held[*slot] = if file.is_some() { Picture::Waiting } else { Picture::Unasked };
        }
        if self.worker.is_none() && changed.iter().all(|(_, file)| file.is_none()) {
            return;
        }
        let worker = self.worker.get_or_insert_with(spawn_worker);
        if worker.jobs.send(Job { id: self.requests, slots: changed }).is_err() {
            self.worker = None;
        }
    }

    /// Takes in whatever the worker has read since the last frame. A picture read for a file the
    /// cursor has since left is dropped.
    pub(super) fn poll(&mut self) {
        let Some(worker) = &self.worker else {
            return;
        };
        loop {
            match worker.replies.try_recv() {
                Ok(reply) => {
                    if self.asked.get(reply.slot) == Some(&reply.id) {
                        self.held[reply.slot] = reply.image.map_or(Picture::Unreadable, Picture::Read);
                    }
                }
                Err(TryRecvError::Empty) => return,
                Err(TryRecvError::Disconnected) => {
                    self.worker = None;
                    return;
                }
            }
        }
    }

    /// The picture the built-in browser shows as a chart's cover: the stage file, or the banner
    /// when there is none.
    pub(super) fn cover(&self) -> Option<&DecodedImage> {
        self.held[STAGEFILE].image().or(self.held[BANNER].image())
    }

    /// The pictures this frame can refer to, uploaded to `canvas`. A picture is uploaded once for
    /// each decode, and again only when the target no longer holds it, which is what a target made
    /// anew looks like. A slot with nothing read into it refers to nothing.
    pub(super) fn textures(&mut self, canvas: &mut Canvas<'_>) -> ReferenceImages {
        let mut textures = [None; SLOTS];
        for (slot, texture) in textures.iter_mut().enumerate() {
            let Some(image) = self.held[slot].image() else {
                continue;
            };
            let size = Some((image.width, image.height));
            let current = self.uploaded[slot].filter(|held| held.generation == image.generation && (!held.taken || canvas.texture_size(held.texture) == size));
            let uploaded = current.unwrap_or_else(|| {
                let texture = canvas.register_texture(TEXTURE_KEYS[slot], &image.rgba, image.width, image.height);
                Uploaded { generation: image.generation, texture, taken: canvas.texture_size(texture) == size }
            });
            self.uploaded[slot] = Some(uploaded);
            *texture = uploaded.taken.then_some(uploaded.texture);
        }
        ReferenceImages { stagefile: textures[STAGEFILE], backbmp: textures[BACKBMP], banner: textures[BANNER] }
    }

    /// Whether every file asked for has been read or found unreadable, so nothing more will change
    /// until the next request.
    #[cfg(test)]
    pub(super) fn is_settled(&self) -> bool {
        !self.held.iter().any(|picture| matches!(picture, Picture::Waiting))
    }

    /// The size of the picture each slot holds -- stage file, banner, back image -- for the tests
    /// that follow one being replaced.
    #[cfg(test)]
    pub(super) fn sizes(&self) -> [Option<(u32, u32)>; SLOTS] {
        [STAGEFILE, BANNER, BACKBMP].map(|slot| self.held[slot].image().map(|image| (image.width, image.height)))
    }
}

/// Starts the thread that reads pictures, which runs until the browser that started it is gone.
fn spawn_worker() -> Worker {
    let (jobs, incoming) = channel();
    let (outgoing, replies) = channel();
    std::thread::spawn(move || read_pictures(&incoming, &outgoing));
    Worker { jobs, replies }
}

/// The worker's loop: keeps the newest file asked of each slot and reads one of them at a time,
/// looking for newer requests between every two reads.
fn read_pictures(jobs: &Receiver<Job>, replies: &Sender<Reply>) {
    let mut pending: [Option<(u64, PathBuf)>; SLOTS] = [None, None, None];
    loop {
        if pending.iter().all(Option::is_none) {
            match jobs.recv() {
                Ok(job) => take_job(&mut pending, job),
                Err(_) => return,
            }
        }
        while let Ok(job) = jobs.try_recv() {
            take_job(&mut pending, job);
        }
        let Some((slot, (id, file))) = pending.iter_mut().enumerate().find_map(|(slot, asked)| Some((slot, asked.take()?))) else {
            continue;
        };
        if replies.send(Reply { id, slot, image: read_picture(&file) }).is_err() {
            return;
        }
    }
}

/// Puts a request's files in front of whatever its slots were still waiting to read.
fn take_job(pending: &mut [Option<(u64, PathBuf)>; SLOTS], job: Job) {
    for (slot, file) in job.slots {
        if let Some(asked) = pending.get_mut(slot) {
            *asked = file.map(|file| (job.id, file));
        }
    }
}

/// Reads one picture a chart names, the way a chart's background images are read: the name is
/// looked for beside the chart with the extensions a chart may have meant.
fn read_picture(file: &Path) -> Option<DecodedImage> {
    decode_bga_image(file.parent()?, file.file_name()?.to_str()?)
}
