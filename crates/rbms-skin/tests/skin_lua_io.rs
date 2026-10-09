//! The `io` and `os` libraries a `.luaskin` is given: reads that look in the write overlay before
//! the skin root, writes that only ever land in the overlay, names that cannot leave the root, the
//! shapes the reference's library answers in, and the handful of `os` functions that are left.
//!
//! Every fixture under `tests/fixtures/luaio` was written for these tests. The files whose exact
//! bytes matter -- carriage returns, a missing final line feed, a symbolic link -- are written into a
//! scratch directory by the test that needs them, so no checkout setting can change them.

#![cfg(feature = "lua")]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use mlua::FromLuaMulti;
use rbms_skin::lua::{SkinLua, SkinLuaConfig};

/// The seed every interpreter of these tests is pinned to.
const TEST_SEED: u64 = 42;

/// What every answered failure starts with.
const FAILURE_PREFIX: &str = "io error: ";

/// What a name outside the skin root is refused with.
const ACCESS_DENIED: &str = "io error: Lua skin file access denied: ";

/// What a read-mode open of a missing file is answered with.
const NOT_FOUND: &str = "io error: Lua file not found: ";

/// The environment variable that names an external skin pack for the optional test.
const SKIN_PACK_ENV: &str = "RBMS_SKIN_PACK";

/// The registry field the C libraries record themselves under when they are opened.
const NATIVE_LOADED_TABLE: &str = "_LOADED";

/// The module of the pack these tests were measured against that reads the calendar as it loads.
const PACK_TIME_MODULE_FILE: &str = "Root/customtime.lua";

/// The largest file of an external pack the optional test will append to.
const PACK_SAMPLE_MAX_BYTES: u64 = 64 * 1024;

/// How far `os.time()` may be from the clock this test reads right after it, in seconds.
const CLOCK_TOLERANCE_SECONDS: f64 = 5.0;

/// One billion seconds after the epoch: a moment in 2001 no time zone rule is ambiguous about.
const SAMPLE_MOMENT: i64 = 1_000_000_000;

/// Files a skin may hold open at once.
const OPEN_FILE_LIMIT: usize = 64;

/// How many files a leak test opens without closing or keeping any: many times the limit.
const LEAKED_OPENS: usize = 1_000;

/// The fixture skin's root.
fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures").join("luaio")
}

/// A scratch directory that removes itself, for the tests that need files written on the spot.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("rbms-luaio-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("the scratch directory should be creatable");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    /// Where this scratch keeps a skin's write overlay. Nothing creates it but the skin's writes.
    fn overlay(&self) -> PathBuf {
        self.0.join("overlay")
    }

    /// Writes `bytes` to `relative` under the scratch directory, creating its parents.
    fn write(&self, relative: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(relative);
        std::fs::create_dir_all(path.parent().expect("a scratch file has a parent")).expect("the parent should be creatable");
        std::fs::write(&path, bytes).expect("the scratch file should be writable");
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A seeded interpreter for the skin at `root`, writing to `overlay` when there is one.
fn skin(root: &Path, overlay: Option<&Path>) -> SkinLua {
    SkinLua::new(SkinLuaConfig { overlay: overlay.map(Path::to_path_buf), seed: Some(TEST_SEED), ..SkinLuaConfig::new(root) }).expect("the runtime builds")
}

/// Runs a chunk in the interpreter and reads what it returns.
fn eval<T: FromLuaMulti>(runtime: &SkinLua, source: &str) -> T {
    runtime.lua().load(source).eval().unwrap_or_else(|error| panic!("{source} should run: {error}"))
}

/// Every file under `root` with its bytes, by its path relative to `root`.
fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut found = BTreeMap::new();
    if !root.exists() {
        return found;
    }
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).expect("the directory should be listable").flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if let Ok(relative) = path.strip_prefix(root) {
                found.insert(relative.to_path_buf(), std::fs::read(&path).expect("the file should be readable"));
            }
        }
    }
    found
}

/// Every file under `root` with its size and the moment it was last changed, by its path relative
/// to `root`. This is how a pack too large to read whole is shown to be untouched.
fn listing(root: &Path) -> BTreeMap<PathBuf, (u64, SystemTime)> {
    let mut found = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).expect("the directory should be listable").flatten() {
            let path = entry.path();
            let metadata = entry.metadata().expect("the entry should be readable");
            if metadata.is_dir() {
                pending.push(path);
            } else if let Ok(relative) = path.strip_prefix(root) {
                found.insert(relative.to_path_buf(), (metadata.len(), metadata.modified().expect("the platform keeps modification times")));
            }
        }
    }
    found
}

/// The bytes of one fixture file.
fn fixture(relative: &str) -> Vec<u8> {
    std::fs::read(fixture_root().join(relative)).expect("the fixture should be readable")
}

/// The whole text of a file as the skin reads it.
fn read_all(runtime: &SkinLua, path: &str) -> String {
    eval(runtime, &format!("local file = io.open('{path}') local text = file:read('*a') file:close() return text"))
}

