//! The four pattern functions of `string`, written here so that the budget can stop them.
//!
//! `string.find`, `string.match`, `string.gmatch` and `string.gsub` match by backtracking, and one
//! call can backtrack for longer than the machine will last: `string.find(("a"):rep(40),
//! ("a*"):rep(40) .. "b")` never returns in practice. The interpreter's own versions run inside one
//! instruction, where its hook cannot see them, so a single line of a skin would hang a load or a
//! frame. [`install`] replaces the four with a matcher that counts what it does and settles with
//! the [`Meter`](super::Meter) as it goes.
//!
//! The matcher is the interpreter's own (`lstrlib.c` of Lua 5.2), carried over case by case: the
//! same classes, sets, anchors, captures, position captures, `%b`, `%f` and back references, the
//! same limit of 32 captures and of 200 nested matches, the same messages for a malformed pattern
//! -- each with the file and line of the Lua code that made the call in front, as the C library
//! puts it there -- and the same reading of every edge -- a `^` is an anchor in `find`, `match` and `gsub` and an
//! ordinary character in `gmatch`, a negative `gsub` limit is no limit, and a `-` in a set is a
//! range only between two characters. A pattern and a subject are bytes; a class tests one byte as
//! the C locale would.
//!
//! What is charged:
//!
//! - one step for every position the matcher tries a pattern item at, which is what grows without
//!   bound when a pattern backtracks;
//! - the bytes a single item walks over without trying anything -- the run a `*` swallows before it
//!   backs off, a `%b` span, a back reference, a plain search -- at a fraction of a step each.
//!
//! The total is handed to [`Meter::charge`](super::Meter) in blocks. When the meter refuses, the call
//! raises the budget error, which no `pcall` of the skin's can hold on to. What a `gsub` has built
//! lives outside the interpreter until it is done, so it is held to the interpreter's own memory
//! ceiling and counts as an overrun past it.

use std::cell::Cell;
use std::rc::Rc;

use mlua::{Function, Lua, MultiValue, Table, Value};

use super::budget::budget_error;
use super::{LuaShared, bad_argument_message, coerce};

/// The global table the functions live in.
const STRING_GLOBAL: &str = "string";

/// The character that escapes the next one, in a pattern and in a replacement.
const ESCAPE: u8 = b'%';

/// The characters that make a pattern more than plain text (`SPECIALS`).
const SPECIALS: &[u8] = b"^$*+?.([%-";

/// The character that anchors a pattern to where the search starts.
const ANCHOR: u8 = b'^';

/// Captures one pattern may hold (`LUA_MAXCAPTURES`).
const MAX_CAPTURES: usize = 32;

/// How deep one match may nest (`MAXCCALLS`).
const MAX_MATCH_DEPTH: u32 = 200;

/// What one tried position costs, in the units the work is counted in.
const STEP_COST: u64 = 8;

/// What one byte walked over without trying anything costs.
const BYTE_COST: u64 = 1;

/// How many units of work are charged as one instruction.
const COST_PER_INSTRUCTION: u64 = 8;

/// How much work is run up before it is settled with the meter: a thousand instructions' worth, the
/// block the interpreter's own hook accounts in.
const SETTLE_AT: u64 = 1_000 * COST_PER_INSTRUCTION;

/// The stack level of whoever called the running function.
const CALLER_LEVEL: usize = 1;

/// The byte a pattern or a subject reads as past its end, as the terminator of a C string does.
const END_BYTE: u8 = 0;

const MALFORMED_ESCAPE: &str = "malformed pattern (ends with '%')";
const MALFORMED_SET: &str = "malformed pattern (missing ']')";
const MALFORMED_BALANCE: &str = "malformed pattern (missing arguments to '%b')";
const MALFORMED_FRONTIER: &str = "missing '[' after '%f' in pattern";
const TOO_COMPLEX: &str = "pattern too complex";
const TOO_MANY_CAPTURES: &str = "too many captures";
const INVALID_PATTERN_CAPTURE: &str = "invalid pattern capture";
const INVALID_CAPTURE_INDEX: &str = "invalid capture index";
const UNFINISHED_CAPTURE: &str = "unfinished capture";
const INVALID_REPLACEMENT_ESCAPE: &str = "invalid use of '%' in replacement string";

