//! The `luajava` facade a `.luaskin` is given: the classes it may bind and the ones it may not, the
//! `java.io.File` that reads through the write overlay and writes nowhere else, the window and key
//! questions that reach whichever host is bound when they are asked, the libGDX key codes, the
//! controllers that are not there, and the URL that is never connected.
//!
//! Every directory these tests build is made on the spot in a scratch directory, so no checkout
//! setting can change what a listing shows. The one test that runs a real skin pack is optional: it
//! needs `RBMS_SKIN_PACK` and passes silently without it.

#![cfg(feature = "lua")]

use std::collections::BTreeMap;
use std::io::ErrorKind;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use mlua::{FromLuaMulti, Function};
use rbms_skin::lua::{LuaFnKind, LuaMode, SkinLua, SkinLuaConfig};
use rbms_skin::property::MapHost;

/// The seed every interpreter of these tests is pinned to.
const TEST_SEED: u64 = 11;

/// The environment variable that names an external skin pack for the optional test.
const SKIN_PACK_ENV: &str = "RBMS_SKIN_PACK";

/// What binding a class that is not on the list is refused with.
const CLASS_DENIED: &str = "Legacy Lua skin class access denied: ";

/// What constructing anything that is not on the list is refused with.
const CONSTRUCTOR_DENIED: &str = "Legacy Lua skin constructor access denied";

/// What a path outside the skin root is refused with.
const PATH_DENIED: &str = "skin file access denied: ";

/// What every attempt to reach the network is refused with.
const CONNECTION_REFUSED: &str = "Legacy Lua skin HTTP connection failed: network access is not available to skins";

/// The classes `luajava.bindClass` accepts.
const ALLOWED_CLASSES: [&str; 5] =
    ["java.io.File", "com.badlogic.gdx.Gdx", "com.badlogic.gdx.Input", "com.badlogic.gdx.controllers.Controllers", "com.badlogic.gdx.controllers.Controller"];

/// Classes a skin might ask for to reach the machine, and near misses of the ones it may have.
const REFUSED_CLASSES: [&str; 9] = [
    "java.lang.System",
    "java.lang.Runtime",
    "java.lang.ProcessBuilder",
    "java.io.FileWriter",
    "java.io.FileInputStream",
    "java.net.Socket",
    "java.net.URL",
    "java.io.file",
    "com.badlogic.gdx.Gdx ",
];

/// Constants of libGDX's `Input.Keys`, with the values the reference's `gdx.jar` holds.
const KEY_CODES: [(&str, i32); 24] = [
    ("UP", 19),
    ("DOWN", 20),
    ("LEFT", 21),
    ("RIGHT", 22),
    ("A", 29),
    ("Z", 54),
    ("NUM_0", 7),
    ("NUM_9", 16),
    ("NUMPAD_5", 149),
    ("SPACE", 62),
    ("ENTER", 66),
    ("TAB", 61),
    ("ESCAPE", 131),
    ("BACKSPACE", 67),
    ("DEL", 67),
    ("SHIFT_LEFT", 59),
    ("ALT_RIGHT", 58),
    ("CONTROL_LEFT", 129),
    ("PAGE_UP", 92),
    ("F1", 244),
    ("F12", 255),
    ("BUTTON_START", 108),
    ("META_SHIFT_ON", 1),
    ("ANY_KEY", -1),
];

/// The code a name that is no key at all answers.
const UNKNOWN_KEY: i32 = -1;

/// The code of the key whose display name is `8`.
const EIGHT_KEY: i32 = 15;

/// The keys the pack this was written against asks about.
const KEY_RIGHT: i32 = 22;
const KEY_LEFT: i32 = 21;
const KEY_UP: i32 = 19;

/// The window sizes of the two hosts a skin is asked through.
const FIRST_WINDOW: (i32, i32) = (1280, 720);
const SECOND_WINDOW: (i32, i32) = (1920, 1080);

/// A directory of the pack the optional test makes in the overlay, and the folder it is made under.
const PACK_PROBE_PARENT: &str = "io/Play/sp";
const PACK_PROBE_NAME: &str = "rbms-luajava-probe";

