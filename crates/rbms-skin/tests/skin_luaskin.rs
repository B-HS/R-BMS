//! Loading a Lua skin: the two passes, the header merge between them, and what comes out.
//!
//! Everything but the last test runs against `fixtures/luaskin/mini`, a skin written for these
//! tests in the shape real ones have: an entry file that requires a module and returns its header
//! or the result of its `main`, a required part, two parts run with `dofile` under `pcall`, and
//! function values where a JSON document writes ids. The last test loads an external pack and only
//! runs when `RBMS_SKIN_PACK` names one.

#![cfg(feature = "lua")]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use mlua::FromLuaMulti;
use rbms_model::Mode;
use rbms_skin::SkinError;
use rbms_skin::dst::{DrawCondition, SkinOffset, TimerRef};
use rbms_skin::loader::lua_skin::{LuaSkinOptions, load_lua_skin};
use rbms_skin::loader::{
    DEFAULT_MAX_DOCUMENT_BYTES, LoadedSkin, OPTION_RANDOM_VALUE, ParserKind, PlayTimings, SKIN_TYPE_COURSE_RESULT, SKIN_TYPE_PLAY_7KEYS, SKIN_TYPE_SKIN_SELECT,
    SkinLoadOptions, SkinUserConfig, automatic_offsets, is_known_skin_type, is_lua_skin, load_header, load_skin, load_skin_with_host, skin_resolution,
};
use rbms_skin::lua::{LoadBudget, LuaBudget};
use rbms_skin::model::PropertyRef;
use rbms_skin::property::{DefaultState, INTEGER_ABSENT, MapHost, PropertyKind, SkinHost};
use rbms_skin::resolve::RANDOM_SELECTION;

/// A seed every load in this file pins.
const TEST_SEED: u64 = 7;

/// The environment variable that names an external skin pack for the optional tests.
const SKIN_PACK_ENV: &str = "RBMS_SKIN_PACK";

/// The extension of a Lua skin's entry file.
const LUA_SKIN_EXTENSION: &str = "luaskin";

/// The first of the six difficulty options, the one the pack scenario turns on.
const DIFFICULTY_FIRST: i32 = 150;

/// The last of the six difficulty options.
const DIFFICULTY_LAST: i32 = 155;

/// The number a result screen reads its clear type from.
const NUMBER_CLEAR: i32 = 370;

/// The clear type the pack scenario reports.
const SCENARIO_CLEAR: i32 = 5;

/// Bytes in the unit the pack report states memory in.
const BYTES_PER_KIBIBYTE: usize = 1024;

/// The one entry file an external pack is allowed to fail on.
///
/// The pack these tests were written against ships a key configuration screen whose body requires a
/// module of another screen and calls a function that screen's own configuration never defines, so
/// it fails in the reference as well. Its header still loads.
const KNOWN_BROKEN_ENTRY: &str = "keyconfig.luaskin";

/// The number the fixture's value function reads.
const SCORE_NUMBER: i32 = 71;

/// What the host answers for [`SCORE_NUMBER`].
const SCORE_VALUE: i32 = 21;

/// The option the fixture's draw function reads.
const AUTOPLAY_OPTION: i32 = 33;

/// The timer the fixture's timer function reads.
const PLAY_TIMER: i32 = 41;

/// The microsecond the host reports [`PLAY_TIMER`] as on since.
const PLAY_SINCE_US: i64 = 5_000_000;

/// Instructions a budget test lets one pass execute.
const SMALL_LOAD_INSTRUCTIONS: u64 = 200_000;

/// Every file under `root`, as paths relative to it.
fn files_under(root: &Path) -> BTreeSet<PathBuf> {
    let mut found = BTreeSet::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).expect("the directory should be listable").flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if let Ok(relative) = path.strip_prefix(root) {
                found.insert(relative.to_path_buf());
            }
        }
    }
    found
}

/// The host an external pack is loaded against: the first difficulty on and the other five off,
/// every text the reference names filled in, and a clear type for the result screens.
fn pack_scenario() -> MapHost {
    let mut host = MapHost::new();
    for option in DIFFICULTY_FIRST..=DIFFICULTY_LAST {
        host.booleans.insert(option, option == DIFFICULTY_FIRST);
    }
    for (id, name) in PropertyKind::String.tables().iter().flat_map(|table| table.iter()) {
        host.texts.insert(*id, format!("sample {name}"));
    }
    host.integers.insert(NUMBER_CLEAR, SCENARIO_CLEAR);
    host
}