/// Why a call did not produce its result.
enum Fault {
    /// The meter refused more work, or a result outgrew the memory ceiling.
    Budget,
    /// The pattern, the replacement or an argument is not one. Raised as this message, behind the
    /// position of the caller.
    Malformed(String),
    /// A call back into the interpreter failed.
    Raised(mlua::Error),
}

impl Fault {
    fn malformed(message: &str) -> Self {
        Self::Malformed(message.to_owned())
    }
}

impl From<mlua::Error> for Fault {
    fn from(error: mlua::Error) -> Self {
        Self::Raised(error)
    }
}

/// The work the pattern functions of one interpreter have done and not yet settled with the meter.
///
/// It outlives a call, so a skin that makes many small calls pays for all of them and not only for
/// the ones long enough to fill a block.
struct Work {
    shared: Rc<LuaShared>,
    owed: Cell<u64>,
}

impl Work {
    /// Counts `cost` units of work and settles with the meter once a block has run up.
    fn spend(&self, cost: u64) -> Result<(), Fault> {
        let owed = self.owed.get().saturating_add(cost);
        if owed < SETTLE_AT {
            self.owed.set(owed);
            return Ok(());
        }
        self.owed.set(owed % COST_PER_INSTRUCTION);
        if self.shared.meter.charge(owed / COST_PER_INSTRUCTION) { Ok(()) } else { Err(Fault::Budget) }
    }

    /// Counts `bytes` walked over without trying anything.
    fn spend_bytes(&self, bytes: usize) -> Result<(), Fault> {
        self.spend((bytes as u64).saturating_mul(BYTE_COST))
    }

    /// The error the skin is given for `fault`. An overrun is latched in the meter first, so that
    /// nothing in the skin can catch it.
    fn raise(&self, lua: &Lua, fault: Fault) -> mlua::Error {
        match fault {
            Fault::Budget => {
                self.shared.meter.cut_off();
                budget_error()
            }
            Fault::Malformed(message) => mlua::Error::runtime(format!("{}{message}", caller_position(lua))),
            Fault::Raised(error) => error,
        }
    }
}

/// Where the Lua code that called the running function stands, as the `file:line: ` the C library
/// opens its messages with, or nothing when the caller is not Lua code (`luaL_where`).
fn caller_position(lua: &Lua) -> String {
    let position = lua.inspect_stack(CALLER_LEVEL, |caller| match (caller.source().short_src, caller.current_line()) {
        (Some(source), Some(line)) => format!("{source}:{line}: "),
        _ => String::new(),
    });
    position.unwrap_or_default()
}

/// How far a capture has got.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Extent {
    /// Opened and not yet closed.
    Unfinished,
    /// A position capture, `()`.
    Position,
    /// Closed, with this many bytes.
    Closed(usize),
}

#[derive(Clone, Copy)]
struct Capture {
    start: usize,
    extent: Extent,
}

/// What a capture holds once a match is done.
enum Captured<'a> {
    Text(&'a [u8]),
    /// The one-based position a `()` stood at.
    Position(usize),
}

/// One subject, one pattern, and the captures of the match being tried (`MatchState`).
struct Matcher<'a> {
    source: &'a [u8],
    pattern: &'a [u8],
    work: &'a Work,
    level: usize,
    captures: [Capture; MAX_CAPTURES],
    depth: u32,
}

impl<'a> Matcher<'a> {
    fn new(source: &'a [u8], pattern: &'a [u8], work: &'a Work) -> Self {
        Self { source, pattern, work, level: 0, captures: [Capture { start: 0, extent: Extent::Unfinished }; MAX_CAPTURES], depth: MAX_MATCH_DEPTH }
    }

    /// The pattern byte at `at`, or the terminator past the end.
    fn pattern_byte(&self, at: usize) -> u8 {
        self.pattern.get(at).copied().unwrap_or(END_BYTE)
    }

    /// The subject byte at `at`, or the terminator past the end.
    fn source_byte(&self, at: usize) -> u8 {
        self.source.get(at).copied().unwrap_or(END_BYTE)
    }