/// A scratch directory that removes itself.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("rbms-luajava-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("the scratch directory should be creatable");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    /// The skin root of a test: the scratch directory's `skin` folder.
    fn root(&self) -> PathBuf {
        self.0.join("skin")
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

    /// Makes an empty directory under the scratch directory.
    fn directory(&self, relative: &str) {
        std::fs::create_dir_all(self.0.join(relative)).expect("the directory should be creatable");
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A scratch directory with a skin root that holds a few files and folders, and no overlay yet.
fn scratch_skin(name: &str) -> Scratch {
    let scratch = Scratch::new(name);
    scratch.write("skin/data/a.txt", b"a\n");
    scratch.write("skin/data/b.txt", b"b\n");
    scratch.write("skin/data/sub/c.txt", b"c\n");
    scratch.directory("skin/io/Play");
    scratch
}

/// A seeded interpreter for the skin at `root`, writing to `overlay` when there is one, with the
/// skin root's spelling published as the global `ROOT`.
fn skin_in_mode(root: &Path, overlay: Option<&Path>, mode: LuaMode) -> SkinLua {
    let runtime = SkinLua::new(SkinLuaConfig { overlay: overlay.map(Path::to_path_buf), seed: Some(TEST_SEED), mode, ..SkinLuaConfig::new(root) })
        .expect("the runtime builds");
    let spelling = root_spelling(&runtime);
    runtime.lua().globals().set("ROOT", spelling).expect("the global is set");
    runtime
}

fn skin(root: &Path, overlay: Option<&Path>) -> SkinLua {
    skin_in_mode(root, overlay, LuaMode::Full)
}

/// The skin root as a skin is told it: the one spelling, with `/` for separators.
fn root_spelling(runtime: &SkinLua) -> String {
    runtime.paths().root().to_string_lossy().replace('\\', "/")
}

/// Runs a chunk in the interpreter and reads what it returns.
fn eval<T: FromLuaMulti>(runtime: &SkinLua, source: &str) -> T {
    runtime.lua().load(source).eval().unwrap_or_else(|error| panic!("{source} should run: {error}"))
}

/// A Lua function made from a chunk that returns one.
fn function(runtime: &SkinLua, source: &str) -> Function {
    eval(runtime, source)
}

/// The message of a chunk that must fail inside the skin's own `pcall`. The message is read as a
/// string, which is the proof that the skin sees a string and not an error object.
fn failure(runtime: &SkinLua, body: &str) -> String {
    let (completed, message): (bool, String) = eval(runtime, &format!("local ok, message = pcall(function() {body} end) return ok, message"));
    assert!(!completed, "{body} should fail");
    message
}

/// `luajava.new(File, path)` for the skin's `File` class, as a function of the path.
fn file_object(runtime: &SkinLua) -> Function {
    function(runtime, "local File = luajava.bindClass('java.io.File') return function(path) return luajava.new(File, path) end")
}

/// `File:mkdir()` on a path.
fn mkdir(runtime: &SkinLua) -> impl Fn(&str) -> bool {
    let make = function(runtime, "local File = luajava.bindClass('java.io.File') return function(path) return luajava.new(File, path):mkdir() end");
    move |path| make.call::<bool>(path).expect("mkdir runs")
}

/// `File:listFiles()` on a path: `None` for `nil`, the entries otherwise.
fn list_files(runtime: &SkinLua) -> impl Fn(&str) -> Option<Vec<String>> {
    let list = function(runtime, "local File = luajava.bindClass('java.io.File') return function(path) return luajava.new(File, path):listFiles() end");
    move |path| list.call::<Option<Vec<String>>>(path).expect("listFiles runs")
}

/// Everything under `root`, directories included, with what shows a change: kind, size and the
/// moment it was last changed. A directory that gained an entry changes, so this is how a tree is
/// shown to be untouched.
fn tree(root: &Path) -> BTreeMap<PathBuf, (bool, u64, SystemTime)> {
    let mut found = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).expect("the directory should be listable").flatten() {
            let path = entry.path();
            let metadata = entry.metadata().expect("the entry should be readable");
            if metadata.is_dir() {
                pending.push(path.clone());
            }
            if let Ok(relative) = path.strip_prefix(root) {
                found.insert(relative.to_path_buf(), (metadata.is_dir(), metadata.len(), metadata.modified().expect("the platform keeps modification times")));
            }
        }
    }
    found
}