/// What opening `path_expression` in `mode` answers: whether the first value is `nil`, the second
/// value, and how many values there are.
fn open_outcome(runtime: &SkinLua, path_expression: &str, mode: &str) -> (bool, String, i64) {
    eval(
        runtime,
        &format!("local file, message = io.open({path_expression}, '{mode}') return file == nil, message, select('#', io.open({path_expression}, '{mode}'))"),
    )
}

#[test]
fn a_read_looks_in_the_overlay_before_the_root() {
    let scratch = Scratch::new("overlay-first");
    scratch.write("overlay/data/shadowed.txt", b"from the overlay\n");
    scratch.write("overlay/data/added.txt", b"only in the overlay\n");
    let runtime = skin(&fixture_root(), Some(&scratch.overlay()));
    let root = runtime.paths().root().to_string_lossy().replace('\\', "/");

    assert_eq!(read_all(&runtime, "data/shadowed.txt"), "from the overlay\n", "the overlay's copy hides the root's");
    assert_eq!(read_all(&runtime, "data/added.txt"), "only in the overlay\n", "a file the root never had is read from the overlay");
    assert_eq!(read_all(&runtime, "data/notes.txt").into_bytes(), fixture("data/notes.txt"), "a file the overlay lacks is read from the root");
    assert_eq!(read_all(&runtime, &format!("{root}/data/shadowed.txt")), "from the overlay\n", "an absolute name under the root is the same file");
    assert_eq!(read_all(&runtime, "data\\\\shadowed.txt"), "from the overlay\n", "a backslash separates as a slash does");
    assert_eq!(fixture("data/shadowed.txt"), b"from the root\n".to_vec());
}

#[test]
fn a_write_lands_in_the_overlay_and_leaves_the_root_alone() {
    let scratch = Scratch::new("write-overlay");
    let overlay = scratch.overlay();
    let before = snapshot(&fixture_root());
    let runtime = skin(&fixture_root(), Some(&overlay));
    runtime.lua().globals().set("ROOT", runtime.paths().root().to_string_lossy().replace('\\', "/")).expect("the global is set");

    let (information, made, notes, shadowed): (String, bool, i64, String) = eval(
        &runtime,
        r#"
        local files = require('files')
        files.overwrite('History/today/information.txt', 'now playing\n')
        files.overwrite('data/shadowed.txt', 'rewritten\n')
        files.touch('made/empty.txt')
        files.reset('data/notes.txt')
        files.overwrite(ROOT .. '/absolute/named.txt', 'by absolute path\n')
        files.overwrite('back\\slash.txt', 'windows\n')
        return files.lines('History/today/information.txt')[1], files.exists('made/empty.txt'), files.count('data/notes.txt', 0), files.lines('data/shadowed.txt')[1]
        "#,
    );
    assert_eq!((information.as_str(), made, notes, shadowed.as_str()), ("now playing", true, 0, "rewritten"), "the skin reads back what it wrote");

    assert_eq!(snapshot(&fixture_root()), before, "writing changed the skin root");
    let written = snapshot(&overlay);
    let expected: BTreeMap<PathBuf, Vec<u8>> = [
        ("History/today/information.txt", &b"now playing\n"[..]),
        ("data/shadowed.txt", b"rewritten\n"),
        ("made/empty.txt", b""),
        ("data/notes.txt", b""),
        ("absolute/named.txt", b"by absolute path\n"),
        ("back/slash.txt", b"windows\n"),
    ]
    .into_iter()
    .map(|(path, bytes)| (PathBuf::from(path), bytes.to_vec()))
    .collect();
    assert_eq!(written, expected, "every write landed in the overlay at the same relative path");
}

#[test]
fn appending_copies_the_roots_file_into_the_overlay_first() {
    let scratch = Scratch::new("append-copy");
    let overlay = scratch.overlay();
    let before = snapshot(&fixture_root());
    let runtime = skin(&fixture_root(), Some(&overlay));

    eval::<()>(&runtime, "require('files').append('data/log.txt', 'three\\n')");
    let mut expected = fixture("data/log.txt");
    expected.extend_from_slice(b"three\n");
    assert_eq!(std::fs::read(overlay.join("data/log.txt")).expect("the copy exists"), expected, "the copy starts as the root's file");

    eval::<()>(&runtime, "require('files').append('data/log.txt', 'four\\n')");
    expected.extend_from_slice(b"four\n");
    assert_eq!(std::fs::read(overlay.join("data/log.txt")).expect("the copy exists"), expected, "a second append adds to the copy, not to a new one");
    assert_eq!(read_all(&runtime, "data/log.txt").into_bytes(), expected);

    eval::<()>(&runtime, "local file = io.open('data/notes.txt', 'r+') file:write('FIRST') file:close()");
    let mut updated = fixture("data/notes.txt");
    updated[..5].copy_from_slice(b"FIRST");
    assert_eq!(std::fs::read(overlay.join("data/notes.txt")).expect("the copy exists"), updated, "updating in place also works on a copy");

    eval::<()>(&runtime, "require('files').append('fresh/list.txt', 'x\\n')");
    assert_eq!(std::fs::read(overlay.join("fresh/list.txt")).expect("the file exists"), b"x\n", "appending to a file that is nowhere creates it");

    assert_eq!(snapshot(&fixture_root()), before, "appending changed the skin root");
}