    /// Tries the pattern from `pattern_at` against the subject from `source_at` with no captures
    /// open, and answers where the match ends.
    fn try_at(&mut self, source_at: usize, pattern_at: usize) -> Result<Option<usize>, Fault> {
        self.level = 0;
        self.depth = MAX_MATCH_DEPTH;
        self.do_match(source_at, pattern_at)
    }

    /// Where the single item starting at `p` ends (`classend`).
    fn class_end(&self, p: usize) -> Result<usize, Fault> {
        let mut p = p + 1;
        match self.pattern_byte(p - 1) {
            ESCAPE => {
                if p >= self.pattern.len() {
                    return Err(Fault::malformed(MALFORMED_ESCAPE));
                }
                Ok(p + 1)
            }
            b'[' => {
                if self.pattern_byte(p) == b'^' {
                    p += 1;
                }
                loop {
                    if p >= self.pattern.len() {
                        return Err(Fault::malformed(MALFORMED_SET));
                    }
                    let member = self.pattern_byte(p);
                    p += 1;
                    if member == ESCAPE && p < self.pattern.len() {
                        p += 1;
                    }
                    if self.pattern_byte(p) == b']' {
                        return Ok(p + 1);
                    }
                }
            }
            _ => Ok(p),
        }
    }

    /// Whether the set that opens at `p` and closes at `close` holds `byte` (`matchbracketclass`).
    fn set_holds(&self, byte: u8, p: usize, close: usize) -> bool {
        let mut p = p;
        let mut found = true;
        if self.pattern_byte(p + 1) == b'^' {
            found = false;
            p += 1;
        }
        loop {
            p += 1;
            if p >= close {
                return !found;
            }
            if self.pattern_byte(p) == ESCAPE {
                p += 1;
                if class_holds(byte, self.pattern_byte(p)) {
                    return found;
                }
            } else if self.pattern_byte(p + 1) == b'-' && p + 2 < close {
                p += 2;
                if self.pattern_byte(p - 2) <= byte && byte <= self.pattern_byte(p) {
                    return found;
                }
            } else if self.pattern_byte(p) == byte {
                return found;
            }
        }
    }

    /// Whether the subject byte at `s` is one the item from `p` to `end` accepts (`singlematch`).
    fn single_match(&self, s: usize, p: usize, end: usize) -> bool {
        let Some(byte) = self.source.get(s).copied() else {
            return false;
        };
        match self.pattern_byte(p) {
            b'.' => true,
            ESCAPE => class_holds(byte, self.pattern_byte(p + 1)),
            b'[' => self.set_holds(byte, p, end - 1),
            literal => literal == byte,
        }
    }

    /// Where the balanced span that starts at `s` ends, for the two delimiters at `p`
    /// (`matchbalance`).
    fn match_balance(&self, s: usize, p: usize) -> Result<Option<usize>, Fault> {
        if p + 1 >= self.pattern.len() {
            return Err(Fault::malformed(MALFORMED_BALANCE));
        }
        let (open, close) = (self.pattern_byte(p), self.pattern_byte(p + 1));
        if self.source_byte(s) != open {
            return Ok(None);
        }
        let mut depth = 1usize;
        for (offset, byte) in self.source.iter().enumerate().skip(s + 1) {
            if *byte == close {
                depth -= 1;
                if depth == 0 {
                    self.work.spend_bytes(offset - s)?;
                    return Ok(Some(offset + 1));
                }
            } else if *byte == open {
                depth += 1;
            }
        }
        self.work.spend_bytes(self.source.len().saturating_sub(s))?;
        Ok(None)
    }

    /// The longest run of the item first, then ever shorter ones (`max_expand`).
    fn max_expand(&mut self, s: usize, p: usize, end: usize) -> Result<Option<usize>, Fault> {
        let mut count = 0;
        while self.single_match(s + count, p, end) {
            count += 1;
        }
        self.work.spend_bytes(count)?;
        loop {
            if let Some(matched) = self.do_match(s + count, end + 1)? {
                return Ok(Some(matched));
            }
            if count == 0 {
                return Ok(None);
            }
            count -= 1;
        }
    }