/// Loads every Lua skin of the pack `RBMS_SKIN_PACK` names, against a host that describes a chart,
/// and prints what each one came to: its header, its object counts and everything that went wrong
/// inside it. Passes silently when the variable is unset.
///
/// The pack is read only. Whatever a skin writes goes to an overlay in the temporary directory, and
/// the pack's folder is compared before and after.
#[test]
fn an_external_skin_pack_loads_every_screen_but_the_one_its_author_left_broken() {
    let Some(pack) = std::env::var_os(SKIN_PACK_ENV).map(PathBuf::from) else {
        return;
    };
    let overlay = std::env::temp_dir().join(format!("rbms-luaskin-{}-pack-overlay", std::process::id()));
    let _ = std::fs::remove_dir_all(&overlay);
    let before = files_under(&pack);
    let mut entries: Vec<PathBuf> = std::fs::read_dir(&pack)
        .expect("the pack should be listable")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case(LUA_SKIN_EXTENSION)))
        .collect();
    entries.sort();
    assert!(!entries.is_empty(), "the pack holds no Lua skin");

    let host = pack_scenario();
    let user = SkinUserConfig::default();
    let mut loaded = Vec::new();
    let mut failed = Vec::new();
    let mut caught = 0;
    for entry in &entries {
        let name = entry.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
        let options = SkinLoadOptions { rng_seed: Some(TEST_SEED), write_overlay: Some(&overlay), ..SkinLoadOptions::new(&pack, &user, Mode::BEAT_7K) };

        let header = load_header(entry, options).unwrap_or_else(|error| panic!("{name} should give its header: {error}"));
        println!(
            "{name}: header type {} {:?} by {:?} {}x{} | property {} filepath {} offset {} category {}",
            header.skin_type,
            header.name,
            header.author,
            header.width,
            header.height,
            header.properties.len(),
            header.custom_files.len(),
            header.offsets.len(),
            header.categories.len(),
        );

        match load_skin_with_host(entry, options, &host) {
            Ok(skin) => {
                assert_eq!(skin.def.skin_type, header.skin_type, "{name}: the two ways of reading the header disagree");
                caught += report(&name, &skin);
                loaded.push(name);
            }
            Err(error) => {
                println!("{name}: FAILED: {error}");
                failed.push(name);
            }
        }
    }
    let _ = std::fs::remove_dir_all(&overlay);

    println!("loaded {} of {}: {loaded:?}; failed: {failed:?}; errors caught by pcall: {caught}", loaded.len(), entries.len());
    assert_eq!(files_under(&pack), before, "loading the pack changed its folder");
    assert!(failed.iter().all(|name| name == KNOWN_BROKEN_ENTRY), "a screen that should load did not: {failed:?}");
    assert!(!loaded.is_empty(), "no screen of the pack loaded");
    assert_eq!(caught, 0, "a part of a loaded screen failed inside the skin's own pcall, so that part is silently missing");
}

/// Prints what one loaded screen of an external pack came to, and answers how many distinct errors
/// the skin's own `pcall` caught while it loaded.
fn report(name: &str, skin: &LoadedSkin) -> usize {
    let runtime = skin.runtime().expect("a Lua skin keeps its interpreter");
    let diagnostics = runtime.diagnostics();
    println!(
        "{name}: memory {} KiB | type {} {:?} {}x{} | destination {} (assembled {}) | image {} value {} text {} source {} ({} on disk) font {} ({} on disk) | functions {} | options {} files {} offsets {} | warnings {} | pcall failures {} | function failures {} | prints {} | over budget {}",
        runtime.memory_used() / BYTES_PER_KIBIBYTE,
        skin.def.skin_type,
        skin.def.name,
        skin.resolution.0,
        skin.resolution.1,
        skin.def.destination.len(),
        skin.destinations.len(),
        skin.def.image.len(),
        skin.def.value.len(),
        skin.def.text.len(),
        skin.sources.len(),
        skin.sources.values().filter(|file| file.exists()).count(),
        skin.fonts.len(),
        skin.fonts.values().filter(|file| file.exists()).count(),
        runtime.function_count(),
        skin.selected_options.len(),
        skin.custom_files.len(),
        skin.offsets.len(),
        skin.warnings.len(),
        diagnostics.swallowed.len(),
        diagnostics.function_failures.len(),
        diagnostics.prints.len(),
        diagnostics.frames_over_budget,
    );
    for swallowed in &diagnostics.swallowed {
        println!("    pcall caught x{}: {}", swallowed.count, swallowed.message);
    }
    for failure in &diagnostics.function_failures {
        println!("    function {:?} failed x{}: {}", failure.function, failure.count, failure.first_message);
    }
    for warning in skin.warnings.iter() {
        println!("    warning: {warning}");
    }
    for line in &diagnostics.prints {
        println!("    print x{}: {}", line.count, line.text);
    }
    diagnostics.swallowed.len()
}

/// The fixture skin's root.
fn mini_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures").join("luaskin").join("mini")
}