#[test]
fn luajava_is_a_global_and_a_module_whatever_the_mode() {
    let scratch = scratch_skin("module");
    for mode in [LuaMode::Full, LuaMode::HeaderOnly] {
        let runtime = skin_in_mode(&scratch.root(), None, mode);
        let published: bool = eval(
            &runtime,
            r#"
            return luajava == require('luajava') and luajava == package.loaded.luajava
                and type(luajava.bindClass) == 'function' and type(luajava.new) == 'function' and type(luajava.newInstance) == 'function'
            "#,
        );
        assert!(published, "{mode:?} publishes one luajava table");
        let members: i64 = eval(&runtime, "local count = 0 for _ in pairs(luajava) do count = count + 1 end return count");
        assert_eq!(members, 3, "{mode:?} offers bindClass, new and newInstance and nothing else");
    }
}

#[test]
fn only_the_listed_classes_can_be_bound() {
    let scratch = scratch_skin("classes");
    let runtime = skin(&scratch.root(), Some(&scratch.overlay()));
    for class in ALLOWED_CLASSES {
        let kind: String = eval(&runtime, &format!("return type(luajava.bindClass('{class}'))"));
        assert_eq!(kind, "table", "{class} is bound");
    }
    for class in REFUSED_CLASSES {
        let message = failure(&runtime, &format!("return luajava.bindClass('{class}')"));
        assert!(message.contains(&format!("{CLASS_DENIED}{class}")), "{class} is refused with the reference's words: {message}");
    }
    assert!(failure(&runtime, "return luajava.bindClass(5)").contains(&format!("{CLASS_DENIED}5")), "a number is read as the name it prints as");
    for body in ["return luajava.bindClass()", "return luajava.bindClass(nil)", "return luajava.bindClass({})", "return luajava.bindClass(true)"] {
        let message = failure(&runtime, body);
        assert!(message.contains("bad argument #1 to 'bindClass' (string expected, got "), "{body}: {message}");
        assert!(!message.contains("[skin luajava]"), "{body} is reported at the skin's own line: {message}");
    }
}

#[test]
fn only_the_file_class_can_be_constructed() {
    let scratch = scratch_skin("constructors");
    let runtime = skin(&scratch.root(), Some(&scratch.overlay()));
    let refused = |body: &str, expected: &str| {
        let message = failure(&runtime, body);
        assert!(message.contains(expected), "{body}: {message}");
    };
    refused("return luajava.new('java.io.File', ROOT)", CONSTRUCTOR_DENIED);
    refused("return luajava.new(nil, ROOT)", CONSTRUCTOR_DENIED);
    refused("return luajava.new({}, ROOT)", &format!("{CONSTRUCTOR_DENIED}: null"));
    refused("return luajava.new(luajava.bindClass('com.badlogic.gdx.Gdx'), ROOT)", &format!("{CONSTRUCTOR_DENIED}: null"));
    refused(
        "return luajava.new(luajava.bindClass('com.badlogic.gdx.controllers.Controller'), ROOT)",
        &format!("{CONSTRUCTOR_DENIED}: com.badlogic.gdx.controllers.Controller"),
    );
    refused("return luajava.new(luajava.bindClass('java.io.File'))", "bad argument #2 to 'new' (string expected, got nil)");
    refused("return luajava.newInstance('java.lang.ProcessBuilder', 'ls')", &format!("{CONSTRUCTOR_DENIED}: java.lang.ProcessBuilder"));
    refused("return luajava.newInstance('java.io.File', ROOT)", &format!("{CONSTRUCTOR_DENIED}: java.io.File"));
    refused("return luajava.newInstance()", "bad argument #1 to 'newInstance' (string expected, got nil)");
    refused("return luajava.newInstance('java.net.URL')", "bad argument #2 to 'newInstance' (string expected, got nil)");
    refused("return luajava.newInstance('java.io.BufferedReader', 'not a reader')", "Legacy Lua skin reader access denied");

    let same: bool = eval(
        &runtime,
        r#"
        local reader = { readLine = function() return 'line' end }
        local wrapped = luajava.newInstance('java.io.BufferedReader', luajava.newInstance('java.io.InputStreamReader', reader, 'utf8'))
        return wrapped == reader and luajava.newInstance('java.io.InputStreamReader', reader) == reader and luajava.newInstance('java.io.InputStreamReader') == nil
        "#,
    );
    assert!(same, "the two readers hand their second argument back");
}