#[test]
fn a_skin_keeps_its_rotation_lists_in_the_overlay() {
    let scratch = Scratch::new("rotation");
    let before = snapshot(&fixture_root());
    let runtime = skin(&fixture_root(), Some(&scratch.overlay()));

    let picks: Vec<String> = eval(
        &runtime,
        r#"
        local files = require('files')
        local picks = {}
        for turn = 1, 5 do
            picks[turn] = files.rotate('state/rotation', { 'a.png', 'b.png' })
        end
        return picks
        "#,
    );
    assert_eq!(picks, ["a.png", "b.png", "a.png", "b.png", "a.png"], "the used list is read back, filled and reset");
    assert_eq!(std::fs::read(scratch.overlay().join("state/rotation/pathList.txt")).expect("the list exists"), b"a.png\nb.png\n");
    assert_eq!(snapshot(&fixture_root()), before, "the rotation changed the skin root");
}

#[test]
fn a_name_outside_the_root_is_refused() {
    let scratch = Scratch::new("escape");
    scratch.write("skin/inside.txt", b"inside\n");
    let outside = scratch.write("outside/secret.txt", b"secret\n");
    let overlay = scratch.overlay();
    let runtime = skin(&scratch.path().join("skin"), Some(&overlay));
    runtime.lua().globals().set("OUTSIDE", outside.to_string_lossy().into_owned()).expect("the global is set");

    let names = ["OUTSIDE", "'../outside/secret.txt'", "'sub/../../outside/secret.txt'", "'..\\\\outside\\\\secret.txt'", "'../outside/created.txt'"];
    for name in names {
        for mode in ["r", "w", "a", "r+"] {
            let (refused, message, values) = open_outcome(&runtime, name, mode);
            assert!(refused && message.starts_with(ACCESS_DENIED), "{name} in mode {mode} answered {message}");
            assert_eq!(values, 2, "a refusal is nil and a message");
        }
    }
    for name in ["'.'", "''", "'sub/..'"] {
        let (refused, message, _) = open_outcome(&runtime, name, "w");
        assert!(refused && message.starts_with(FAILURE_PREFIX), "the root itself is not a file to write: {message}");
    }
    assert!(eval::<bool>(&runtime, "return not pcall(io.lines, '../outside/secret.txt')"), "io.lines has no way to answer and raises");

    assert_eq!(snapshot(&scratch.path().join("outside")), BTreeMap::from([(PathBuf::from("secret.txt"), b"secret\n".to_vec())]));
    assert!(!overlay.exists(), "a refused write created the overlay");
    assert_eq!(read_all(&runtime, "inside.txt"), "inside\n");
}

#[cfg(unix)]
#[test]
fn a_symbolic_link_out_of_the_root_is_refused() {
    let scratch = Scratch::new("symlink-escape");
    scratch.write("skin/real/kept.txt", b"kept\n");
    scratch.write("outside/secret.txt", b"secret\n");
    let skin_root = scratch.path().join("skin");
    std::os::unix::fs::symlink(scratch.path().join("outside").join("secret.txt"), skin_root.join("leak.txt")).expect("the link is created");
    std::os::unix::fs::symlink(scratch.path().join("outside"), skin_root.join("out")).expect("the link is created");
    std::os::unix::fs::symlink(skin_root.join("real"), skin_root.join("alias")).expect("the link is created");
    let overlay = scratch.overlay();
    let runtime = skin(&skin_root, Some(&overlay));

    for (name, mode) in
        [("'leak.txt'", "r"), ("'leak.txt'", "a"), ("'leak.txt'", "w"), ("'out/secret.txt'", "r"), ("'out/secret.txt'", "r+"), ("'out/created.txt'", "w")]
    {
        let (refused, message, _) = open_outcome(&runtime, name, mode);
        assert!(refused && message.starts_with(ACCESS_DENIED), "{name} in mode {mode} answered {message}");
    }
    assert_eq!(snapshot(&scratch.path().join("outside")), BTreeMap::from([(PathBuf::from("secret.txt"), b"secret\n".to_vec())]));
    assert!(!overlay.exists(), "a refused write created the overlay");

    assert_eq!(read_all(&runtime, "alias/kept.txt"), "kept\n", "a link that stays inside the root is followed");
    eval::<()>(&runtime, "io.open('alias/kept.txt', 'a'):write('more\\n'):close()");
    assert_eq!(std::fs::read(overlay.join("alias/kept.txt")).expect("the copy exists"), b"kept\nmore\n");
    assert_eq!(std::fs::read(skin_root.join("real/kept.txt")).expect("the original exists"), b"kept\n");
}

