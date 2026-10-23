//! What the tests of every part share: a folder of its own for each test, and archives written to order.

use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

mod extract;
mod fetch;
mod link;
mod name;

const CHART: &[u8] = b"#TITLE test\n#BPM 120\n#WAV01 kick.wav\n#00111:01\n";
const SOUND: &[u8] = b"RIFF....WAVEfmt ";
const MD5: &str = "0123456789abcdef0123456789abcdef";

/// Keeps one test's folder clear of another's when the suite runs them in one process.
static FIXTURE_SEQUENCE: AtomicU32 = AtomicU32::new(0);

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Fixture {
        let sequence = FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("rbms-download-{tag}-{}-{sequence}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create the fixture root");
        Fixture { root }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.path(name);
        fs::write(&path, bytes).expect("write the fixture file");
        path
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn not_cancelled() -> AtomicBool {
    AtomicBool::new(false)
}

/// Every file under `dir`, as `/`-separated paths relative to it, in order.
fn files_under(dir: &Path) -> Vec<String> {
    fn walk(dir: &Path, prefix: &str, found: &mut Vec<String>) {
        for entry in fs::read_dir(dir).expect("read the folder") {
            let entry = entry.expect("read the entry");
            let name = format!("{prefix}{}", entry.file_name().to_string_lossy());
            if entry.file_type().expect("read the entry type").is_dir() {
                walk(&entry.path(), &format!("{name}/"), found);
            } else {
                found.push(name);
            }
        }
    }
    let mut found = Vec::new();
    walk(dir, "", &mut found);
    found.sort();
    found
}

/// A zip archive written entry by entry. Names go through the real writer, so a name the writer
/// would not take as it is can be written as a placeholder of the same length and swapped for its
/// bytes once the archive is done, which is also how a name that is not UTF-8 gets in.
struct ArchiveBuilder {
    writer: ZipWriter<Cursor<Vec<u8>>>,
    swaps: Vec<(String, Vec<u8>)>,
}

impl ArchiveBuilder {
    fn new() -> ArchiveBuilder {
        ArchiveBuilder { writer: ZipWriter::new(Cursor::new(Vec::new())), swaps: Vec::new() }
    }

    fn entry(mut self, name: &str, data: &[u8], method: CompressionMethod) -> ArchiveBuilder {
        self.writer.start_file(name, SimpleFileOptions::default().compression_method(method)).expect("start the entry");
        self.writer.write_all(data).expect("write the entry");
        self
    }

    fn stored(self, name: &str, data: &[u8]) -> ArchiveBuilder {
        self.entry(name, data, CompressionMethod::Stored)
    }

    fn deflated(self, name: &str, data: &[u8]) -> ArchiveBuilder {
        self.entry(name, data, CompressionMethod::Deflated)
    }

    fn directory(mut self, name: &str) -> ArchiveBuilder {
        self.writer.add_directory(name, SimpleFileOptions::default()).expect("add the folder entry");
        self
    }

    fn symlink(mut self, name: &str, target: &str) -> ArchiveBuilder {
        self.writer.add_symlink(name, target, SimpleFileOptions::default()).expect("add the link entry");
        self
    }

    fn name_bytes(mut self, placeholder: &str, raw: &[u8]) -> ArchiveBuilder {
        assert_eq!(placeholder.len(), raw.len(), "a swapped name keeps its length");
        self.swaps.push((placeholder.to_string(), raw.to_vec()));
        self
    }

    fn build(self) -> Vec<u8> {
        let mut bytes = self.writer.finish().expect("finish the archive").into_inner();
        for (placeholder, raw) in &self.swaps {
            let mut at = 0;
            while at + raw.len() <= bytes.len() {
                if bytes[at..at + raw.len()] == *placeholder.as_bytes() {
                    bytes[at..at + raw.len()].copy_from_slice(raw);
                    at += raw.len();
                } else {
                    at += 1;
                }
            }
        }
        bytes
    }
}