#[test]
fn no_controller_is_reported() {
    let scratch = scratch_skin("controllers");
    let runtime = skin(&scratch.root(), None);
    let (dot_size, dot_first, colon_size, colon_first): (i64, bool, i64, bool) = eval(
        &runtime,
        r#"
        local Controllers = luajava.bindClass('com.badlogic.gdx.controllers.Controllers')
        local dotted = Controllers.getControllers()
        local coloned = Controllers:getControllers()
        return dotted.size, dotted.first() == nil, coloned.size, coloned:first() == nil
        "#,
    );
    assert_eq!((dot_size, dot_first, colon_size, colon_first), (0, true, 0, true));
}

#[test]
fn a_directory_is_made_one_level_deep_in_the_overlay_and_never_in_the_skin_root() {
    let scratch = scratch_skin("mkdir");
    let (root, overlay) = (scratch.root(), scratch.overlay());
    let root_before = tree(&root);
    let runtime = skin(&root, Some(&overlay));
    let make = mkdir(&runtime);
    let spelling = root_spelling(&runtime);

    assert!(make(&format!("{spelling}/io/Play/sp")), "a new folder under a folder only the skin root has");
    assert!(overlay.join("io/Play/sp").is_dir(), "the folder is in the overlay, under the overlay's own copy of its parents");
    assert!(!root.join("io/Play/sp").exists(), "the skin root is not written");
    assert!(!make(&format!("{spelling}/io/Play/sp")), "a folder that exists in the overlay is not made again");
    assert!(make("io/Play/sp/deeper"), "a folder under one that only the overlay has");
    assert!(overlay.join("io/Play/sp/deeper").is_dir());
    assert!(make("io/Play/dp"), "a path relative to the skin root is read from the root and not from the working directory");
    assert!(overlay.join("io/Play/dp").is_dir());

    assert!(!make("data"), "a folder the skin root already has is not made");
    assert!(!overlay.join("data").exists(), "and nothing of it is copied");
    assert!(!make("data/a.txt"), "a file is in the way");
    assert!(!make("data/sub/"), "a trailing separator names the same folder");
    assert!(!make("io/Missing/deeper"), "the parent has to exist: one level only");
    assert!(!overlay.join("io/Missing").exists());
    assert!(!make(&spelling), "the root is already there");

    assert_eq!(tree(&root), root_before, "the skin root is untouched");
    let siblings = std::fs::read_dir(scratch.path()).expect("the scratch directory is listable").count();
    assert_eq!(siblings, 2, "only the skin root and the overlay exist");
}

#[test]
fn without_an_overlay_no_directory_is_made() {
    let scratch = scratch_skin("mkdir-readonly");
    let root = scratch.root();
    let before = tree(scratch.path());
    let runtime = skin(&root, None);
    let make = mkdir(&runtime);
    assert!(!make("io/Play/sp"));
    assert!(!make("brand-new"));
    assert_eq!(tree(scratch.path()), before, "nothing was created anywhere");
}

