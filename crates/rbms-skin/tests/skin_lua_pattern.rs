//! The pattern functions a skin is given, held against the interpreter's own.
//!
//! A skin gets `string.find`, `string.match`, `string.gmatch` and `string.gsub` from this crate, not
//! from the C library, so that the budget can stop one that backtracks without end. They have to
//! answer exactly what the C ones answer. These tests run the same Lua driver in a skin's
//! interpreter and in a plain one that still has the C functions, and compare every answer -- the
//! values, their types, and the message of every error -- over a table of patterns and subjects
//! chosen to reach each branch of the matcher, and over a few thousand generated ones.

#![cfg(feature = "lua")]

use std::path::Path;

use mlua::{Function, Lua};
use rbms_skin::lua::{SkinLua, SkinLuaConfig};

/// The seed the skin's interpreter is built with. No pattern function draws from it.
const TEST_SEED: u64 = 11;

/// How many generated cases the fuzz comparison runs.
const GENERATED_CASES: i64 = 4_000;

/// The driver both interpreters run. It returns three functions: how many patterns and subjects the
/// table holds, the answers of every call shape for one pattern and one subject of the table, and
/// the answers for one generated case.
///
/// Every answer is written as `type=value` so that a number and the string that spells it differ.
/// The generator is a linear congruential one written out in Lua, so both interpreters draw the same
/// cases whatever their own `math.random` does.
const DRIVER: &str = r##"
local find, match, gmatch, gsub = string.find, string.match, string.gmatch, string.gsub
local select, type, tostring, pcall, concat = select, type, tostring, pcall, table.concat

local function show(...)
    local parts = {}
    for index = 1, select("#", ...) do
        local value = select(index, ...)
        parts[index] = type(value) .. "=" .. tostring(value)
    end
    return concat(parts, "|")
end

