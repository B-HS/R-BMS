//! The default skin pack in the repository, checked as the files it is made of.
//!
//! The pack is `assets/skins/rbms-default`, the one a first run draws, so these checks name it
//! themselves and run wherever the tests do. They need no tool that generates the pack's pictures and
//! sounds, so they do not generate them again: they hold the generated files to what the generator
//! wrote about them, and the Lua to what a skin that is read from a folder it may not write to has
//! to leave alone.
//!
//! - No Lua of the pack reaches for what a read-only, reproducible skin cannot use: `luajava`, the
//!   `io` and `os` libraries, the `main_state` writers of files, `math.random`, anything http, or a
//!   way round the scan itself -- a library taken as a value, the table of globals, text run as a
//!   program. The Lua is read as Lua for this, so a string that holds `--` does not hide the rest
//!   of its line and a string that merely names a library is not taken to call it.
//! - Every picture sheet has the table the generator wrote for it and the table says what the sheet
//!   is: the sheet is there, is the size the table gives, and every sprite lies inside it.
//!
//! The documents loading, drawing and costing nothing they should not is the dump tool's strict gate
//! (`rbms-cli`), and the screens being drawn is the player's.

#![cfg(feature = "lua")]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use mlua::{Lua, Table};

/// The pack's folder in the repository, from this package's own.
const PACK_FOLDER: &str = "../../assets/skins/rbms-default";

/// The folder of the pack that holds what its pictures and sounds are generated with, which no Lua
/// of a skin is in.
const TOOLS_FOLDER: &str = "tools";

/// The folder of the pack that holds the table of each picture sheet, and the one the sheets are in.
const ATLAS_FOLDER: &str = "lua/atlas";
const SHEET_FOLDER: &str = "image";

/// The extensions of the pack's Lua: a module, and a skin document that is Lua too.
const LUA_EXTENSIONS: [&str; 2] = ["lua", "luaskin"];

/// What a Lua comment starts with, what a string or a comment that runs over lines is opened and
/// closed with -- any number of [`LONG_BRACKET_LEVEL`] between the two brackets -- and what a
/// string escapes its next character with.
const COMMENT_OPEN: &str = "--";
const LONG_BRACKET_OPEN: char = '[';
const LONG_BRACKET_CLOSE: char = ']';
const LONG_BRACKET_LEVEL: char = '=';
const STRING_QUOTES: [char; 2] = ['\'', '"'];
const STRING_ESCAPE: char = '\\';

/// What reaches a member of a table, by name and as a method.
const MEMBER_ACCESS: [char; 2] = ['.', ':'];

/// What a call's arguments open with.
const CALL_OPEN: char = '(';

/// The bytes a PNG file starts with, and where in it the width and height of the picture are: the
/// first chunk is the 13 bytes of header, after its length and its name.
const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];
const PNG_HEADER_NAME_AT: usize = 12;
const PNG_WIDTH_AT: usize = 16;
const PNG_HEIGHT_AT: usize = 20;
const PNG_HEADER_END: usize = 24;

/// One piece of a Lua program, with its comments and the white space between its pieces left out.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    /// A name or a keyword.
    Word(String),
    /// What a string holds, as it is written.
    Text(String),
    /// A single character that is neither.
    Symbol(char),
    /// A number, or a run of dots that joins two strings or stands for the rest of the arguments:
    /// something that is not a name and does not reach a member.
    Other,
}