#[test]
fn a_path_that_leaves_the_skin_root_is_refused_when_the_object_is_made() {
    let scratch = scratch_skin("escape");
    let runtime = skin(&scratch.root(), Some(&scratch.overlay()));
    let elsewhere = scratch.path().join("elsewhere");
    std::fs::create_dir_all(&elsewhere).expect("the outside folder is creatable");
    let elsewhere_spelling = elsewhere.to_string_lossy().replace('\\', "/");
    runtime.lua().globals().set("ELSEWHERE", elsewhere_spelling.clone()).expect("the global is set");
    let make = file_object(&runtime);

    let refused = [
        ("'../elsewhere'", "../elsewhere".to_owned()),
        ("'data/../../elsewhere'", "data/../../elsewhere".to_owned()),
        ("ELSEWHERE", elsewhere_spelling),
        ("ROOT .. '/../elsewhere'", format!("{}/../elsewhere", root_spelling(&runtime))),
    ];
    for (path, named) in refused {
        let message = failure(&runtime, &format!("return luajava.new(luajava.bindClass('java.io.File'), {path})"));
        assert!(message.contains(&format!("{PATH_DENIED}{named}")), "{path}: {message}");
    }
    assert!(make.call::<mlua::Table>("data/../data/a.txt").is_ok(), "a path that returns to the root is fine");
    assert!(std::fs::read_dir(&elsewhere).expect("the outside folder is listable").next().is_none(), "nothing was created outside");
}

#[test]
fn a_listing_merges_the_overlay_over_the_skin_root_as_absolute_paths() {
    let scratch = scratch_skin("list");
    scratch.write("overlay/data/b.txt", b"shadowing b\n");
    scratch.write("overlay/data/d.txt", b"only in the overlay\n");
    scratch.directory("overlay/only-here/inside");
    let (root, overlay) = (scratch.root(), scratch.overlay());
    let before = (tree(&root), tree(&overlay));
    let runtime = skin(&root, Some(&overlay));
    let list = list_files(&runtime);
    let spelling = root_spelling(&runtime);

    let expected: Vec<String> = ["a.txt", "b.txt", "d.txt", "sub"].iter().map(|name| format!("{spelling}/data/{name}")).collect();
    assert_eq!(list("data"), Some(expected.clone()), "the two trees are one directory and a shadowed name is listed once, sorted");
    assert_eq!(list(&format!("{spelling}/data")), Some(expected), "an absolute path names the same directory");
    assert_eq!(list("data/"), list("data"), "a trailing separator changes nothing");
    assert_eq!(list("data/sub"), Some(vec![format!("{spelling}/data/sub/c.txt")]));
    assert_eq!(list("only-here"), Some(vec![format!("{spelling}/only-here/inside")]), "a directory only the overlay has");
    assert_eq!(list("only-here/inside"), Some(Vec::new()), "an empty directory is an empty array and not nil");
    let top = list(&spelling).expect("the root lists");
    assert!(top.contains(&format!("{spelling}/data")) && top.contains(&format!("{spelling}/only-here")) && top.contains(&format!("{spelling}/io")), "{top:?}");

    assert_eq!(list("nowhere"), None, "a directory that is in neither tree");
    assert_eq!(list("data/a.txt"), None, "a file is not a directory");
    assert_eq!((tree(&root), tree(&overlay)), before, "listing changes neither tree");

    let (first, zero, count): (String, bool, i64) =
        eval(&runtime, "local files = luajava.new(luajava.bindClass('java.io.File'), 'data'):listFiles() return files[1], files[0] == nil, #files");
    assert_eq!((first.as_str(), zero, count), (format!("{spelling}/data/a.txt").as_str(), true, 4), "an array that starts at 1");
}

#[test]
fn a_folder_the_skin_made_is_listed_and_can_be_listed_into() {
    let scratch = scratch_skin("made-then-listed");
    let runtime = skin(&scratch.root(), Some(&scratch.overlay()));
    let (make, list) = (mkdir(&runtime), list_files(&runtime));
    let spelling = root_spelling(&runtime);
    assert!(make("data/new-folder"));
    assert_eq!(list("data").map(|names| names.len()), Some(4), "a.txt, b.txt, new-folder and sub");
    assert!(list("data").expect("data lists").contains(&format!("{spelling}/data/new-folder")));
    assert_eq!(list("data/new-folder"), Some(Vec::new()));
    eval::<()>(&runtime, "local file = io.open('data/new-folder/note.txt', 'w') file:write('hi') file:close()");
    assert_eq!(list("data/new-folder"), Some(vec![format!("{spelling}/data/new-folder/note.txt")]), "what io wrote is listed");
}