/// The fixture's play skin.
fn mini_entry() -> PathBuf {
    mini_root().join("play.luaskin")
}

/// A scratch directory that removes itself.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("rbms-luaskin-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("the scratch directory should be creatable");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    /// Writes `text` to `relative` under the scratch directory, creating its parents.
    fn write(&self, relative: &str, text: &str) -> PathBuf {
        let path = self.0.join(relative);
        std::fs::create_dir_all(path.parent().expect("a scratch file has a parent")).expect("the parent should be creatable");
        std::fs::write(&path, text).expect("the scratch file should be writable");
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Options over `root` with every draw pinned and nowhere to write.
fn seeded<'a>(root: &'a Path, user: &'a SkinUserConfig) -> SkinLoadOptions<'a> {
    SkinLoadOptions { rng_seed: Some(TEST_SEED), ..SkinLoadOptions::new(root, user, Mode::BEAT_7K) }
}

/// A host that answers the three ids the fixture's functions read.
fn host() -> MapHost {
    let mut host = MapHost::new();
    host.integers.insert(SCORE_NUMBER, SCORE_VALUE);
    host.booleans.insert(AUTOPLAY_OPTION, true);
    host.timers.insert(PLAY_TIMER, PLAY_SINCE_US);
    host
}

/// The fixture's play skin, loaded with the player's choices against `host`.
fn load_mini(user: &SkinUserConfig, host: &dyn SkinHost) -> LoadedSkin {
    let root = mini_root();
    load_skin_with_host(&mini_entry(), seeded(&root, user), host).unwrap_or_else(|error| panic!("the fixture skin should load: {error}"))
}

/// Runs a chunk in a loaded skin's interpreter and reads what it returns.
fn eval<T: FromLuaMulti>(skin: &LoadedSkin, source: &str) -> T {
    let runtime = skin.runtime().expect("a Lua skin keeps its interpreter");
    runtime.lua().load(source).eval().unwrap_or_else(|error| panic!("{source} should run: {error}"))
}

/// The ids of a load's assembled objects, in order.
fn object_ids(skin: &LoadedSkin) -> Vec<&str> {
    skin.destinations.iter().map(|named| named.id.as_str()).collect()
}

/// The file an image source resolved to, as its path from the skin root.
fn source_file(skin: &LoadedSkin, id: &str) -> String {
    let file = skin.sources.get(id).unwrap_or_else(|| panic!("the source {id:?} should resolve"));
    file.strip_prefix(&skin.root).unwrap_or(file).to_string_lossy().replace('\\', "/")
}

/// Loads a one-file skin written on the spot.
fn load_scratch(scratch: &Scratch, source: &str) -> Result<LoadedSkin, SkinError> {
    let path = scratch.write("skin.luaskin", source);
    let user = SkinUserConfig::default();
    load_skin_with_host(&path, seeded(scratch.path(), &user), &DefaultState)
}

#[test]
fn a_lua_skin_is_known_by_its_extension() {
    assert!(is_lua_skin(Path::new("pack/play7.luaskin")));
    assert!(is_lua_skin(Path::new("PLAY7.LUASKIN")), "the extension is compared ignoring case");
    assert!(!is_lua_skin(Path::new("play7.json")));
    assert!(!is_lua_skin(Path::new("play7.lua")), "a module is not an entry file");
    assert!(!is_lua_skin(Path::new("luaskin")));
}

#[test]
fn the_header_is_read_without_building_the_screen() {
    let root = mini_root();
    let user = SkinUserConfig::default();
    let header = load_header(&mini_entry(), seeded(&root, &user)).expect("the header should load");

    assert_eq!(header.skin_type, SKIN_TYPE_PLAY_7KEYS);
    assert_eq!((header.name.as_str(), header.author.as_str()), ("Mini", "rbms"));
    assert_eq!((header.width, header.height), (1920, 1080));
    assert_eq!(header.parser, ParserKind::Lua);
    assert_eq!(header.categories.len(), 1);
    assert_eq!(header.categories[0].item, vec!["1", "2", "3"], "a category label written as a number is read as its text");
    assert_eq!(
        header.properties.iter().map(|row| (row.name.as_str(), row.category.as_str(), row.item.len())).collect::<Vec<_>>(),
        vec![("Panel", "1", 2), ("Lucky", "1", 3)]
    );
    assert_eq!(header.properties[0].def.as_deref(), Some("Off"));

    assert_eq!(header.custom_files.len(), 2);
    assert_eq!(header.custom_files[0].candidates, vec![RANDOM_SELECTION, "day.png", "night.png"]);
    assert_eq!(header.custom_files[1].candidates, vec![RANDOM_SELECTION, "alice", "bob"], "a slot that ends at its wildcard offers folders");

    let offsets: Vec<(&str, i32)> = header.offsets.iter().map(|offset| (offset.name.as_str(), offset.id)).collect();
    assert_eq!(offsets, vec![("Shift", 40), ("All offset(%)", 10), ("Notes offset", 30), ("Judge offset", 32), ("Judge Detail offset", 33)]);
    assert!(header.offsets[0].x && header.offsets[0].y && !header.offsets[0].w, "an axis the skin wrote, even as zero, is one the player may move");
}

#[test]
fn a_program_that_returns_no_header_is_not_a_skin() {
    let scratch = Scratch::new("no-header");
    let path = scratch.write("skin.luaskin", "return 42");
    let user = SkinUserConfig::default();
    assert!(matches!(load_header(&path, seeded(scratch.path(), &user)), Err(SkinError::TypeMissing)));
    assert!(matches!(load_skin(&path, seeded(scratch.path(), &user)), Err(SkinError::TypeMissing)));
}

#[test]
fn the_two_passes_share_one_interpreter_and_a_module_runs_once() {
    let skin = load_mini(&SkinUserConfig::default(), &host());

    assert_eq!(eval::<i32>(&skin, "return module_runs"), 1, "the module's top level ran in the header pass and was cached for the body pass");
    assert_eq!(eval::<i32>(&skin, "return frame_runs"), 1, "and so did the part it requires");
    assert_eq!(eval::<i32>(&skin, "return main_runs"), 1, "the body pass alone built the screen");
    assert!(eval::<bool>(&skin, "return package.loaded.main ~= nil and package.loaded['parts.frame'] ~= nil"));
    assert_eq!(skin.parser, ParserKind::Lua);
    assert_eq!(skin.mode, Mode::BEAT_7K, "the skin's own type names the mode it draws");
}

#[test]
fn the_body_pass_reads_skin_config_in_the_shape_the_reference_gives_it() {
    let mut user = SkinUserConfig::default();
    user.filepaths.insert("Background".to_owned(), "day.png".to_owned());
    user.offsets.insert(40, SkinOffset { x: 12.0, y: -3.0, ..SkinOffset::default() });
    user.offsets.insert(32, SkinOffset { a: 50.0, ..SkinOffset::default() });
    let skin = load_mini(&user, &host());

    assert!(eval::<bool>(&skin, "return seen_config == skin_config and type(skin_config) == 'table'"), "the body pass saw the global the loader published");
    assert_eq!(
        eval::<(i32, i32)>(&skin, "return skin_config.option.Panel, skin_config.option.Lucky"),
        (901, 910),
        "a row with no stored choice is on its `def`, or its first item"
    );
    assert_eq!(
        eval::<(i32, i32, i32)>(&skin, "local on = skin_config.enabled_options return #on, on[1], on[2]"),
        (2, 901, 910),
        "the same values, in the header's order"
    );

    assert_eq!(eval::<String>(&skin, "return skin_config.file_path.Background"), "day.png", "a stored choice is published as it was stored");
    assert!(eval::<bool>(&skin, "return skin_config.file_path.Chara == nil"), "a slot the player never chose has no entry, whatever its `def`");

    assert_eq!(eval::<(f32, f32, f32)>(&skin, "local shift = skin_config.offset.Shift return shift.x, shift.y, shift.w"), (12.0, -3.0, 0.0));
    assert_eq!(eval::<f32>(&skin, "return skin_config.offset['Judge offset'].a"), 50.0, "the automatic offsets are published beside the skin's own");
    let fields: i32 = eval(&skin, "local count = 0 for _ in pairs(skin_config.offset['Notes offset']) do count = count + 1 end return count");
    assert_eq!(fields, 6, "an offset nothing is stored for is six zeros");
    assert_eq!(eval::<i32>(&skin, "local count = 0 for _ in pairs(skin_config.offset) do count = count + 1 end return count"), 5);
}

#[test]
fn get_path_answers_an_absolute_path_the_skin_can_open() {
    let skin = load_mini(&SkinUserConfig::default(), &host());
    let root = skin.runtime().expect("a Lua skin keeps its interpreter").paths().root().to_string_lossy().replace('\\', "/");

    assert_eq!(eval::<String>(&skin, "return skin_config.get_path('parts/frame.lua')"), format!("{root}/parts/frame.lua"));
    assert_eq!(
        eval::<String>(&skin, "return skin_config.get_path('no/such/file.png')"),
        format!("{root}/no/such/file.png"),
        "a path is answered whether or not it exists"
    );
    assert_eq!(eval::<String>(&skin, "return skin_config.get_path('')"), root, "the skin folder itself has no trailing separator");
    assert_eq!(
        eval::<String>(&skin, "return skin_config.get_path('bg/*.png')"),
        format!("{root}/bg/night.png"),
        "a slot's pattern takes the file chosen for it"
    );
    assert_eq!(eval::<String>(&skin, "return skin_config.get_path('chara/*|1P|/body.png')"), format!("{root}/chara/bob/body.png"));
    assert_eq!(
        eval::<String>(&skin, "return skin_config.get_path('nothing/*.png')"),
        format!("{root}/nothing/*.png"),
        "a wildcard that matches nothing comes back as written"
    );
    let drawn: String = eval(&skin, "return skin_config.get_path('parts/*.lua')");
    assert!(
        [format!("{root}/parts/broken.lua"), format!("{root}/parts/extra.lua"), format!("{root}/parts/frame.lua")].contains(&drawn),
        "a wildcard no slot covers draws a file: {drawn}"
    );
    assert!(eval::<bool>(&skin, "return io.open(skin_config.get_path('parts/frame.lua'), 'r') ~= nil"), "and `io.open` reads what `get_path` names");
}

#[test]
fn a_stored_option_switches_the_objects_gated_on_it() {
    let untouched = load_mini(&SkinUserConfig::default(), &host());
    assert_eq!(untouched.selected_options, vec![("Panel".to_owned(), 901), ("Lucky".to_owned(), 910)]);
    assert_eq!(untouched.enabled_options, BTreeSet::from([901, 910]));
    assert_eq!(untouched.declared_options, BTreeSet::from([900, 901, 910, 911, 912]));
    let panels: Vec<f32> = untouched.destinations.iter().filter(|named| named.id == "panel").map(|named| named.track.frames[0].rect.x).collect();
    assert_eq!(panels, vec![100.0], "only the variant behind the selected option is assembled");
    let panel = untouched.destinations.iter().find(|named| named.id == "panel").expect("the selected variant is there");
    assert!(panel.track.draw_conditions.is_empty(), "and its option is settled, not asked about every frame");

    let mut user = SkinUserConfig::default();
    user.properties.insert("Panel".to_owned(), 900);
    let switched = load_mini(&user, &host());
    let panels: Vec<f32> = switched.destinations.iter().filter(|named| named.id == "panel").map(|named| named.track.frames[0].rect.x).collect();
    assert_eq!(panels, vec![0.0]);
    assert_eq!(eval::<i32>(&switched, "return skin_config.option.Panel"), 900);
}

#[test]
fn an_option_stored_as_random_draws_one_of_its_items_the_same_way_for_the_same_seed() {
    let mut user = SkinUserConfig::default();
    user.properties.insert("Lucky".to_owned(), OPTION_RANDOM_VALUE);
    let first = load_mini(&user, &host());
    let second = load_mini(&user, &host());

    let lucky = first.selected_options[1].1;
    assert!([910, 911, 912].contains(&lucky), "drew {lucky}");
    assert_eq!(second.selected_options[1].1, lucky, "the seed pins the draw");
    assert_eq!(eval::<i32>(&first, "return skin_config.option.Lucky"), lucky, "the skin is told the same answer");

    let root = mini_root();
    let drawn: BTreeSet<i32> = (0..32u64)
        .map(|seed| {
            let options = SkinLoadOptions { rng_seed: Some(seed), ..SkinLoadOptions::new(&root, &user, Mode::BEAT_7K) };
            load_skin_with_host(&mini_entry(), options, &DefaultState).expect("the fixture skin should load").selected_options[1].1
        })
        .collect();
    assert!(drawn.len() > 1, "thirty-two seeds all drew {drawn:?}");
}

#[test]
fn a_file_slot_takes_the_stored_choice_the_default_or_a_draw() {
    let untouched = load_mini(&SkinUserConfig::default(), &host());
    assert_eq!(source_file(&untouched, "0"), "bg/night.png", "`def = \"Night\"` names `night.png`: the stem, ignoring case");
    assert_eq!(source_file(&untouched, "2"), "chara/bob/body.png", "a folder slot keeps what the pattern had after its wildcard");
    assert_eq!(source_file(&untouched, "1"), "parts/sheet.png", "a plain path is resolved whether or not the file is there");
    assert_eq!(untouched.filemap().len(), 2);

    let mut user = SkinUserConfig::default();
    user.filepaths.insert("Background".to_owned(), "day.png".to_owned());
    user.filepaths.insert("Chara".to_owned(), "alice".to_owned());
    let chosen = load_mini(&user, &host());
    assert_eq!((source_file(&chosen, "0"), source_file(&chosen, "2")), ("bg/day.png".to_owned(), "chara/alice/body.png".to_owned()));

    user.filepaths.insert("Background".to_owned(), RANDOM_SELECTION.to_owned());
    let random = load_mini(&user, &host());
    let drawn = source_file(&random, "0");
    assert!(["bg/day.png", "bg/night.png"].contains(&drawn.as_str()), "drew {drawn}");
    assert_eq!(source_file(&load_mini(&user, &host()), "0"), drawn, "the seed pins the draw");
    assert_eq!(eval::<String>(&random, "return skin_config.file_path.Background"), RANDOM_SELECTION, "the skin is told what was stored, not what was drawn");
}

#[test]
fn a_play_skin_gains_the_four_automatic_offsets_and_no_other_screen_does() {
    let skin = load_mini(&SkinUserConfig::default(), &host());
    assert_eq!(skin.offsets.iter().map(|offset| offset.id).collect::<Vec<_>>(), vec![40, 10, 30, 32, 33], "the skin's own first, then the four");
    assert_eq!(skin.def.offset.len(), 1, "the document itself still says what the skin declared");

    let all = &skin.offsets[1];
    assert_eq!((all.x, all.y, all.w, all.h, all.r, all.a), (true, true, true, true, false, false));
    let notes = &skin.offsets[2];
    assert_eq!((notes.x, notes.y, notes.w, notes.h, notes.r, notes.a), (false, false, false, true, false, false));
    for judge in &skin.offsets[3..] {
        assert_eq!((judge.x, judge.y, judge.w, judge.h, judge.r, judge.a), (true, true, true, true, false, true));
    }

    for play in [0, 1, 2, 3, 4, 16, 17] {
        assert_eq!(automatic_offsets(play).len(), 4, "type {play} is a play screen");
    }
    for other in [5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 18] {
        assert!(automatic_offsets(other).is_empty(), "type {other} gains nothing");
    }
}

#[test]
fn the_skin_configuration_and_course_result_screens_load_like_any_other() {
    let root = mini_root();
    let user = SkinUserConfig::default();
    for (file, skin_type) in [("config.luaskin", SKIN_TYPE_SKIN_SELECT), ("course.luaskin", SKIN_TYPE_COURSE_RESULT)] {
        let skin = load_skin(&root.join(file), seeded(&root, &user)).unwrap_or_else(|error| panic!("{file} should load: {error}"));
        assert_eq!(skin.def.skin_type, skin_type);
        assert!(skin.offsets.is_empty());
        assert_eq!(load_header(&root.join(file), seeded(&root, &user)).expect("its header loads").skin_type, skin_type);
    }
    assert!((0..=18).all(is_known_skin_type), "every id the reference has is one a skin can be loaded as");
    assert!(!is_known_skin_type(19) && !is_known_skin_type(-1));
}

#[test]
fn a_type_the_reference_does_not_have_is_refused() {
    let scratch = Scratch::new("unknown-type");
    let outcome = load_scratch(&scratch, "return { type = 99, destination = {} }");
    assert!(matches!(outcome, Err(SkinError::TypeUnsupported(99))), "got {outcome:?}");
}

#[test]
fn the_play_timings_apply_only_when_the_note_object_is_placed() {
    let placed = load_mini(&SkinUserConfig::default(), &host());
    assert_eq!(placed.play, PlayTimings { close: 3000, loadend: 3500, playstart: 1000, judgetimer: 2, finishmargin: 300 });

    let root = mini_root();
    let user = SkinUserConfig::default();
    let unplaced = load_skin(&root.join("nonote.luaskin"), seeded(&root, &user)).expect("the skin should load");
    assert_eq!((unplaced.def.close, unplaced.def.loadend, unplaced.def.judgetimer), (3000, 3500, 2), "the skin declares the same fields");
    assert_eq!(unplaced.play, PlayTimings::default(), "but no destination places its notes, so none of them is taken");
    assert_eq!(PlayTimings::default(), PlayTimings { close: 0, loadend: 0, playstart: 0, judgetimer: 1, finishmargin: 0 });
}

#[test]
fn a_function_value_is_registered_and_answers_from_the_frames_host() {
    let host = host();
    let skin = load_mini(&SkinUserConfig::default(), &host);
    let runtime = skin.runtime().expect("a Lua skin keeps its interpreter");
    assert_eq!(runtime.function_count(), 3, "the value, the condition and the timer");

    let Some(PropertyRef::Func(value)) = skin.def.value[0].value else {
        panic!("the value should be a function: {:?}", skin.def.value[0].value);
    };
    let combo = &skin.destinations.iter().find(|named| named.id == "combo").expect("the object is assembled").track;
    let [DrawCondition::Function(gate)] = combo.draw_conditions.as_slice() else {
        panic!("the object should be gated on one function: {:?}", combo.draw_conditions);
    };
    let Some(TimerRef::Lua(timer)) = combo.timer else {
        panic!("the object should follow a function timer: {:?}", combo.timer);
    };
    assert_eq!(
        (combo.frames[0].rect.w, combo.frames[1].rect.x, combo.frames[1].time_ms),
        (15.0, 110.0, 1000),
        "`30 / 2` is a whole number and a later keyframe inherits the rest"
    );

    let answers = runtime.frame(&host, |frame| (frame.call_integer(value), frame.call_boolean(*gate), frame.call_timer(timer))).expect("the frame binds");
    assert_eq!(answers, (SCORE_VALUE * 2, true, PLAY_SINCE_US));

    let silent = runtime.frame(&DefaultState, |frame| (frame.call_integer(value), frame.call_boolean(*gate))).expect("the frame binds");
    assert_eq!(silent, (INTEGER_ABSENT.wrapping_mul(2), false), "the same functions against a host that knows nothing");
    assert!(runtime.diagnostics().function_failures.is_empty(), "{:?}", runtime.diagnostics().function_failures);
}

#[test]
fn a_part_run_under_pcall_adds_its_objects_and_a_broken_one_is_recorded() {
    let skin = load_mini(&SkinUserConfig::default(), &host());

    assert!(eval::<bool>(&skin, "return extra_ran"));
    assert!(!eval::<bool>(&skin, "return broken_ran"), "the skin's own `pcall` still answers false");
    assert_eq!(object_ids(&skin), vec!["bg", "panel", "combo", "notes", "chara", "bg"], "the required part and the part run with `dofile` both added one");
    assert_eq!(skin.destinations.last().map(|named| named.track.blend), Some(2));

    let diagnostics = skin.runtime().expect("a Lua skin keeps its interpreter").diagnostics();
    assert_eq!(diagnostics.swallowed.len(), 1, "{:?}", diagnostics.swallowed);
    let caught = &diagnostics.swallowed[0];
    assert_eq!((caught.file.as_deref(), caught.line, caught.count), (Some("parts/broken.lua"), Some(2), 1), "{}", caught.message);
    assert_eq!(skin.warnings, Vec::<String>::new());
}

#[test]
fn a_skin_writes_into_its_overlay_and_never_into_its_own_folder() {
    let root = mini_root();
    let before = files_under(&root);
    let scratch = Scratch::new("overlay");
    let user = SkinUserConfig::default();

    let read_only = load_mini(&user, &host());
    assert!(!eval::<bool>(&read_only, "return wrote_log"), "with no overlay the write fails and the skin goes on");

    let options = SkinLoadOptions { write_overlay: Some(scratch.path()), ..seeded(&root, &user) };
    let writable = load_skin_with_host(&mini_entry(), options, &host()).expect("the fixture skin should load");
    assert!(eval::<bool>(&writable, "return wrote_log"));
    assert_eq!(std::fs::read_to_string(scratch.path().join("log").join("loaded.txt")).expect("the write landed in the overlay"), "loaded");
    assert!(
        eval::<bool>(&writable, "return io.open(skin_config.get_path('log/loaded.txt'), 'r') ~= nil"),
        "and the skin reads it back as though it were in its own folder"
    );

    assert_eq!(files_under(&root), before, "loading the skin changed its folder");
}

#[test]
fn a_size_no_skin_can_be_authored_at_is_read_as_the_default_one() {
    assert_eq!(skin_resolution(1920, 1080), (1920, 1080));
    assert_eq!(skin_resolution(640, 480), (640, 480));
    assert_eq!(skin_resolution(1000, 500), (1280, 720));
    assert_eq!(skin_resolution(1080, 1920), (1280, 720), "a resolution turned on its side is not one of the fifteen");

    let scratch = Scratch::new("odd-size");
    let skin = load_scratch(&scratch, "return { type = 6, w = 1000, h = 500, destination = {} }").expect("the skin should load");
    assert_eq!(skin.resolution, (1280, 720));
    assert_eq!((skin.def.w, skin.def.h), (1280, 720), "everything that scales the skin reads the same size");
    assert!(skin.warnings.iter().any(|warning| warning.contains("1000x500")), "{:?}", skin.warnings);

    assert_eq!(load_mini(&SkinUserConfig::default(), &host()).resolution, (1920, 1080));
}

#[test]
fn the_header_pass_decides_the_rows_and_the_body_pass_everything_drawn() {
    let scratch = Scratch::new("two-results");
    let source = r#"
        if skin_config then
            return { type = 7, name = "body", w = 1920, h = 1080, input = 500, property = {}, destination = { { id = "-110", dst = { { x = 0 } } } } }
        end
        return { type = 6, name = "header", w = 640, h = 480, input = 100, property = { { name = "Row", item = { { name = "Only", op = 900 } } } } }
    "#;
    let skin = load_scratch(&scratch, source).expect("the skin should load");

    assert_eq!((skin.def.skin_type, skin.def.name.as_str()), (6, "header"), "the type and the name are the header pass's");
    assert_eq!(skin.def.property.len(), 1, "and so are the customisation rows");
    assert_eq!(skin.selected_options, vec![("Row".to_owned(), 900)]);
    assert_eq!((skin.def.w, skin.def.h, skin.def.input), (1920, 1080, 500), "the size and the scene timings are the body pass's");
    assert_eq!(object_ids(&skin), vec!["-110"]);
}

#[test]
fn a_body_pass_that_returns_nothing_to_draw_is_refused() {
    let scratch = Scratch::new("no-destination");
    let outcome = load_scratch(&scratch, "if skin_config then return { type = 6 } end return { type = 6 }");
    let Err(SkinError::LuaLoad { path, message }) = outcome else {
        panic!("a skin with no destination list must not load, got {outcome:?}");
    };
    assert!(path.contains("body pass") && message.contains("destination"), "{path}: {message}");

    let empty = load_scratch(&scratch, "return { type = 6, destination = {} }").expect("an empty list is still a list");
    assert!(empty.destinations.is_empty());
}

#[test]
fn a_skin_that_fails_while_it_runs_is_abandoned_with_the_place_it_failed() {
    let scratch = Scratch::new("raises");
    scratch.write("part.lua", "local part = {}\nfunction part.build()\n    error('no such asset')\nend\nreturn part\n");
    let outcome = load_scratch(&scratch, "local part = require('part')\nif skin_config then return part.build() end\nreturn { type = 6 }");
    let Err(SkinError::LuaLoad { path, message }) = outcome else {
        panic!("a skin that raises must not load, got {outcome:?}");
    };
    assert!(path.contains("skin.luaskin") && path.contains("body pass"), "{path}");
    assert!(message.contains("part.lua:3: no such asset"), "{message}");
}

#[test]
fn a_skin_that_never_finishes_is_cut_off_by_the_load_budget() {
    let scratch = Scratch::new("runaway");
    let path = scratch.write("skin.luaskin", "if skin_config then while true do end end return { type = 6 }");
    let user = SkinUserConfig::default();
    let budget = LuaBudget { load: LoadBudget { max_instructions: SMALL_LOAD_INSTRUCTIONS, ..LoadBudget::default() }, ..LuaBudget::default() };
    let outcome = load_lua_skin(&path, &LuaSkinOptions { load: seeded(scratch.path(), &user), budget }, &DefaultState);
    assert!(matches!(outcome, Err(SkinError::LuaBudget { .. })), "got {outcome:?}");
}

#[test]
fn an_entry_file_outside_the_root_or_over_the_ceiling_is_never_run() {
    let scratch = Scratch::new("refused");
    scratch.write("outside.luaskin", "ran = true return { type = 6, destination = {} }");
    let inside = scratch.write("pack/skin.luaskin", "return { type = 6, destination = {} }");
    let root = scratch.path().join("pack");
    let user = SkinUserConfig::default();

    let escaped = load_skin(&root.join("..").join("outside.luaskin"), seeded(&root, &user));
    assert!(matches!(escaped, Err(SkinError::PathEscape(_))), "got {escaped:?}");

    let small = SkinLoadOptions { max_document_bytes: 4, ..seeded(&root, &user) };
    let too_large = load_skin(&inside, small);
    assert!(matches!(too_large, Err(SkinError::TooLarge { limit: 4, .. })), "got {too_large:?}");
    assert!(load_skin(&inside, SkinLoadOptions { max_document_bytes: DEFAULT_MAX_DOCUMENT_BYTES, ..seeded(&root, &user) }).is_ok());
}

#[test]
fn a_skin_cannot_reach_above_the_folder_its_entry_file_is_in() {
    let scratch = Scratch::new("confined");
    scratch.write("shared.lua", "return 'shared'");
    scratch.write("secret.txt", "not for a skin");
    let source = r#"
        reached_module = pcall(require, "..shared")
        reached_file = pcall(dofile, "../shared.lua")
        reached_data = io.open("../secret.txt", "r") ~= nil
        made_bytecode = string.dump ~= nil
        return { type = 6, destination = {} }
    "#;
    let path = scratch.write("pack/skin.luaskin", source);
    let user = SkinUserConfig::default();
    let skin = load_skin(&path, seeded(scratch.path(), &user)).expect("the skin should load");

    assert!(
        !eval::<bool>(&skin, "return reached_module or reached_file or reached_data or made_bytecode"),
        "the allowed root is the pack's parent, but the skin's world is its own folder"
    );
}

#[test]
fn a_file_that_is_not_there_is_reported_as_unreadable() {
    let root = mini_root();
    let user = SkinUserConfig::default();
    let outcome = load_skin_with_host(&root.join("missing.luaskin"), seeded(&root, &user), &MapHost::new());
    assert!(matches!(outcome, Err(SkinError::Read(_))), "got {outcome:?}");
}