    /// The shortest run of the item first, then ever longer ones (`min_expand`).
    fn min_expand(&mut self, s: usize, p: usize, end: usize) -> Result<Option<usize>, Fault> {
        let mut s = s;
        loop {
            if let Some(matched) = self.do_match(s, end + 1)? {
                return Ok(Some(matched));
            }
            if !self.single_match(s, p, end) {
                return Ok(None);
            }
            s += 1;
        }
    }

    /// Opens a capture at `s` and goes on with the pattern (`start_capture`).
    fn start_capture(&mut self, s: usize, p: usize, extent: Extent) -> Result<Option<usize>, Fault> {
        if self.level >= MAX_CAPTURES {
            return Err(Fault::malformed(TOO_MANY_CAPTURES));
        }
        self.captures[self.level] = Capture { start: s, extent };
        self.level += 1;
        let matched = self.do_match(s, p)?;
        if matched.is_none() {
            self.level -= 1;
        }
        Ok(matched)
    }

    /// Closes the innermost open capture at `s` and goes on with the pattern (`end_capture`).
    fn end_capture(&mut self, s: usize, p: usize) -> Result<Option<usize>, Fault> {
        let open = (0..self.level).rev().find(|level| self.captures[*level].extent == Extent::Unfinished);
        let level = open.ok_or_else(|| Fault::malformed(INVALID_PATTERN_CAPTURE))?;
        self.captures[level].extent = Extent::Closed(s - self.captures[level].start);
        let matched = self.do_match(s, p)?;
        if matched.is_none() {
            self.captures[level].extent = Extent::Unfinished;
        }
        Ok(matched)
    }

    /// Where the subject at `s` stops repeating the capture the digit `digit` names
    /// (`match_capture`).
    fn match_capture(&self, s: usize, digit: u8) -> Result<Option<usize>, Fault> {
        let invalid = || Fault::Malformed(format!("{INVALID_CAPTURE_INDEX} %{}", char::from(digit)));
        let index = usize::from(digit).checked_sub(usize::from(b'1')).filter(|index| *index < self.level).ok_or_else(invalid)?;
        let capture = self.captures[index];
        match capture.extent {
            Extent::Unfinished => Err(invalid()),
            Extent::Position => Ok(None),
            Extent::Closed(length) => {
                self.work.spend_bytes(length)?;
                let repeated = self.source.len() - s >= length && self.source[capture.start..capture.start + length] == self.source[s..s + length];
                Ok(repeated.then_some(s + length))
            }
        }
    }

    /// Matches the pattern from `p` against the subject from `s` and answers where the match ends
    /// (`match`).
    fn do_match(&mut self, s: usize, p: usize) -> Result<Option<usize>, Fault> {
        if self.depth == 0 {
            return Err(Fault::malformed(TOO_COMPLEX));
        }
        self.depth -= 1;
        let matched = self.match_items(s, p)?;
        self.depth += 1;
        Ok(matched)
    }

    /// The body of [`Self::do_match`]: one pattern item per turn of the loop, for as long as the
    /// rest of the match is the match of the rest of the pattern.
    fn match_items(&mut self, s: usize, p: usize) -> Result<Option<usize>, Fault> {
        let (mut s, mut p) = (s, p);
        loop {
            self.work.spend(STEP_COST)?;
            if p >= self.pattern.len() {
                return Ok(Some(s));
            }
            match self.pattern_byte(p) {
                b'(' if self.pattern_byte(p + 1) == b')' => return self.start_capture(s, p + 2, Extent::Position),
                b'(' => return self.start_capture(s, p + 1, Extent::Unfinished),
                b')' => return self.end_capture(s, p + 1),
                b'$' if p + 1 == self.pattern.len() => return Ok((s == self.source.len()).then_some(s)),
                ESCAPE => match self.pattern_byte(p + 1) {
                    b'b' => match self.match_balance(s, p + 2)? {
                        Some(after) => {
                            s = after;
                            p += 4;
                            continue;
                        }
                        None => return Ok(None),
                    },
                    b'f' => {
                        p += 2;
                        if self.pattern_byte(p) != b'[' {
                            return Err(Fault::malformed(MALFORMED_FRONTIER));
                        }
                        let end = self.class_end(p)?;
                        let previous = if s == 0 { END_BYTE } else { self.source_byte(s - 1) };
                        if !self.set_holds(previous, p, end - 1) && self.set_holds(self.source_byte(s), p, end - 1) {
                            p = end;
                            continue;
                        }
                        return Ok(None);
                    }
                    digit @ b'0'..=b'9' => match self.match_capture(s, digit)? {
                        Some(after) => {
                            s = after;
                            p += 2;
                            continue;
                        }
                        None => return Ok(None),
                    },
                    _ => {}
                },
                _ => {}
            }

            let end = self.class_end(p)?;
            let suffix = self.pattern_byte(end);
            if !self.single_match(s, p, end) {
                if matches!(suffix, b'*' | b'?' | b'-') {
                    p = end + 1;
                    continue;
                }
                return Ok(None);
            }
            match suffix {
                b'?' => {
                    if let Some(matched) = self.do_match(s + 1, end + 1)? {
                        return Ok(Some(matched));
                    }
                    p = end + 1;
                }
                b'+' => return self.max_expand(s + 1, p, end),
                b'*' => return self.max_expand(s, p, end),
                b'-' => return self.min_expand(s, p, end),
                _ => {
                    s += 1;
                    p = end;
                }
            }
        }
    }