#[test]
fn a_missing_file_answers_nil_and_a_message() {
    let scratch = Scratch::new("missing");
    let runtime = skin(&fixture_root(), Some(&scratch.overlay()));

    assert_eq!(open_outcome(&runtime, "'data/absent.txt'", "r"), (true, format!("{NOT_FOUND}data/absent.txt"), 2));
    assert_eq!(open_outcome(&runtime, "'data/absent.txt'", "r+"), (true, format!("{NOT_FOUND}data/absent.txt"), 2));
    assert_eq!(open_outcome(&runtime, "'data'", "r"), (true, format!("{NOT_FOUND}data"), 2), "a directory is not a file");
    assert!(eval::<bool>(&runtime, "return io.open('data/absent.txt') == nil and io.open('data/absent.txt', 'r') == nil"));
    assert_eq!(
        eval::<(bool, bool)>(&runtime, "local files = require('files') return files.exists('data/absent.txt'), files.exists('data/notes.txt')"),
        (false, true)
    );

    let (ok, message): (bool, String) = eval(&runtime, "return pcall(io.lines, 'data/absent.txt')");
    assert!(!ok && message.contains(NOT_FOUND), "io.lines raises what io.open answers: {message}");
    let (ok, message): (bool, String) = eval(&runtime, "return pcall(io.open, 'data/notes.txt', 'x')");
    assert!(!ok && message.contains("invalid mode"), "a mode that is not one is raised: {message}");
    assert!(eval::<bool>(&runtime, "return not pcall(io.open, 'data/notes.txt', 'rb+') and not pcall(io.open, 'data/notes.txt', '') and not pcall(io.open)"));
    assert!(eval::<bool>(&runtime, "local file = io.open('data/notes.txt', 'rb') local open = io.type(file) == 'file' file:close() return open"));
    assert!(!scratch.overlay().exists(), "reading created the overlay");
}

#[test]
fn each_read_format_reads_its_own_way() {
    let scratch = Scratch::new("formats");
    scratch.write("skin/formats.txt", b"12 -3.5\nsecond line\nthird\n");
    scratch.write("skin/only.txt", b"only\n");
    let runtime = skin(&scratch.path().join("skin"), None);

    let read: (f64, f64, String, String, String, String, bool, String, bool) = eval(
        &runtime,
        r#"
        local file = io.open('formats.txt')
        local first, second = file:read('*n', '*n')
        local rest = file:read('*l')
        local line = file:read('l')
        local three = file:read(3)
        local whole = file:read('*L')
        local nothing = file:read('*l')
        local all_at_end = file:read('*a')
        local zero_at_end = file:read(0)
        file:close()
        return first, second, rest, line, three, whole, nothing == nil, all_at_end, zero_at_end == nil
        "#,
    );
    assert_eq!(read, (12.0, -3.5, String::new(), "second line".to_owned(), "thi".to_owned(), "rd\n".to_owned(), true, String::new(), true));

    assert_eq!(read_all(&runtime, "formats.txt"), "12 -3.5\nsecond line\nthird\n");
    let read: (String, String, String, String, f64) = eval(
        &runtime,
        r#"
        local file = io.open('formats.txt')
        local default = file:read()
        local zero = file:read(0)
        local counted = file:read(6.9)
        local all = file:read('a')
        file:close()
        return default, zero, counted, all, #all
        "#,
    );
    assert_eq!(read, ("12 -3.5".to_owned(), String::new(), "second".to_owned(), " line\nthird\n".to_owned(), 12.0));

    let stops: (i64, i64, bool) = eval(
        &runtime,
        r#"
        local file = io.open('only.txt')
        local some = select('#', file:read('*l', '*l', '*l'))
        local none = select('#', file:read('*l', '*x'))
        local reached = pcall(file.read, file, '*x')
        file:close()
        return some, none, reached
        "#,
    );
    assert_eq!(stops, (2, 1, false), "a read stops at the first item the file cannot give, before looking at the formats after it");

    for bad in ["'*x'", "'x'", "'*'", "''", "{}", "true", "-1"] {
        let (ok, message): (bool, String) = eval(&runtime, &format!("local file = io.open('formats.txt') return pcall(file.read, file, {bad})"));
        assert!(!ok && message.contains("bad argument #1 to 'read'"), "{bad} answered {message}");
    }
}

#[test]
fn a_number_is_read_only_when_something_follows_it() {
    let scratch = Scratch::new("numbers");
    scratch.write("skin/followed.txt", b"  42\n");
    scratch.write("skin/last.txt", b"42");
    scratch.write("skin/word.txt", b"abc\n");
    scratch.write("skin/empty.txt", b"");
    let runtime = skin(&scratch.path().join("skin"), None);

    assert_eq!(eval::<f64>(&runtime, "return io.open('followed.txt'):read('*n')"), 42.0);
    assert_eq!(eval::<f64>(&runtime, "return io.open('followed.txt'):read('n')"), 42.0);
    for name in ["last.txt", "empty.txt"] {
        let (missing, message, values): (bool, String, i64) = eval(
            &runtime,
            &format!(
                "local file = io.open('{name}') local value, message = file:read('*n') return value == nil, message, select('#', io.open('{name}'):read('*n'))"
            ),
        );
        assert!(missing && message.starts_with(FAILURE_PREFIX) && values == 2, "{name} answered {message}");
    }
    let (values, next): (i64, String) = eval(&runtime, "local file = io.open('word.txt') return select('#', file:read('*n')), file:read('*l')");
    assert_eq!((values, next.as_str()), (1, "abc"), "what is not a number is left where it was and answered as a lone nil");
}