/// How a name is looked for in a program.
#[derive(Debug, Clone, Copy)]
enum Name {
    /// A name wherever it is written, a member of a table or not: `file_write`, but not
    /// `file_writer` or `my_file_write`.
    Anywhere(&'static str),
    /// A name that stands on its own rather than naming a member of a table: the library `io`, but
    /// not the `io` of `layout.io`.
    Global(&'static str),
    /// A library that may be reached into for its members, all but the ones listed, and may not be
    /// used any other way: `math.floor(1)`, but not `math.random()`, `local m = math` or
    /// `math["random"]`.
    Library(&'static str, &'static [&'static str]),
    /// A call that asks for a module by one of the listed names: `require("io")`.
    Module(&'static str, &'static [&'static str]),
    /// A run of letters in any name or string, whatever the case: `http` in `main_state.http_get`
    /// and in a URL.
    Run(&'static str),
}

/// The libraries a skin may not have, under the names a module is asked for by.
const FORBIDDEN_MODULES: [&str; 5] = ["io", "os", "luajava", "debug", "package"];

/// What a Lua program of the pack may not name, and why: each is a way to write a file, read the
/// clock, draw a number the next run draws differently, reach out of the machine, or get at one of
/// those without naming it where this scan would see it. `main_state.file_*` are the writers of
/// files the host gives a skin.
const FORBIDDEN: [(Name, &str); 18] = [
    (Name::Anywhere("luajava"), "reaches into the host's classes"),
    (Name::Global("io"), "reads and writes files"),
    (Name::Global("os"), "reads the clock and the machine"),
    (Name::Library("math", &["random", "randomseed"]), "draws a number a run cannot repeat, or hands the library on under another name"),
    (Name::Anywhere("file_write"), "writes a file"),
    (Name::Anywhere("file_append"), "writes a file"),
    (Name::Anywhere("file_mkdir"), "makes a folder"),
    (Name::Anywhere("file_clear"), "empties a file"),
    (Name::Run("http"), "reaches out of the machine"),
    (Name::Global("_G"), "reaches any library by a name this scan cannot read"),
    (Name::Global("_ENV"), "reaches any library by a name this scan cannot read"),
    (Name::Global("debug"), "reaches behind the program"),
    (Name::Global("package"), "holds every library that was loaded"),
    (Name::Global("load"), "runs text as a program"),
    (Name::Global("loadstring"), "runs text as a program"),
    (Name::Global("loadfile"), "runs a file this scan did not read"),
    (Name::Global("dofile"), "runs a file this scan did not read"),
    (Name::Module("require", &FORBIDDEN_MODULES), "asks for a library a skin may not have"),
];

fn pack() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(PACK_FOLDER)
}

/// Every file under `folder` with one of `extensions`, below `root`'s `tools` folder or not, as its
/// path from `root` with `/` between its parts.
fn files_with(root: &Path, folder: &Path, extensions: &[&str], found: &mut BTreeSet<String>) {
    let entries = std::fs::read_dir(folder).unwrap_or_else(|error| panic!("{} should be listable: {error}", folder.display()));
    for entry in entries.flatten() {
        let path = entry.path();
        let relative = path.strip_prefix(root).expect("a file under the pack is below it");
        if relative.starts_with(TOOLS_FOLDER) {
            continue;
        }
        if path.is_dir() {
            files_with(root, &path, extensions, found);
        } else if path.extension().is_some_and(|extension| extensions.iter().any(|known| extension.eq_ignore_ascii_case(known))) {
            let parts: Vec<String> = relative.components().map(|part| part.as_os_str().to_string_lossy().into_owned()).collect();
            found.insert(parts.join("/"));
        }
    }
}

/// How many [`LONG_BRACKET_LEVEL`] a long bracket that opens at the start of `text` has between
/// its two brackets, or `None` when no long bracket opens there.
fn long_bracket_level(text: &str) -> Option<usize> {
    let inside = text.strip_prefix(LONG_BRACKET_OPEN)?;
    let level = inside.chars().take_while(|character| *character == LONG_BRACKET_LEVEL).count();
    inside[level..].starts_with(LONG_BRACKET_OPEN).then_some(level)
}

/// What a long bracket of `level` that opens at the start of `text` holds, and how much of `text`
/// it takes with its brackets. One that is never closed holds the rest of the text.
fn long_bracket(text: &str, level: usize) -> (&str, usize) {
    let opened = level + 2 * LONG_BRACKET_OPEN.len_utf8();
    let close = format!("{LONG_BRACKET_CLOSE}{}{LONG_BRACKET_CLOSE}", LONG_BRACKET_LEVEL.to_string().repeat(level));
    match text[opened..].find(&close) {
        Some(at) => (&text[opened..opened + at], opened + at + close.len()),
        None => (&text[opened..], text.len()),
    }
}

/// What the quoted string that opens at the start of `text` holds, as it is written, and how much
/// of `text` it takes with its quotes. One that is never closed holds the rest of its line.
fn quoted(text: &str, quote: char) -> (&str, usize) {
    let mut escaped = false;
    for (at, character) in text.char_indices().skip(1) {
        if escaped {
            escaped = false;
        } else if character == STRING_ESCAPE {
            escaped = true;
        } else if character == quote {
            return (&text[quote.len_utf8()..at], at + quote.len_utf8());
        } else if character == '\n' {
            return (&text[quote.len_utf8()..at], at);
        }
    }
    (&text[quote.len_utf8()..], text.len())
}

fn is_identifier_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

/// How much of the start of `text` the characters `wanted` accepts take.
fn run_of(text: &str, wanted: impl Fn(char) -> bool) -> usize {
    text.find(|character| !wanted(character)).unwrap_or(text.len())
}

/// The Lua in `text` as its pieces, in order: its names, what its strings hold, and its symbols.
/// Comments are left out, so a program that only talks about `io.open` is not taken to call it, and
/// a string is one piece whatever it holds, so one that holds `--` hides nothing after it.
fn tokens(text: &str) -> Vec<Token> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(first) = rest.chars().next() {
        let taken = if let Some(comment) = rest.strip_prefix(COMMENT_OPEN) {
            let length = match long_bracket_level(comment) {
                Some(level) => long_bracket(comment, level).1,
                None => comment.find('\n').unwrap_or(comment.len()),
            };
            COMMENT_OPEN.len() + length
        } else if let Some(level) = long_bracket_level(rest) {
            let (held, length) = long_bracket(rest, level);
            found.push(Token::Text(held.to_owned()));
            length
        } else if STRING_QUOTES.contains(&first) {
            let (held, length) = quoted(rest, first);
            found.push(Token::Text(held.to_owned()));
            length
        } else if first.is_ascii_digit() {
            found.push(Token::Other);
            run_of(rest, |character| is_identifier_character(character) || character == '.')
        } else if is_identifier_character(first) {
            let length = run_of(rest, is_identifier_character);
            found.push(Token::Word(rest[..length].to_owned()));
            length
        } else if first == '.' {
            let length = run_of(rest, |character| character == '.');
            found.push(if length == 1 { Token::Symbol(first) } else { Token::Other });
            length
        } else {
            if !first.is_whitespace() {
                found.push(Token::Symbol(first));
            }
            first.len_utf8()
        };
        rest = &rest[taken..];
    }
    found
}

/// Whether the piece at `at` is the name `word`.
fn is_word(tokens: &[Token], at: usize, word: &str) -> bool {
    matches!(tokens.get(at), Some(Token::Word(found)) if found == word)
}

/// Whether the piece at `at` is the name `word` standing on its own, not naming a member of the
/// table before it.
fn is_global(tokens: &[Token], at: usize, word: &str) -> bool {
    let is_member = at.checked_sub(1).is_some_and(|before| matches!(tokens[before], Token::Symbol(symbol) if MEMBER_ACCESS.contains(&symbol)));
    is_word(tokens, at, word) && !is_member
}

/// Whether the program `tokens` is the pieces of names `name`.
fn names(tokens: &[Token], name: Name) -> bool {
    let places = 0..tokens.len();
    match name {
        Name::Anywhere(word) => places.into_iter().any(|at| is_word(tokens, at, word)),
        Name::Global(word) => places.into_iter().any(|at| is_global(tokens, at, word)),
        Name::Library(library, withheld) => places.into_iter().filter(|at| is_global(tokens, *at, library)).any(|at| {
            let member = match (tokens.get(at + 1), tokens.get(at + 2)) {
                (Some(Token::Symbol('.')), Some(Token::Word(member))) => Some(member.as_str()),
                _ => None,
            };
            member.is_none_or(|member| withheld.contains(&member))
        }),
        Name::Module(call, modules) => places.into_iter().filter(|at| is_global(tokens, *at, call)).any(|at| {
            let argument = tokens[at + 1..].iter().find(|token| **token != Token::Symbol(CALL_OPEN));
            matches!(argument, Some(Token::Text(module)) if modules.contains(&module.as_str()))
        }),
        Name::Run(run) => tokens.iter().any(|token| matches!(token, Token::Word(held) | Token::Text(held) if held.to_ascii_lowercase().contains(run))),
    }
}

/// Every reason the Lua in `text` may not be a skin's, one line each.
fn findings(file: &str, text: &str) -> Vec<String> {
    let code = tokens(text);
    FORBIDDEN.iter().filter(|(name, _)| names(&code, *name)).map(|(name, why)| format!("{file} names {name:?}, which {why}")).collect()
}

/// The scanner finds what it is looking for, and not what only looks like it: it is the check on
/// the pack's Lua, and one that finds nothing would pass the pack whatever the pack held. What it
/// has to find includes the ways round it: a library reached past a string that holds `--`, taken
/// as a value, indexed by a string, or asked for by name.
#[test]
fn the_scan_for_forbidden_names_finds_them_and_only_them() {
    let flagged = |source: &str| findings("probe.lua", source).len();
    for source in [
        "local f = io.open('x', 'w')",
        "io .write('x')",
        "return os.date('%Y')",
        "local clock = os.time()",
        "return os . clock()",
        "return math.random(3)",
        "math.randomseed(7)",
        "local java = luajava.bindClass('java.io.File')",
        "require('main_state').file_write('a', 'b')",
        "main_state.http_get('u')",
        "local u = luajava.new(File, 'x')",
        "return 'see https://example.org'",
        "local dashes = \"--\" local f = io",
        "local dashes = '--' return io.open('x')",
        "local dashes = [[--]] return os.date()",
        "local quote = 'it\\'s -- fine' return math.random()",
        "local rand = math[\"random\"]",
        "local m = math",
        "local r = (math).random",
        "return f(math)",
        "local lib = _G[\"io\"]",
        "local lib = _ENV['o' .. 's']",
        "return 'x' ..os.date()",
        "return load('return io')()",
        "return loadstring('return 1')",
        "dofile('x.lua')",
        "local lib = require('io')",
        "local lib = require \"os\"",
        "local lib = package.loaded.io",
        "return debug.getregistry()",
        "--[==[ a comment ]] that goes on ]==] return io.read()",
    ] {
        assert!(flagged(source) > 0, "{source:?} was not found");
    }
    for source in [
        "-- io.open is not used here\nreturn 1",
        "--[[ math.random\nos.date ]] return 2",
        "--[==[ io.open ]] os.date ]==] return 3",
        "local radio = 1 return radio.open",
        "local ratio = position + math.floor(1.5)",
        "return math.max(math.min(a, b), math.abs(c)), math.ceil(d), math.huge",
        "return 'shotgun'",
        "return 'io.open, os.date and math.random are only talked about'",
        "return [[ math.random is only talked about ]]",
        "return layout.io, layout.os, layout:load()",
        "local ui = require(\"lua.ui\") return require(PREFIX .. name)",
        "local dashes = '--' return 1",
        "return 0x10 + 1e5 + 3.5 .. 'px'",
    ] {
        assert_eq!(flagged(source), 0, "{source:?} was found: {:?}", findings("probe.lua", source));
    }
}

/// The pieces a program is read as: a string is one piece whatever it holds, a comment is none, and
/// the dot that reaches a member is told from the dots that join two strings.
#[test]
fn a_program_is_read_as_its_names_strings_and_symbols() {
    let word = |name: &str| Token::Word(name.to_owned());
    let text = |held: &str| Token::Text(held.to_owned());
    assert_eq!(tokens("a.b -- c\n'--' .. [=[x]]y]=]"), [word("a"), Token::Symbol('.'), word("b"), text("--"), Token::Other, text("x]]y")]);
    assert_eq!(
        tokens("f('a\\'b', \"c\") --[[ d\ne ]] g"),
        [word("f"), Token::Symbol('('), text("a\\'b"), Token::Symbol(','), text("c"), Token::Symbol(')'), word("g")]
    );
    assert_eq!(tokens("x = 'unclosed\ny"), [word("x"), Token::Symbol('='), text("unclosed"), word("y")]);
    assert_eq!(tokens("t[1][2]"), [word("t"), Token::Symbol('['), Token::Other, Token::Symbol(']'), Token::Symbol('['), Token::Other, Token::Symbol(']')]);
}

/// None of the pack's Lua names what a skin read from a folder it may not write to, and drawn
/// the same way every run, has no use for.
#[test]
fn the_packs_lua_names_no_file_writer_no_clock_no_random_number_and_no_network() {
    let pack = pack();
    let mut files = BTreeSet::new();
    files_with(&pack, &pack, &LUA_EXTENSIONS, &mut files);
    assert!(files.iter().any(|file| file.ends_with(".luaskin")) && files.iter().any(|file| file.starts_with("lua/")), "found {files:?}");

    let found: Vec<String> = files
        .iter()
        .flat_map(|file| {
            let text = std::fs::read_to_string(pack.join(file)).unwrap_or_else(|error| panic!("{file} should be readable: {error}"));
            findings(file, &text)
        })
        .collect();
    assert!(found.is_empty(), "the pack's Lua reaches for what it may not:\n{}", found.join("\n"));
}

/// A positive whole number out of a Lua table, which Lua keeps as a float.
fn whole(table: &Table, key: &str) -> u32 {
    let value: f64 = table.get(key).unwrap_or_else(|error| panic!("{key} should be a number: {error}"));
    assert!(value.fract() == 0.0 && value >= 0.0 && value <= f64::from(u32::MAX), "{key} is {value}, not a whole number of pixels");
    value as u32
}

/// The width and height a PNG file says its picture has.
fn png_size(file: &Path) -> (u32, u32) {
    let bytes = std::fs::read(file).unwrap_or_else(|error| panic!("{} should be readable: {error}", file.display()));
    assert!(bytes.len() >= PNG_HEADER_END && bytes.starts_with(&PNG_SIGNATURE), "{} is not a PNG file", file.display());
    assert_eq!(&bytes[PNG_HEADER_NAME_AT..PNG_WIDTH_AT], b"IHDR", "{} does not start with its header", file.display());
    let read = |at: usize| u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
    (read(PNG_WIDTH_AT), read(PNG_HEIGHT_AT))
}

/// Every picture sheet of the pack has the table the generator wrote for it and every table has its
/// sheet; the sheet is the size the table says, and each sprite lies wholly inside it, is at least a
/// pixel across, and is cut into whole cells. The tables are what the Lua of the pack places every
/// image by, so a sprite outside its sheet is a picture the skin would draw as nothing.
#[test]
fn every_picture_sheet_is_the_size_its_table_says_and_holds_its_sprites() {
    let pack = pack();
    let lua = Lua::new();
    let tables: BTreeSet<String> = std::fs::read_dir(pack.join(ATLAS_FOLDER))
        .expect("the pack has its tables")
        .flatten()
        .filter_map(|entry| entry.path().file_stem().map(|stem| stem.to_string_lossy().into_owned()))
        .collect();
    let sheets: BTreeSet<String> = std::fs::read_dir(pack.join(SHEET_FOLDER))
        .expect("the pack has its sheets")
        .flatten()
        .filter_map(|entry| entry.path().file_stem().map(|stem| stem.to_string_lossy().into_owned()))
        .collect();
    assert!(!tables.is_empty(), "the pack has no table of a sheet");
    assert_eq!(tables, sheets, "a sheet has no table, or a table has no sheet");

    for name in &tables {
        let table_file = pack.join(ATLAS_FOLDER).join(format!("{name}.lua"));
        let source = std::fs::read_to_string(&table_file).unwrap_or_else(|error| panic!("{} should be readable: {error}", table_file.display()));
        let atlas: Table =
            lua.load(&source).set_name(format!("{name}.lua")).eval().unwrap_or_else(|error| panic!("{name}.lua should return its table: {error}"));

        let sheet: String = atlas.get("path").unwrap_or_else(|error| panic!("{name}.lua should name its sheet: {error}"));
        assert_eq!(sheet, format!("{SHEET_FOLDER}/{name}.png"), "{name}.lua names another sheet than the one it is the table of");
        let generator: String = atlas.get("generated_by").unwrap_or_else(|error| panic!("{name}.lua should name what wrote it: {error}"));
        assert!(pack.join(&generator).is_file(), "{name}.lua was written by {generator}, which is not in the pack");

        let (width, height) = (whole(&atlas, "width"), whole(&atlas, "height"));
        assert_eq!(png_size(&pack.join(&sheet)), (width, height), "{sheet} is not the size {name}.lua says it is");

        let sprites: Table = atlas.get("sprites").unwrap_or_else(|error| panic!("{name}.lua should hold its sprites: {error}"));
        let mut count = 0;
        for pair in sprites.pairs::<String, Table>() {
            let (sprite, cell) = pair.unwrap_or_else(|error| panic!("{name}.lua holds a sprite that is not a table: {error}"));
            let (x, y, w, h) = (whole(&cell, "x"), whole(&cell, "y"), whole(&cell, "w"), whole(&cell, "h"));
            let (divx, divy) = (whole(&cell, "divx"), whole(&cell, "divy"));
            assert!(w > 0 && h > 0, "{name}.{sprite} is {w}x{h}");
            assert!(x + w <= width && y + h <= height, "{name}.{sprite} at {x},{y} of {w}x{h} runs out of its {width}x{height} sheet");
            assert!(
                divx > 0 && divy > 0 && w.is_multiple_of(divx) && h.is_multiple_of(divy),
                "{name}.{sprite} of {w}x{h} is not cut into {divx}x{divy} whole cells"
            );
            count += 1;
        }
        assert!(count > 0, "{name}.lua holds no sprite");
    }
}
