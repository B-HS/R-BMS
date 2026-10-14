//! Builds the table of the default skin pack's files that the player carries inside its binary.
//!
//! The pack is `assets/skins/rbms-default`: its skin documents, and everything under its `lua`,
//! `image`, `font` and `sound` folders. The folder is walked here rather than listed by hand, so a
//! file added to the pack is shipped by the next build and one removed from it stops being shipped.
//! The scripts the pack's images and sounds are generated with (`tools`) are not part of what a
//! player is given and are left out.
//!
//! The table is written to `default_pack.rs` in the build's output folder as one expression, a
//! slice of `EmbeddedFile`, which `src/assets/default_pack.rs` includes. Each entry carries the
//! file's path inside the pack with `/` between its parts, the SHA-256 of its bytes, and the bytes
//! themselves. The entries are in the byte order of their paths, which is the same on every
//! platform whatever order a folder is listed in.
//!
//! A file the renderer already carries is not embedded a second time: its entry names the
//! renderer's copy ([`SHARED_FILES`]), and the build stops if the two files have drifted apart.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// The pack's folder, from this package's own.
const PACK_FOLDER: &str = "../../assets/skins/rbms-default";

/// The extension of the skin documents at the top of the pack.
const DOCUMENT_EXTENSION: &str = "luaskin";

/// The folders of the pack that are shipped whole.
const SHIPPED_FOLDERS: [&str; 4] = ["font", "image", "lua", "sound"];

/// What a file a desktop left behind in a folder starts with. Nothing a skin reads is named so.
const HIDDEN_PREFIX: char = '.';

/// The file the table is written to, in the build's output folder.
const TABLE_FILE: &str = "default_pack.rs";

/// One file of the pack that another crate already carries.
struct SharedFile {
    /// The file's path inside the pack.
    name: &'static str,
    /// The file the other crate embeds, from this package's folder.
    source: &'static str,
    /// The expression that names the other crate's copy.
    bytes: &'static str,
}

/// The files of the pack that are embedded once for the whole binary.
const SHARED_FILES: [SharedFile; 1] =
    [SharedFile { name: "font/Inter-Regular.ttf", source: "../../assets/fonts/Inter-Regular.ttf", bytes: "&rbms_render::font::BUNDLED_FONT" }];

/// Whether a file name is one the pack ships.
fn is_shipped_name(name: &str) -> bool {
    !name.starts_with(HIDDEN_PREFIX)
}

/// Every file under `folder`, as its path below `root` with `/` between its parts.
fn files_below(root: &Path, folder: &Path, found: &mut Vec<String>) {
    let entries = std::fs::read_dir(folder).unwrap_or_else(|error| panic!("{} cannot be listed: {error}", folder.display()));
    for entry in entries {
        let entry = entry.unwrap_or_else(|error| panic!("{} cannot be listed: {error}", folder.display()));
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if !is_shipped_name(&name) {
            continue;
        }
        if path.is_dir() {
            files_below(root, &path, found);
            continue;
        }
        let relative = path.strip_prefix(root).unwrap_or_else(|_| panic!("{} is not below the pack", path.display()));
        let parts: Vec<String> = relative.components().map(|part| part.as_os_str().to_string_lossy().into_owned()).collect();
        found.push(parts.join("/"));
    }
}

/// The path of every file the pack ships, in byte order.
fn shipped_files(pack: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let entries = std::fs::read_dir(pack).unwrap_or_else(|error| panic!("{} cannot be listed: {error}", pack.display()));
    for entry in entries {
        let entry = entry.unwrap_or_else(|error| panic!("{} cannot be listed: {error}", pack.display()));
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let is_document = path.is_file() && path.extension().and_then(|extension| extension.to_str()) == Some(DOCUMENT_EXTENSION);
        if is_document && is_shipped_name(&name) {
            found.push(name);
        }
    }
    for folder in SHIPPED_FOLDERS {
        let folder = pack.join(folder);
        if folder.is_dir() {
            files_below(pack, &folder, &mut found);
        }
    }
    found.sort_unstable();
    found
}

/// The variable a release build names its compiled Windows resource (the app icon) in.
const WINDOWS_RESOURCE_VARIABLE: &str = "RBMS_WINDOWS_RESOURCE";

/// Links the compiled Windows resource a release build names into the player's executable, which
/// gives `rbms-player.exe` the app icon in Explorer and on the taskbar. The release workflow
/// compiles `assets/icon/rbms.rc` with the Windows SDK's `rc.exe` and names the `.res` it wrote;
/// any other build names none and links nothing, so no build depends on the SDK tool. A name that
/// does not lead to a file stops the build rather than shipping an executable without its icon.
fn link_windows_resource() {
    println!("cargo:rerun-if-env-changed={WINDOWS_RESOURCE_VARIABLE}");
    let Some(resource) = std::env::var_os(WINDOWS_RESOURCE_VARIABLE).map(PathBuf::from) else {
        return;
    };
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    assert!(target_os == "windows" && target_env == "msvc", "{WINDOWS_RESOURCE_VARIABLE} is set for a target that is not Windows MSVC");
    assert!(resource.is_file(), "{WINDOWS_RESOURCE_VARIABLE} names {} which is not a file", resource.display());
    println!("cargo:rerun-if-changed={}", resource.display());
    println!("cargo:rustc-link-arg-bins={}", resource.display());
}

fn main() {
    let package = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("cargo names the package folder"));
    let output = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo names the output folder"));
    let pack = package.join(PACK_FOLDER);
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", pack.display());
    link_windows_resource();

    let mut table = String::from("&[\n");
    for name in shipped_files(&pack) {
        let bytes = std::fs::read(pack.join(&name)).unwrap_or_else(|error| panic!("{name} cannot be read: {error}"));
        let digest: String = Sha256::digest(&bytes).iter().fold(String::new(), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        });
        let source = match SHARED_FILES.iter().find(|shared| shared.name == name) {
            Some(shared) => {
                let other = package.join(shared.source);
                println!("cargo:rerun-if-changed={}", other.display());
                let carried = std::fs::read(&other).unwrap_or_else(|error| panic!("{} cannot be read: {error}", other.display()));
                assert!(carried == bytes, "{name} is no longer the same file as {}, which is embedded in its place", other.display());
                shared.bytes.to_owned()
            }
            None => format!("include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/{PACK_FOLDER}/\", {name:?}))"),
        };
        let _ = writeln!(table, "    EmbeddedFile {{ name: {name:?}, digest: {digest:?}, bytes: {source} }},");
    }
    table.push_str("]\n");
    let written = output.join(TABLE_FILE);
    std::fs::write(&written, table).unwrap_or_else(|error| panic!("{} cannot be written: {error}", written.display()));
}