#[test]
fn the_arrow_keys_and_the_rest_of_the_constants_answer_their_libgdx_codes() {
    let scratch = scratch_skin("keys");
    let runtime = skin(&scratch.root(), None);
    let key = function(&runtime, "local Keys = luajava.bindClass('com.badlogic.gdx.Input').Keys return function(name) return Keys[name] end");
    for (name, code) in KEY_CODES {
        assert_eq!(key.call::<i32>(name).expect("the lookup runs"), code, "Input.Keys.{name}");
    }
    let arrows: (i32, i32, i32, i32) =
        eval(&runtime, "local input = luajava.bindClass('com.badlogic.gdx.Input') return input.Keys.UP, input.Keys.DOWN, input.Keys.LEFT, input.Keys.RIGHT");
    assert_eq!(arrows, (19, 20, 21, 22), "the arrows are what the skin meant and not the reference's -1");

    assert_eq!(key.call::<i32>("Right").expect("runs"), KEY_RIGHT, "the reference's own display-name lookup still works");
    assert_eq!(key.call::<i32>("L-Ctrl").expect("runs"), 129);
    for unknown in ["right", "NOPE", "", "Input.Keys.UP"] {
        assert_eq!(key.call::<i32>(unknown).expect("runs"), UNKNOWN_KEY, "{unknown:?} is no key");
    }
    assert_eq!(key.call::<i32>(8).expect("a number is read as the name it prints as"), EIGHT_KEY, "the display name 8 is the 8 key");

    for body in ["return luajava.bindClass('com.badlogic.gdx.Input').Keys[true]", "return luajava.bindClass('com.badlogic.gdx.Input').Keys[nil]"] {
        let message = failure(&runtime, body);
        assert!(message.contains("bad argument #2 to '__index' (string expected, got "), "{body}: {message}");
        assert!(!message.contains("[skin luajava]"), "{body}: {message}");
    }
    let raw: bool =
        eval(&runtime, "local Keys = luajava.bindClass('com.badlogic.gdx.Input').Keys Keys.MINE = 5 return rawget(Keys, 'UP') == nil and Keys.MINE == 5");
    assert!(raw, "the table holds what the skin puts in it and answers the rest by lookup");
}

#[test]
fn a_key_or_window_question_is_answered_by_the_host_bound_when_it_is_asked() {
    let scratch = scratch_skin("host");
    let runtime = skin(&scratch.root(), None);
    eval::<()>(&runtime, "Gdx = luajava.bindClass('com.badlogic.gdx.Gdx') input = luajava.bindClass('com.badlogic.gdx.Input')");

    let mut right_held = MapHost::new();
    right_held.pressed_keys.insert(KEY_RIGHT);
    right_held.screen = Some(FIRST_WINDOW);
    let mut left_held = MapHost::new();
    left_held.pressed_keys.insert(KEY_LEFT);
    left_held.screen = Some(SECOND_WINDOW);

    let ask = |host: &MapHost| {
        runtime
            .with_host(host, || {
                eval::<(bool, bool, bool, bool, i32, i32)>(
                    &runtime,
                    r#"
                    return Gdx.input:isKeyPressed(input.Keys.RIGHT), Gdx.input:isKeyPressed(input.Keys.LEFT), Gdx.input.isKeyPressed(22),
                        Gdx.input:isKeyPressed('21'), Gdx.graphics:getWidth(), Gdx.graphics:getHeight()
                    "#,
                )
            })
            .expect("the host binds")
    };
    assert_eq!(ask(&right_held), (true, false, true, false, FIRST_WINDOW.0, FIRST_WINDOW.1));
    assert_eq!(ask(&left_held), (false, true, false, true, SECOND_WINDOW.0, SECOND_WINDOW.1), "the facade the skin kept answers for the host of this binding");

    let lonely = MapHost::new();
    let nothing_held: bool =
        runtime.with_host(&lonely, || eval(&runtime, "return Gdx.input:isKeyPressed(input.Keys.UP) or Gdx.input:isKeyPressed(-1)")).expect("binds");
    assert!(!nothing_held);

    let messages: Vec<String> = runtime
        .with_host(&lonely, || {
            ["Gdx.input:isKeyPressed()", "Gdx.input:isKeyPressed(nil)", "Gdx.input:isKeyPressed('Left')", "Gdx.input:isKeyPressed({})"]
                .iter()
                .map(|call| failure(&runtime, &format!("return {call}")))
                .collect()
        })
        .expect("binds");
    for message in messages {
        assert!(message.contains("bad argument to 'isKeyPressed' (number expected, got "), "{message}");
    }
}