#[test]
fn reading_a_line_drops_carriage_returns() {
    let scratch = Scratch::new("carriage-returns");
    scratch.write("skin/windows.txt", b"one\r\ntwo\r\n\r\nfo\rur\r");
    scratch.write("skin/trailing.txt", b"x\r\n\r");
    scratch.write("skin/unix.txt", b"one\ntwo\n\nfour");
    let runtime = skin(&scratch.path().join("skin"), None);
    let expected = ["one", "two", "", "four"];

    assert_eq!(eval::<Vec<String>>(&runtime, "local found = {} for line in io.lines('windows.txt') do found[#found + 1] = line end return found"), expected);
    assert_eq!(
        eval::<Vec<String>>(&runtime, "local found = {} for line in io.open('windows.txt'):lines() do found[#found + 1] = line end return found"),
        expected
    );
    assert_eq!(eval::<Vec<String>>(&runtime, "local found = {} for line in io.lines('unix.txt') do found[#found + 1] = line end return found"), expected);
    assert_eq!(
        eval::<Vec<String>>(&runtime, "local file, found = io.open('windows.txt'), {} for index = 1, 4 do found[index] = file:read('*l') end return found"),
        expected
    );
    assert_eq!(eval::<Vec<String>>(&runtime, "local found = {} for line in io.lines('trailing.txt') do found[#found + 1] = line end return found"), ["x"]);
    assert_eq!(eval::<String>(&runtime, "return io.open('windows.txt'):read('*L')"), "one\r\n", "the whole-line format keeps what ended the line");
    assert_eq!(read_all(&runtime, "windows.txt"), "one\r\ntwo\r\n\r\nfo\rur\r", "reading everything keeps every byte");
}

#[test]
fn io_lines_closes_its_file_and_file_lines_does_not() {
    let runtime = skin(&fixture_root(), None);

    let outcome: (i64, bool, String, bool, String, bool) = eval(
        &runtime,
        r#"
        local seen = 0
        local step = io.lines('data/notes.txt')
        for _ in step do
            seen = seen + 1
        end
        local ok, message = pcall(step)
        local file = io.open('data/notes.txt')
        for _ in file:lines() do
        end
        local still_open = io.type(file) == 'file'
        file:close()
        return seen, ok, message, still_open, io.type(file), io.type(42) == nil and io.type(nil) == nil and io.type({}) == nil
        "#,
    );
    assert_eq!(outcome.0, 3);
    assert!(!outcome.1 && outcome.2.contains("file is already closed"), "the iterator of io.lines closed its file: {}", outcome.2);
    assert_eq!((outcome.3, outcome.4.as_str(), outcome.5), (true, "closed file", true));

    let chunks: Vec<String> =
        eval(&runtime, "local found = {} for chunk in io.lines('files.lua', 5) do found[#found + 1] = chunk if #found == 2 then break end end return found");
    assert_eq!(chunks.concat().into_bytes(), fixture("files.lua")[..10].to_vec(), "lines passes its formats to every read");
    assert!(eval::<bool>(&runtime, "local file = io.open('data/notes.txt') file:close() return not pcall(file.lines, file)"));
}