    /// What capture `index` holds for the match from `start` to `end`. With no captures at all the
    /// first one is the whole match (`push_onecapture`).
    fn captured(&self, index: usize, start: usize, end: usize) -> Result<Captured<'a>, Fault> {
        if index >= self.level {
            return if index == 0 { Ok(Captured::Text(&self.source[start..end])) } else { Err(Fault::malformed(INVALID_CAPTURE_INDEX)) };
        }
        let capture = self.captures[index];
        match capture.extent {
            Extent::Unfinished => Err(Fault::malformed(UNFINISHED_CAPTURE)),
            Extent::Position => Ok(Captured::Position(capture.start + 1)),
            Extent::Closed(length) => Ok(Captured::Text(&self.source[capture.start..capture.start + length])),
        }
    }

    /// Capture `index` as a value of the interpreter.
    fn capture_value(&self, lua: &Lua, index: usize, start: usize, end: usize) -> Result<Value, Fault> {
        Ok(match self.captured(index, start, end)? {
            Captured::Text(text) => Value::String(lua.create_string(text)?),
            Captured::Position(position) => Value::Integer(position as mlua::Integer),
        })
    }

    /// Every capture as a value. `whole` is the match itself, which stands in when the pattern has
    /// no captures; `find` passes none and so answers no capture then (`push_captures`).
    fn capture_values(&self, lua: &Lua, whole: Option<(usize, usize)>) -> Result<Vec<Value>, Fault> {
        let count = if self.level == 0 && whole.is_some() { 1 } else { self.level };
        let (start, end) = whole.unwrap_or_default();
        (0..count).map(|index| self.capture_value(lua, index, start, end)).collect()
    }
}

/// Whether `byte` is in the class the letter `class` names; any other character is itself
/// (`match_class`).
fn class_holds(byte: u8, class: u8) -> bool {
    let held = match class.to_ascii_lowercase() {
        b'a' => byte.is_ascii_alphabetic(),
        b'c' => byte.is_ascii_control(),
        b'd' => byte.is_ascii_digit(),
        b'g' => byte.is_ascii_graphic(),
        b'l' => byte.is_ascii_lowercase(),
        b'p' => byte.is_ascii_punctuation(),
        b's' => byte == b' ' || (b'\t'..=b'\r').contains(&byte),
        b'u' => byte.is_ascii_uppercase(),
        b'w' => byte.is_ascii_alphanumeric(),
        b'x' => byte.is_ascii_hexdigit(),
        b'z' => byte == END_BYTE,
        _ => return class == byte,
    };
    held == class.is_ascii_lowercase()
}

/// Where `needle` first stands in `haystack` (`lmemfind`).
fn find_plain(haystack: &[u8], needle: &[u8], work: &Work) -> Result<Option<usize>, Fault> {
    let Some((first, rest)) = needle.split_first() else {
        return Ok(Some(0));
    };
    let Some(last_start) = haystack.len().checked_sub(needle.len()) else {
        return Ok(None);
    };
    let mut start = 0;
    while start <= last_start {
        let Some(offset) = haystack[start..=last_start].iter().position(|byte| byte == first) else {
            work.spend_bytes(last_start + 1 - start)?;
            return Ok(None);
        };
        start += offset;
        let window = &haystack[start + 1..start + needle.len()];
        let agreed = window.iter().zip(rest).take_while(|(found, wanted)| found == wanted).count();
        work.spend_bytes(offset + agreed + 1)?;
        if agreed == rest.len() {
            return Ok(Some(start));
        }
        start += 1;
    }
    Ok(None)
}