#[test]
fn a_function_the_frame_calls_reads_the_key_of_that_frame() {
    let scratch = scratch_skin("frame");
    let runtime = skin(&scratch.root(), None);
    let draw = runtime.register(
        function(
            &runtime,
            r#"
            local Gdx = luajava.bindClass('com.badlogic.gdx.Gdx')
            local input = luajava.bindClass('com.badlogic.gdx.Input')
            return function() return Gdx.input:isKeyPressed(input.Keys.UP) end
            "#,
        ),
        LuaFnKind::Boolean,
    );
    let mut held = MapHost::new();
    held.pressed_keys.insert(KEY_UP);
    assert!(runtime.frame(&held, |frame| frame.call_boolean(draw)).expect("the frame binds"));
    assert!(!runtime.frame(&MapHost::new(), |frame| frame.call_boolean(draw)).expect("the frame binds"));
    assert!(runtime.diagnostics().function_failures.is_empty(), "the function never failed");
}

#[test]
fn without_a_host_there_is_no_window_and_no_key() {
    let scratch = scratch_skin("hostless");
    let runtime = skin_in_mode(&scratch.root(), None, LuaMode::HeaderOnly);
    let (pressed, width, height): (bool, i32, i32) = eval(
        &runtime,
        "local Gdx = luajava.bindClass('com.badlogic.gdx.Gdx') return Gdx.input:isKeyPressed(22), Gdx.graphics:getWidth(), Gdx.graphics:getHeight()",
    );
    assert_eq!((pressed, width, height), (false, 0, 0), "the header-only interpreter has no main_state functions to ask");

    let full = skin(&scratch.root(), None);
    let message = failure(&full, "return luajava.bindClass('com.badlogic.gdx.Gdx').input:isKeyPressed(22)");
    assert!(!message.is_empty(), "outside any binding main_state refuses, and the facade lets the refusal through");
}

#[test]
fn a_connection_can_be_configured_but_never_made() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port is bindable");
    listener.set_nonblocking(true).expect("the listener can be polled");
    let port = listener.local_addr().expect("the listener has an address").port();
    let scratch = scratch_skin("url");
    let runtime = skin(&scratch.root(), Some(&scratch.overlay()));
    runtime.lua().globals().set("URL", format!("http://127.0.0.1:{port}/")).expect("the global is set");

    eval::<()>(
        &runtime,
        r#"
        local url = luajava.newInstance('java.net.URL', URL)
        local connection = url:openConnection()
        connection:setRequestMethod('GET')
        connection:setConnectTimeout(200)
        "#,
    );
    for call in ["connect", "getResponseCode", "getInputStream"] {
        let message = failure(
            &runtime,
            &format!("local connection = luajava.newInstance('java.net.URL', URL):openConnection() connection:setRequestMethod('GET') connection:{call}()"),
        );
        assert!(message.contains(CONNECTION_REFUSED), "{call}: {message}");
        assert!(!message.contains("[skin luajava]"), "{call} is reported at the skin's own line: {message}");
    }
    let connected: bool =
        eval(&runtime, "local connection = luajava.newInstance('java.net.URL', URL):openConnection() return (pcall(function() connection:connect() end))");
    assert!(!connected, "a skin that guards connect() with pcall takes its failure branch");

    assert!(
        failure(&runtime, "luajava.newInstance('java.net.URL', URL):openConnection():setRequestMethod('POST')")
            .contains("Legacy Lua skin HTTP method denied: POST")
    );
    assert!(
        failure(&runtime, "luajava.newInstance('java.net.URL', URL):openConnection():setConnectTimeout('soon')")
            .contains("bad argument #2 to 'setConnectTimeout' (number expected, got string)")
    );
    assert!(matches!(listener.accept(), Err(ref error) if error.kind() == ErrorKind::WouldBlock), "nothing ever connected to the port");
}