#[test]
fn a_file_seeks_and_mixes_reads_with_writes() {
    let scratch = Scratch::new("seek");
    let overlay = scratch.overlay();
    let runtime = skin(&fixture_root(), Some(&overlay));

    let outcome: (f64, f64, String, f64, f64, bool, bool, bool, bool) = eval(
        &runtime,
        r#"
        local file = io.open('seek.txt', 'w')
        local same = file:write('0123', 456, '789') == file
        local at_end = file:seek()
        local moved = file:seek('set', 2)
        local three = file:read(3)
        file:write('XY')
        local floor = file:seek('cur', -100)
        local size = file:seek('end')
        local flushed = file:flush() == true and file:setvbuf('no') == true and file:setvbuf('full', 1024) == true
        local closed = file:close()
        return at_end, moved, three, floor, size, same, flushed, closed, tostring(file):sub(1, 4) == 'file'
        "#,
    );
    assert_eq!(outcome, (10.0, 2.0, "234".to_owned(), 0.0, 10.0, true, true, true, true));
    assert_eq!(std::fs::read(overlay.join("seek.txt")).expect("the file exists"), b"01234XY789", "a write after a read lands where the read stopped");

    let appended: (f64, String) =
        eval(&runtime, "local file = io.open('seek.txt', 'a+') local start = file:seek() file:write('!') file:seek('set') return start, file:read('*a')");
    assert_eq!(appended, (10.0, "01234XY789!".to_owned()), "an appending file starts at its end and can be read back");

    for call in ["file:read()", "file:write('x')", "file:seek()", "file:flush()", "file:close()", "file:lines()", "file:setvbuf('no')", "io.close(file)"] {
        let (ok, message): (bool, String) =
            eval(&runtime, &format!("local file = io.open('seek.txt') file:close() return pcall(function() return {call} end)"));
        assert!(!ok && message.contains("attempt to use a closed file"), "{call} on a closed file answered {message}");
    }
    assert!(eval::<bool>(&runtime, "local file = io.open('seek.txt') return not pcall(file.seek, file, 'middle') and not pcall(file.setvbuf, file, 'some')"));
    let (refused, message): (bool, String) =
        eval(&runtime, "local file = io.open('seek.txt') local written, message = file:write('x') return written == nil, message");
    assert!(refused && message.starts_with(FAILURE_PREFIX), "a file opened for reading cannot be written: {message}");
    assert!(eval::<bool>(&runtime, "return not pcall(function() return io.open('seek.txt', 'w'):write({}) end)"));
}

#[test]
fn the_standard_streams_are_empty_input_and_the_log() {
    let scratch = Scratch::new("standard");
    let overlay = scratch.overlay();
    let runtime = skin(&fixture_root(), Some(&overlay));

    let outcome: (bool, String, bool, String, String, String, bool) = eval(
        &runtime,
        r#"
        local nothing = io.read()
        local all = io.read('*a')
        local returned = io.write('hello ', 42, '\n', 'second line')
        io.stderr:write('warned\r\n')
        local _, closing = io.close()
        local _, closing_error = io.stderr:close()
        return nothing == nil, all, returned == io.stdout, io.type(io.stdin), closing, closing_error,
            io.output() == io.stdout and io.input() == io.stdin and io.flush() == true and io.lines()() == nil
        "#,
    );
    assert_eq!(outcome, (true, String::new(), true, "file".to_owned(), "cannot close standard file".to_owned(), "cannot close standard file".to_owned(), true));
    let printed: Vec<String> = runtime.diagnostics().prints.into_iter().map(|line| line.text).collect();
    assert_eq!(printed, ["hello 42", "second line", "warned"], "what goes to the standard output and error is logged line by line");

    let redirected: (String, String, bool) = eval(
        &runtime,
        r#"
        local target = io.output('out/redirected.txt')
        io.write('to a file\n', 'and more\n')
        io.close()
        io.output(io.stdout)
        io.input('out/redirected.txt')
        local first = io.read()
        local second
        for line in io.lines() do
            second = line
        end
        io.close(io.input())
        io.input(io.stdin)
        return first, second, io.type(target) == 'closed file' and not pcall(io.output, 'no/../../escape.txt') and not pcall(io.input, 'absent.txt')
        "#,
    );
    assert_eq!(redirected, ("to a file".to_owned(), "and more".to_owned(), true));
    assert_eq!(std::fs::read(overlay.join("out/redirected.txt")).expect("the file exists"), b"to a file\nand more\n");
    assert_eq!(runtime.diagnostics().prints.len(), 3, "a redirected write is not logged");
}

#[test]
fn a_skin_holds_only_so_many_files_open_and_dropped_ones_do_not_count() {
    let scratch = Scratch::new("open-files");
    scratch.write("skin/data.txt", b"kept\n");
    let overlay = scratch.overlay();
    let runtime = skin(&scratch.path().join("skin"), Some(&overlay));

    let (held, message): (usize, String) = eval(
        &runtime,
        "held = {} while true do local file, message = io.open('data.txt') if not file then return #held, message end held[#held + 1] = file end",
    );
    assert_eq!(held, OPEN_FILE_LIMIT);
    assert_eq!(message, "io error: too many open files");
    let (refused, created): (bool, bool) = eval(&runtime, "return io.open('made.txt', 'w') == nil, io.tmpfile() ~= nil");
    assert!(refused && !created, "a refused open creates nothing");
    assert!(!overlay.exists(), "a refused open for writing left something behind");
    assert!(eval::<bool>(&runtime, "return not pcall(io.lines, 'data.txt')"), "io.lines has nowhere to answer and raises");

    assert!(
        eval::<bool>(&runtime, "held[1]:close() local file = io.open('data.txt') return file ~= nil and io.open('data.txt') == nil"),
        "a closed file gives its place up"
    );
    assert_eq!(eval::<String>(&runtime, "held = nil return io.open('data.txt'):read('*l')"), "kept", "files the skin let go of are collected to make room");

    let opened: usize =
        eval(&runtime, &format!("local opened = 0 for _ = 1, {LEAKED_OPENS} do if io.open('data.txt') then opened = opened + 1 end end return opened"));
    assert_eq!(opened, LEAKED_OPENS, "asking whether a file exists by opening it, and never closing it, keeps working");
    let lines: usize =
        eval(&runtime, &format!("local lines = 0 for _ = 1, {LEAKED_OPENS} do for _ in io.lines('data.txt') do lines = lines + 1 end end return lines"));
    assert_eq!(lines, LEAKED_OPENS, "io.lines closes each file it finishes");
}