/// A string argument, which may also be given as a number (`luaL_checklstring`).
fn text_argument(lua: &Lua, value: Value, position: usize, function: &str) -> Result<mlua::LuaString, Fault> {
    let kind = value.type_name();
    let text = match value {
        Value::String(_) | Value::Integer(_) | Value::Number(_) => lua.coerce_string(value)?,
        _ => None,
    };
    text.ok_or_else(|| Fault::Malformed(bad_argument_message(position, function, &format!("string expected, got {kind}"))))
}

/// An optional integer argument: a number, or a string that spells one, cut toward zero as the
/// interpreter cuts it (`luaL_optinteger`).
fn integer_argument(lua: &Lua, value: Value, position: usize, function: &str) -> Result<Option<mlua::Integer>, Fault> {
    if value.is_nil() {
        return Ok(None);
    }
    let kind = value.type_name();
    match lua.coerce_number(value)? {
        Some(number) => Ok(Some(number as mlua::Integer)),
        None => Err(Fault::Malformed(bad_argument_message(position, function, &format!("number expected, got {kind}")))),
    }
}

/// A one-based position that may count from the end, as an offset from the start (`posrelat`).
fn position_from(position: mlua::Integer, length: usize) -> usize {
    let distance = usize::try_from(position.unsigned_abs()).unwrap_or(usize::MAX);
    if position >= 0 {
        return distance;
    }
    if distance > length { 0 } else { length - distance + 1 }
}

/// `string.find` and `string.match` (`str_find_aux`).
fn find(lua: &Lua, work: &Work, arguments: (Value, Value, Value, Value), function: &str, wants_positions: bool) -> Result<MultiValue, Fault> {
    let (source, pattern, init, plain) = arguments;
    let source = text_argument(lua, source, 1, function)?;
    let pattern = text_argument(lua, pattern, 2, function)?;
    let init = integer_argument(lua, init, 3, function)?.unwrap_or(1);
    let (source, pattern) = (source.as_bytes(), pattern.as_bytes());
    let (source, pattern): (&[u8], &[u8]) = (&source, &pattern);

    let first = position_from(init, source.len()).max(1);
    if first > source.len() + 1 {
        return Ok(MultiValue::from_vec(vec![Value::Nil]));
    }
    let found = if wants_positions && (coerce::to_boolean(&plain) || !pattern.iter().any(|byte| SPECIALS.contains(byte))) {
        find_plain(&source[first - 1..], pattern, work)?.map(|offset| {
            let start = first + offset;
            vec![Value::Integer(start as mlua::Integer), Value::Integer((start + pattern.len() - 1) as mlua::Integer)]
        })
    } else {
        search(lua, work, source, pattern, first - 1, wants_positions)?
    };
    Ok(MultiValue::from_vec(found.unwrap_or_else(|| vec![Value::Nil])))
}

/// The first match of `pattern` at or after `from`, as the values `find` or `match` answers.
fn search(lua: &Lua, work: &Work, source: &[u8], pattern: &[u8], from: usize, wants_positions: bool) -> Result<Option<Vec<Value>>, Fault> {
    let (anchored, pattern) = match pattern.split_first() {
        Some((&ANCHOR, rest)) => (true, rest),
        _ => (false, pattern),
    };
    let mut matcher = Matcher::new(source, pattern, work);
    let mut start = from;
    loop {
        if let Some(end) = matcher.try_at(start, 0)? {
            if !wants_positions {
                return matcher.capture_values(lua, Some((start, end))).map(Some);
            }
            let mut values = vec![Value::Integer((start + 1) as mlua::Integer), Value::Integer(end as mlua::Integer)];
            values.extend(matcher.capture_values(lua, None)?);
            return Ok(Some(values));
        }
        if anchored || start >= source.len() {
            return Ok(None);
        }
        start += 1;
    }
}