local function every(subject, pattern)
    local found = {}
    for first, second in gmatch(subject, pattern) do
        found[#found + 1] = show(first, second)
        if #found >= 40 then
            break
        end
    end
    return concat(found, ";")
end

local patterns = {
    "", "a", "a*", "a+", "a-", "a?", ".", ".*", ".-", ".+", "^a", "a$", "^$", "$", "^", "^^a", "a$b", "$a",
    "%a+", "%d+", "%s*", "%w+", "%p", "%x+", "%u", "%l+", "%c", "%g+", "%z", "%A+", "%D", "%S+", "%W", "%L", "%U", "%P+", "%X", "%C+", "%G",
    "[abc]", "[^abc]", "[a-c]+", "[%a_][%w_]*", "[]]", "[^]]", "[a-]", "[-a]", "[%]]", "[z-a]", "[a-c-e]", "[%a-z]", "[^%s]+", "[.]", "[%.]",
    "(a)(b)", "((a)(b))", "()", "()a()", "(a*(.)%w(%s*))", "(.)%1", "(a)(b)%2%1", "(x*)", "(%w+)=(%w+)", "([^,]*),?", "^(.-)%s*$", "^(%s*)",
    "%b()", "%b<>", "%bxx", "%b()%b()", "%f[%a]%a+", "%f[%w]%w+%f[%W]", ".%f[%z]", "%f[^%z]", "%f[%s]",
    "x*", ".-b", "a.-b", "a.*b", "%%", "%.", "a%-b", "[%w%-]+", "%s+$", "a**", "a?+", "(a*)*", "a?a?a?aaa", ".?.?.?$", "(a?)(a?)(a?)",
    "\0", "a\0b", "[\0]", ".\0", "%z+", "[^\0]+",
    "%", "[a", "[", "[^", "(", ")", "(()", "%1", "(a)%2", "%0", "(a%1)", "%b", "%bx", "%f", "%fa", "%f[a", "[a-", "a(", "(a", "[%", "a%",
    ("()"):rep(32), ("()"):rep(33), ("(a)"):rep(32) .. "%9",
}

local subjects = {
    "", "a", "aaa", "abc", "b", "hello world", "key=value", "a,b,,c", "  padded  ", "(foo(bar))baz", "<a><b>", "aXb", "THE (quick) fox",
    "x\0y", "\0", "a-b-c", "abab", "aa bb cc", "%", "]", "[", "^a$", "skin/Play/parts/frame.png", "123 abc 0x1F", "line1\nline2\r\n", "\t\v\f\r\n ",
    "\195\169t\195\169", "\255\254\128", "xxyxx", "a.b", ("a"):rep(60), "((()))", "())(",
}

local inits = { 1, 2, 3, -1, -3, 0, 10, 100, -100, "2", 1.9 }
local limits = { 0, 1, 2, -1, 100 }
local replacements = {
    "<%0>", "%1", "%2", "%1%1", "x", "%%", "%", "", "%a", 7,
    function(first, second) return second or first end,
    function() return false end,
    function() return {} end,
    function(first) return 3 end,
    { a = "A", b = false, [1] = "one", abc = 12 },
}

local function probe(pattern_index, subject_index)
    local pattern, subject = patterns[pattern_index], subjects[subject_index]
    local answers = {
        show(pcall(find, subject, pattern)),
        show(pcall(find, subject, pattern, 1, true)),
        show(pcall(match, subject, pattern)),
        show(pcall(every, subject, pattern)),
    }
    for index = 1, #inits do
        answers[#answers + 1] = show(pcall(find, subject, pattern, inits[index]))
        answers[#answers + 1] = show(pcall(match, subject, pattern, inits[index]))
        answers[#answers + 1] = show(pcall(find, subject, pattern, inits[index], true))
    end
    for index = 1, #replacements do
        answers[#answers + 1] = show(pcall(gsub, subject, pattern, replacements[index]))
    end
    for index = 1, #limits do
        answers[#answers + 1] = show(pcall(gsub, subject, pattern, "[%0]", limits[index]))
    end
    return concat(answers, "\n")
end

local atoms = {
    "a", "b", "c", ".", "%a", "%d", "%s", "%w", "%p", "%A", "%S", "[ab]", "[^a]", "[a-c]", "[%d%s]", "[]a]",
    "(", ")", "()", "*", "+", "-", "?", "^", "$", "%b()", "%bab", "%f[%a]", "%f[^b]", "%1", "%2", "%%", "%", "[", "]", " ", "1", "(",
}
local letters = { "a", "b", "c", "a", "b", " ", "1", "(", ")", "a", "%", "]", "\n", "A" }

local function generated(case)
    local state = (case * 7919 + 104729) % 2147483647
    local function draw(bound)
        state = (state * 48271) % 2147483647
        return state % bound + 1
    end
    local pattern, subject = {}, {}
    for index = 1, draw(6) do
        pattern[index] = atoms[draw(#atoms)]
    end
    for index = 1, draw(12) - 1 do
        subject[index] = letters[draw(#letters)]
    end
    pattern, subject = concat(pattern), concat(subject)
    local replacement = replacements[draw(4)]
    local answers = {
        show(pcall(find, subject, pattern)),
        show(pcall(find, subject, pattern, draw(6) - 3)),
        show(pcall(match, subject, pattern)),
        show(pcall(every, subject, pattern)),
        show(pcall(gsub, subject, pattern, replacement)),
        show(pcall(gsub, subject, pattern, "%0%0", draw(3) - 1)),
    }
    return pattern, subject, concat(answers, "\n")
end

return #patterns, #subjects, probe, generated
"##;

/// A skin's interpreter, whose pattern functions are this crate's.
fn skin() -> SkinLua {
    SkinLua::new(SkinLuaConfig { seed: Some(TEST_SEED), ..SkinLuaConfig::new(Path::new(env!("CARGO_MANIFEST_DIR"))) }).expect("the runtime builds")
}

/// The driver's results in one interpreter: the two sizes of the table and the two functions.
fn driver(lua: &Lua) -> (i64, i64, Function, Function) {
    lua.load(DRIVER).eval().expect("the driver loads")
}

#[test]
fn every_call_shape_answers_what_the_interpreters_own_function_answers() {
    let skin = skin();
    let reference = Lua::new();
    let (patterns, subjects, ours, _) = driver(skin.lua());
    let (reference_patterns, reference_subjects, theirs, _) = driver(&reference);
    assert_eq!((patterns, subjects), (reference_patterns, reference_subjects));
    assert!(patterns > 100 && subjects > 30, "the table is the size it was written at");

    for pattern in 1..=patterns {
        for subject in 1..=subjects {
            let expected: mlua::LuaString = theirs.call((pattern, subject)).expect("the reference driver runs");
            let answered: mlua::LuaString = ours.call((pattern, subject)).expect("the skin's driver runs");
            assert_eq!(
                answered.to_string_lossy(),
                expected.to_string_lossy(),
                "pattern #{pattern} against subject #{subject} answers differently from the interpreter's own function"
            );
        }
    }
}

#[test]
fn generated_patterns_answer_what_the_interpreters_own_function_answers() {
    let skin = skin();
    let reference = Lua::new();
    let (_, _, _, ours) = driver(skin.lua());
    let (_, _, _, theirs) = driver(&reference);

    for case in 1..=GENERATED_CASES {
        let (pattern, subject, expected): (mlua::LuaString, mlua::LuaString, mlua::LuaString) = theirs.call(case).expect("the reference driver runs");
        let (_, _, answered): (mlua::LuaString, mlua::LuaString, mlua::LuaString) = ours.call(case).expect("the skin's driver runs");
        assert_eq!(
            answered.to_string_lossy(),
            expected.to_string_lossy(),
            "case {case}: pattern {:?} against subject {:?}",
            pattern.to_string_lossy(),
            subject.to_string_lossy()
        );
    }
}

#[test]
fn a_match_nests_as_deep_as_the_interpreters_own_and_no_deeper() {
    let skin = skin();
    let reference = Lua::new();
    for source in [
        "return tostring(select(2, pcall(string.find, ('a'):rep(150), ('a?'):rep(150))))",
        "return tostring(select(2, pcall(string.find, ('a'):rep(199), ('a?'):rep(199))))",
        "return tostring(select(2, pcall(string.find, ('a'):rep(200), ('a?'):rep(200))))",
        "return tostring(select(2, pcall(string.find, ('a'):rep(250), ('a?'):rep(250))))",
        "return tostring(select(2, pcall(string.match, ('a'):rep(250), ('(a?)'):rep(31))))",
    ] {
        let expected: String = reference.load(source).eval().expect("the reference runs");
        let answered: String = skin.lua().load(source).eval().expect("the skin's interpreter runs");
        assert_eq!(answered, expected, "{source}");
    }
    let refused: String = skin.lua().load("return select(2, pcall(string.find, ('a'):rep(250), ('a?'):rep(250)))").eval().expect("the chunk runs");
    assert_eq!(refused, "pattern too complex");
}

#[test]
fn a_method_call_and_a_number_reach_the_same_functions() {
    let skin = skin();
    let eval = |source: &str| -> String { skin.lua().load(source).eval().unwrap_or_else(|error| panic!("{source} should run: {error}")) };
    assert_eq!(eval("return ('a,b'):gsub(',', ';')"), "a;b");
    assert_eq!(eval("return ('key=value'):match('=(.*)')"), "value");
    assert_eq!(eval("return tostring(('abc'):find('c', 1, true))"), "3");
    assert_eq!(eval("local out = {} for word in ('x y'):gmatch('%a') do out[#out + 1] = word end return table.concat(out)"), "xy");
    assert_eq!(eval("return (string.gsub(1234, 23, 'x'))"), "1x4", "a number is a subject and a pattern as its text");
    assert_eq!(eval("return (select(2, pcall(string.find)))"), "bad argument #1 to 'find' (string expected, got nil)");
    assert_eq!(eval("return (select(2, pcall(string.gsub, 'a', 'a', true)))"), "bad argument #3 to 'gsub' (string/function/table expected)");
    assert_eq!(eval("return (select(2, pcall(string.find, 'a', 'a', {})))"), "bad argument #3 to 'find' (number expected, got table)");
    assert_eq!(eval("return require('string').find == string.find and 'same' or 'different'"), "same");
}

#[test]
fn an_error_inside_a_replacement_function_reaches_the_skin_as_it_was_raised() {
    let skin = skin();
    let (ok, message): (bool, String) =
        skin.lua().load("return pcall(string.gsub, 'abc', '%w', function(letter) error('no ' .. letter, 0) end)").eval().expect("the chunk runs");
    assert!(!ok);
    assert_eq!(message, "no a");
}