#[test]
fn without_an_overlay_every_write_fails_and_nothing_is_created() {
    let scratch = Scratch::new("read-only");
    scratch.write("skin/kept.txt", b"kept\n");
    let skin_root = scratch.path().join("skin");
    let before = snapshot(scratch.path());
    let runtime = skin(&skin_root, None);

    for mode in ["w", "a", "r+", "w+", "a+"] {
        for name in ["'kept.txt'", "'made/new.txt'"] {
            let (refused, message, values) = open_outcome(&runtime, name, mode);
            assert!(refused && message.starts_with(FAILURE_PREFIX) && values == 2, "{name} in mode {mode} answered {message}");
        }
    }
    let (refused, message): (bool, String) = eval(&runtime, "local file, message = io.tmpfile() return file == nil, message");
    assert!(refused && message.starts_with(FAILURE_PREFIX), "got {message}");
    assert!(eval::<bool>(&runtime, "return not pcall(io.output, 'made/new.txt')"));
    assert_eq!(read_all(&runtime, "kept.txt"), "kept\n", "reading still works");
    assert_eq!(snapshot(scratch.path()), before, "a refused write left something behind");
}

#[test]
fn a_temporary_file_lives_in_the_overlay_and_no_program_is_run() {
    let scratch = Scratch::new("tmpfile");
    let overlay = scratch.overlay();
    let before = snapshot(&fixture_root());
    let runtime = skin(&fixture_root(), Some(&overlay));

    let (text, refused, message, values): (String, bool, String, i64) = eval(
        &runtime,
        r#"
        local file = io.tmpfile()
        file:write('scratch')
        file:seek('set')
        local text = file:read('*a')
        file:close()
        local program, message = io.popen('ls')
        return text, program == nil, message, select('#', io.popen('ls', 'w'))
        "#,
    );
    assert_eq!((text.as_str(), refused, message.as_str(), values), ("scratch", true, "io error: Lua io.popen is not allowed", 2));
    assert!(eval::<bool>(&runtime, "return not pcall(io.popen, 'ls', 'x') and not pcall(io.popen)"));

    let created = snapshot(&overlay);
    assert_eq!(created.len(), 1, "one temporary file: {:?}", created.keys());
    let (name, bytes) = created.iter().next().expect("there is one file");
    let name = name.to_string_lossy();
    assert!(name.starts_with("lua-") && name.ends_with(".tmp") && bytes == b"scratch", "got {name}");
    assert_eq!(snapshot(&fixture_root()), before);
}

#[test]
fn both_libraries_answer_require() {
    let runtime = skin(&fixture_root(), None);
    assert!(eval::<bool>(&runtime, "return require('io') == io and require('os') == os and package.loaded.io == io and package.loaded.os == os"));
    let names: Vec<String> = eval(&runtime, "local names = {} for name in pairs(io) do names[#names + 1] = name end table.sort(names) return names");
    assert_eq!(names, ["close", "flush", "input", "lines", "open", "output", "popen", "read", "stderr", "stdin", "stdout", "tmpfile", "type", "write"]);
}