/// `string.gmatch`: an iterator over every match (`gmatch`, `gmatch_aux`).
fn gmatch(lua: &Lua, work: &Rc<Work>, arguments: (Value, Value)) -> Result<Function, Fault> {
    let source = text_argument(lua, arguments.0, 1, "gmatch")?;
    let pattern = text_argument(lua, arguments.1, 2, "gmatch")?;
    let work = Rc::clone(work);
    let resume_at = Cell::new(0usize);
    let next = lua.create_function(move |lua, ()| {
        let (source, pattern) = (source.as_bytes(), pattern.as_bytes());
        next_match(lua, &work, &source, &pattern, &resume_at).map_err(|fault| work.raise(lua, fault))
    })?;
    Ok(next)
}

/// One step of a `gmatch` iterator: the captures of the next match at or after `resume_at`, or
/// nothing when there is none (`gmatch_aux`).
fn next_match(lua: &Lua, work: &Work, source: &[u8], pattern: &[u8], resume_at: &Cell<usize>) -> Result<MultiValue, Fault> {
    let mut matcher = Matcher::new(source, pattern, work);
    let mut start = resume_at.get();
    while start <= source.len() {
        if let Some(end) = matcher.try_at(start, 0)? {
            resume_at.set(if end == start { end + 1 } else { end });
            return matcher.capture_values(lua, Some((start, end))).map(MultiValue::from_vec);
        }
        start += 1;
    }
    Ok(MultiValue::new())
}

/// What `gsub` puts in place of a match.
enum Replacement {
    /// A string, in which `%0` to `%9` stand for the match and its captures.
    Text(mlua::LuaString),
    /// A function called with the captures.
    Call(Function),
    /// A table indexed with the first capture.
    Lookup(Table),
}

/// What a `gsub` has built so far. It is not the interpreter's memory yet, so it is held to what the
/// interpreter has left of its own ceiling.
struct Output {
    bytes: Vec<u8>,
    room: usize,
}