#[test]
fn a_skin_written_around_the_connection_reads_its_failure_as_no_answer() {
    let scratch = scratch_skin("url-flow");
    let runtime = skin(&scratch.root(), Some(&scratch.overlay()));
    let lines_are_nil: bool = eval(
        &runtime,
        r#"
        local function httpConnection(url)
            local lines = {}
            local url2 = luajava.newInstance("java.net.URL", url)
            local urlConn = url2:openConnection()
            urlConn:setRequestMethod("GET")
            urlConn:setConnectTimeout(200)
            if pcall(function() urlConn:connect() end) then
                local status = urlConn:getResponseCode()
                if status == 200 then
                    local reader = luajava.newInstance("java.io.BufferedReader", luajava.newInstance("java.io.InputStreamReader", urlConn:getInputStream(), "utf8"))
                    local line = reader:readLine()
                    while line do
                        table.insert(lines, line)
                        line = reader:readLine()
                    end
                end
            else
                return
            end
            return lines
        end
        return httpConnection("https://example.invalid/version") == nil
        "#,
    );
    assert!(lines_are_nil);
}

#[test]
fn the_pack_customfunction_module_loads_and_its_folders_go_to_the_overlay() {
    let Some(pack) = std::env::var_os(SKIN_PACK_ENV).map(PathBuf::from) else {
        return;
    };
    if !pack.join("Root/customfunction.lua").is_file() {
        return;
    }
    let scratch = Scratch::new("pack");
    let overlay = scratch.overlay();
    let before = tree(&pack);
    let runtime = skin(&pack, Some(&overlay));

    let loaded: bool = eval(
        &runtime,
        "local module = require('Root.customfunction') return type(module) == 'table' and type(module.mkdir) == 'function' and type(module.getSearchFiles) == 'function'",
    );
    assert!(loaded, "the pack's module loads without an error");
    if pack.join("Select/lua/require/http.lua").is_file() {
        let http: bool = eval(&runtime, "return type(require('Select.lua.require.http')) == 'table'");
        assert!(http, "the pack's connection module loads");
    }

    if pack.join(PACK_PROBE_PARENT).is_dir() {
        let probe = format!("{PACK_PROBE_PARENT}/{PACK_PROBE_NAME}");
        let spelling = root_spelling(&runtime);
        runtime.lua().globals().set("PROBE_PARENT", format!("{spelling}/{PACK_PROBE_PARENT}")).expect("the global is set");
        runtime.lua().globals().set("PROBE", format!("{spelling}/{probe}")).expect("the global is set");
        eval::<()>(&runtime, "require('Root.customfunction').mkdir(PROBE)");
        assert!(overlay.join(&probe).is_dir(), "the module's mkdir made its folder in the overlay");
        assert!(!pack.join(&probe).exists(), "and not in the pack");
        let (listing, count): (String, i64) = eval(&runtime, "return require('Root.customfunction').getSearchFiles(PROBE_PARENT, '^.+$')");
        let present = std::fs::read_dir(pack.join(PACK_PROBE_PARENT)).expect("the folder is listable").count();
        assert_eq!(count, present as i64 + 1, "the pack's own entries and the one the skin made: {listing}");
        assert!(listing.lines().any(|line| line.ends_with(&format!("/{probe}"))), "the new folder is listed: {listing}");
    }
    assert_eq!(tree(&pack), before, "running against the pack changed its folder");
}