#[test]
fn os_is_the_clock_and_the_calendar_and_nothing_else() {
    let runtime = skin(&fixture_root(), None);

    let names: Vec<String> = eval(&runtime, "local names = {} for name in pairs(os) do names[#names + 1] = name end table.sort(names) return names");
    assert_eq!(names, ["clock", "date", "difftime", "setlocale", "time"]);
    for name in ["execute", "exit", "getenv", "remove", "rename", "tmpname"] {
        assert!(eval::<bool>(&runtime, &format!("return os.{name} == nil")), "os.{name} should not exist");
    }

    let loaded: mlua::Table = runtime.lua().named_registry_value(NATIVE_LOADED_TABLE).expect("the C libraries keep a table of what they loaded");
    let native: mlua::Table = loaded.get("os").expect("the C library registered itself");
    assert!(native.is_empty(), "the C library's own table still holds functions");

    let now: f64 = eval(&runtime, "return os.time()");
    let clock = SystemTime::now().duration_since(UNIX_EPOCH).expect("the clock is past the epoch").as_secs_f64();
    assert!((clock - now).abs() < CLOCK_TOLERANCE_SECONDS, "os.time() answered {now} at {clock}");

    assert_eq!(eval::<String>(&runtime, "return os.date('!%Y-%m-%d %H:%M:%S', 0)"), "1970-01-01 00:00:00");
    assert_eq!(eval::<String>(&runtime, "return os.date('!%y%m%d', 86400)"), "700102");
    let fields: (i64, i64, i64, i64, i64, i64, i64, i64, bool) =
        eval(&runtime, "local t = os.date('!*t', 86400) return t.year, t.month, t.day, t.hour, t.min, t.sec, t.wday, t.yday, t.isdst");
    assert_eq!(fields, (1970, 1, 2, 0, 0, 0, 6, 2, false), "the second day of 1970 was a Friday");

    runtime.lua().globals().set("MOMENT", SAMPLE_MOMENT).expect("the global is set");
    assert_eq!(eval::<i64>(&runtime, "return os.time(os.date('*t', MOMENT))"), SAMPLE_MOMENT, "a local date table converts back to its moment");
    assert!(eval::<bool>(
        &runtime,
        r#"
        local t = os.date('*t', MOMENT)
        return os.date('%Y/%m/%d %H:%M:%S', MOMENT) == string.format('%04d/%02d/%02d %02d:%02d:%02d', t.year, t.month, t.day, t.hour, t.min, t.sec)
            and os.date('%y%m%d', MOMENT) == string.format('%02d%02d%02d', t.year % 100, t.month, t.day)
        "#
    ));
    assert!(eval::<bool>(&runtime, "return os.date('%Y-%m-%d'):match('^%d%d%d%d%-%d%d%-%d%d$') ~= nil and type(os.date('*t').wday) == 'number'"));
    assert!(eval::<bool>(&runtime, "return os.date('%H:%M:%S', os.time() - 3600 * 6):match('^%d%d:%d%d:%d%d$') ~= nil"));
    assert_eq!(
        eval::<i64>(
            &runtime,
            "return os.time({ year = 2001, month = 9, day = 9, hour = 1, min = 46, sec = 40 }) - os.time({ year = 2001, month = 9, day = 9, hour = 1 })"
        ),
        2800
    );
    assert!(eval::<bool>(&runtime, "return not pcall(os.time, 'now') and not pcall(os.time, {})"));

    assert_eq!(eval::<f64>(&runtime, "return os.difftime(5, 3)"), 2.0);
    assert_eq!(
        eval::<(String, String, String)>(&runtime, "return os.setlocale(), os.setlocale('ja_JP.UTF-8'), os.setlocale(nil, 'numeric')"),
        ("C".to_owned(), "C".to_owned(), "C".to_owned())
    );
    assert_eq!(eval::<String>(&runtime, "return tostring(1.5)"), "1.5", "asking for a locale changed none");
    assert!(eval::<bool>(&runtime, "local first = os.clock() local second = os.clock() return type(first) == 'number' and first >= 0 and second >= first"));
}

#[test]
fn an_external_skin_pack_is_read_and_written_without_touching_its_folder() {
    let Some(pack) = std::env::var_os(SKIN_PACK_ENV).map(PathBuf::from) else {
        return;
    };
    let scratch = Scratch::new("pack-overlay");
    let overlay = scratch.overlay();
    let before = listing(&pack);
    let (sample, _) = before
        .iter()
        .filter(|(path, (size, _))| (1..PACK_SAMPLE_MAX_BYTES).contains(size) && path.to_str().is_some_and(|path| !path.contains(['\'', '\\'])))
        .min_by_key(|(_, (size, _))| *size)
        .expect("the pack holds a small file");
    let name = sample.to_string_lossy().replace(std::path::MAIN_SEPARATOR, "/");
    let original = std::fs::read(pack.join(sample)).expect("the sample should be readable");
    let runtime = skin(&pack, Some(&overlay));
    runtime.lua().globals().set("SAMPLE", name.clone()).expect("the global is set");

    let text: mlua::LuaString = eval(&runtime, "local file = io.open(SAMPLE) local text = file:read('*a') file:close() return text");
    assert_eq!(text.as_bytes().to_vec(), original, "{name} reads as it is on disk");
    eval::<()>(
        &runtime,
        r#"
        io.open(SAMPLE, 'a'):write('appended\n'):close()
        local file = io.open('History/information.txt', 'w')
        file:write('written\n')
        file:close()
        "#,
    );
    let mut appended = original.clone();
    appended.extend_from_slice(b"appended\n");
    assert_eq!(std::fs::read(overlay.join(sample)).expect("the copy exists"), appended);
    assert_eq!(std::fs::read(overlay.join("History/information.txt")).expect("the file exists"), b"written\n");
    assert_eq!(std::fs::read(pack.join(sample)).expect("the sample should be readable"), original, "{name} changed in the pack");
    if pack.join(PACK_TIME_MODULE_FILE).is_file() {
        assert!(
            eval::<bool>(
                &runtime,
                r#"
                local time = require('Root.customtime')
                return time.DATE:match('^%d%d%d%d%d%d$') ~= nil and time.DATE2:match('^%d%d%d%d/%d%d/%d%d$') ~= nil and time.TIME:match('^%d%d:%d%d:%d%d$') ~= nil
                "#
            ),
            "the pack's own date module reads the calendar"
        );
    }
    assert_eq!(listing(&pack), before, "running against the pack changed its folder");
}