impl Output {
    fn push(&mut self, bytes: &[u8]) -> Result<(), Fault> {
        if bytes.len() > self.room - self.bytes.len() {
            return Err(Fault::Budget);
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
}

/// Adds a replacement string for the match from `start` to `end` (`add_s`).
fn add_text(matcher: &Matcher, output: &mut Output, replacement: &[u8], start: usize, end: usize) -> Result<(), Fault> {
    let mut at = 0;
    while at < replacement.len() {
        let byte = replacement[at];
        at += 1;
        if byte != ESCAPE {
            output.push(&[byte])?;
            continue;
        }
        let escaped = replacement.get(at).copied().unwrap_or(END_BYTE);
        at += 1;
        if !escaped.is_ascii_digit() {
            if escaped != ESCAPE {
                return Err(Fault::malformed(INVALID_REPLACEMENT_ESCAPE));
            }
            output.push(&[escaped])?;
        } else if escaped == b'0' {
            output.push(&matcher.source[start..end])?;
        } else {
            match matcher.captured(usize::from(escaped - b'1'), start, end)? {
                Captured::Text(text) => output.push(text)?,
                Captured::Position(position) => output.push(position.to_string().as_bytes())?,
            }
        }
    }
    Ok(())
}

/// Adds what stands in for the match from `start` to `end` (`add_value`).
fn add_replacement(lua: &Lua, matcher: &Matcher, output: &mut Output, replacement: &Replacement, start: usize, end: usize) -> Result<(), Fault> {
    let value = match replacement {
        Replacement::Text(text) => return add_text(matcher, output, &text.as_bytes(), start, end),
        Replacement::Call(function) => function.call::<Value>(MultiValue::from_vec(matcher.capture_values(lua, Some((start, end)))?))?,
        Replacement::Lookup(table) => table.get::<Value>(matcher.capture_value(lua, 0, start, end)?)?,
    };
    if !coerce::to_boolean(&value) {
        return output.push(&matcher.source[start..end]);
    }
    let kind = value.type_name();
    let text = match value {
        Value::String(_) | Value::Integer(_) | Value::Number(_) => lua.coerce_string(value)?,
        _ => None,
    };
    match text {
        Some(text) => output.push(&text.as_bytes()),
        None => Err(Fault::Malformed(format!("invalid replacement value (a {kind})"))),
    }
}

/// Replaces up to `limit` matches of the matcher's pattern, leaving the result in `output`, and
/// answers how many it replaced (the loop of `str_gsub`).
fn substitute(lua: &Lua, matcher: &mut Matcher, output: &mut Output, replacement: &Replacement, limit: usize, anchored: bool) -> Result<usize, Fault> {
    let source = matcher.source;
    let mut replaced = 0;
    let mut at = 0;
    while replaced < limit {
        let matched = matcher.try_at(at, 0)?;
        if let Some(end) = matched {
            replaced += 1;
            add_replacement(lua, matcher, output, replacement, at, end)?;
        }
        match matched {
            Some(end) if end > at => at = end,
            _ if at < source.len() => {
                output.push(&source[at..=at])?;
                at += 1;
            }
            _ => break,
        }
        if anchored {
            break;
        }
    }
    output.push(&source[at..])?;
    Ok(replaced)
}

/// `string.gsub` (`str_gsub`).
fn gsub(lua: &Lua, work: &Work, arguments: (Value, Value, Value, Value)) -> Result<(mlua::LuaString, usize), Fault> {
    let (source, pattern, replacement, limit) = arguments;
    let source = text_argument(lua, source, 1, "gsub")?;
    let pattern = text_argument(lua, pattern, 2, "gsub")?;
    let limit = integer_argument(lua, limit, 4, "gsub")?;
    let replacement = match replacement {
        Value::Function(function) => Replacement::Call(function),
        Value::Table(table) => Replacement::Lookup(table),
        text @ (Value::String(_) | Value::Integer(_) | Value::Number(_)) => Replacement::Text(text_argument(lua, text, 3, "gsub")?),
        _ => return Err(Fault::Malformed(bad_argument_message(3, "gsub", "string/function/table expected"))),
    };
    let (source, pattern) = (source.as_bytes(), pattern.as_bytes());
    let (source, pattern): (&[u8], &[u8]) = (&source, &pattern);
    let limit = limit.map_or(source.len() + 1, |limit| usize::try_from(limit).unwrap_or(usize::MAX));
    let (anchored, pattern) = match pattern.split_first() {
        Some((&ANCHOR, rest)) => (true, rest),
        _ => (false, pattern),
    };

    let mut output = Output { bytes: Vec::new(), room: work.shared.meter.budget().max_memory_bytes.saturating_sub(lua.used_memory()) };
    let mut matcher = Matcher::new(source, pattern, work);
    let replaced = substitute(lua, &mut matcher, &mut output, &replacement, limit, anchored)?;
    Ok((lua.create_string(&output.bytes)?, replaced))
}

/// Replaces `string.find`, `string.match`, `string.gmatch` and `string.gsub` with the metered ones.
///
/// The table is the one the string metatable indexes, so `("x"):find(...)` reaches them too, and
/// this runs before any other library captures a pattern function in a local.
pub(crate) fn install(lua: &Lua, shared: &Rc<LuaShared>) -> mlua::Result<()> {
    let string: Table = lua.globals().get(STRING_GLOBAL)?;
    let work = Rc::new(Work { shared: Rc::clone(shared), owed: Cell::new(0) });

    let finder = Rc::clone(&work);
    let find_positions = move |lua: &Lua, arguments| find(lua, &finder, arguments, "find", true).map_err(|fault| finder.raise(lua, fault));
    string.set("find", lua.create_function(find_positions)?)?;
    let matcher = Rc::clone(&work);
    let find_captures = move |lua: &Lua, arguments| find(lua, &matcher, arguments, "match", false).map_err(|fault| matcher.raise(lua, fault));
    string.set("match", lua.create_function(find_captures)?)?;
    let iterator = Rc::clone(&work);
    string.set("gmatch", lua.create_function(move |lua, arguments| gmatch(lua, &iterator, arguments).map_err(|fault| iterator.raise(lua, fault)))?)?;
    string.set("gsub", lua.create_function(move |lua, arguments| gsub(lua, &work, arguments).map_err(|fault| work.raise(lua, fault)))?)?;
    Ok(())
}
