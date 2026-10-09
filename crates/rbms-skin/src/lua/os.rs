//! The `os` library, cut down to the clock and the calendar.
//!
//! The reference removes everything that touches the machine and keeps the rest
//! (`SkinLuaAccessor.SafeOsLib`). What is left is exactly `clock`, `date`, `difftime`, `time` and
//! `setlocale`; `execute`, `exit`, `getenv`, `remove`, `rename` and `tmpname` do not exist.
//!
//! The calendar needs the machine's time zone, which nothing in this crate can read without the C
//! library. So the C `os` library is opened once, the functions that only read the calendar are
//! taken out of it, and its table is emptied before a skin ever runs: the table a skin sees is a
//! new one holding the five functions and nothing else.
//!
//! - `os.date` is the C library's. It formats in the machine's local time zone, not UTC: skins name
//!   a day's history folder with `os.date("%y%m%d")` and read `os.date("*t").wday`. A leading `!`
//!   selects UTC.
//! - `os.difftime` is the C library's.
//! - `os.time(table)` is the C library's and converts a local date table to whole seconds.
//!   `os.time()` is the reference's instead: the reference answers the wall clock in seconds *with
//!   its milliseconds as a fraction* (`OsLib.time`), and a skin that subtracts two such readings
//!   depends on the fraction.
//! - `os.clock()` is the reference's too. It does not answer processor time: it answers the seconds
//!   of wall clock since the library was first loaded, to the millisecond (`OsLib.clock`). The
//!   origin is one per process, so the value keeps growing from one skin to the next.
//! - `os.setlocale` changes nothing and answers `"C"` whatever it is asked (`OsLib.setlocale`). The
//!   C function is not exposed, because a locale is process-wide and a skin that set one would
//!   change how every other part of the program reads and prints numbers.
//!
//! The host's own clock comes from here too ([`local_time`]). The reference reads the date and time
//! a skin's number objects show from the same default time zone its `os.date` formats in, so the two
//! are one clock; reading both through the C library keeps them one here.
//!
//! Known differences from the reference, all in `os.date`: `%c` pads the day with a blank where the
//! reference pads with a zero, `%I` counts 01 to 12 where the reference counts 00 to 11, and the
//! conversions the reference does not know are formatted rather than refused.

use std::rc::Rc;
use std::sync::OnceLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use mlua::{Function, Lua, LuaOptions, MultiValue, StdLib, Table, Value};

use super::{LuaShared, package};

/// The global, and the module name, the library is published under.
const OS_GLOBAL: &str = "os";

/// The C library's function that reads the calendar.
const NATIVE_DATE: &str = "date";

/// The functions taken from the C library as they are.
const NATIVE_FUNCTIONS: [&str; 2] = [NATIVE_DATE, "difftime"];

/// The format that makes `os.date` answer a table of the date's fields in the local time zone.
const LOCAL_FIELDS_FORMAT: &str = "*t";

/// The C library's function behind `os.time(table)`.
const NATIVE_TIME: &str = "time";

/// The only locale there is.
const LOCALE: &str = "C";

/// Milliseconds in a second, the precision of both clocks.
const MILLIS_PER_SECOND: f64 = 1_000.0;

/// The moment `os.clock` counts from: the first time any interpreter of this process was given the
/// library.
static CLOCK_ORIGIN: OnceLock<Instant> = OnceLock::new();

/// A span in seconds, to the millisecond.
fn seconds(span: std::time::Duration) -> f64 {
    span.as_millis() as f64 / MILLIS_PER_SECOND
}

/// A date and a time of day in the machine's local time zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalTime {
    pub year: i32,
    /// 1 to 12.
    pub month: i32,
    /// 1 to 31.
    pub day: i32,
    pub hour: i32,
    pub minute: i32,
    pub second: i32,
}

thread_local! {
    /// The interpreter [`local_time`] reads the calendar through: one that holds the C `os` library
    /// and nothing else, and that no skin ever runs in. `None` when it could not be made.
    static CALENDAR: Option<Lua> = Lua::new_with(StdLib::OS, LuaOptions::default()).ok();
}

/// The date and time `unix_seconds` after the epoch fall on in the machine's local time zone, or
/// `None` when the C library has no date for that moment.
///
/// This is `os.date("*t", unix_seconds)` as a skin's own script would read it, so the clock a host
/// shows through its number properties and the one a script formats never disagree.
pub fn local_time(unix_seconds: i64) -> Option<LocalTime> {
    CALENDAR.with(|calendar| {
        let os: Table = calendar.as_ref()?.globals().get(OS_GLOBAL).ok()?;
        let fields: Table = os.get::<Function>(NATIVE_DATE).ok()?.call((LOCAL_FIELDS_FORMAT, unix_seconds)).ok()?;
        let field = |name: &str| fields.get::<i32>(name).ok();
        Some(LocalTime { year: field("year")?, month: field("month")?, day: field("day")?, hour: field("hour")?, minute: field("min")?, second: field("sec")? })
    })
}

/// Publishes the `os` library.
pub(crate) fn install(lua: &Lua, _shared: &Rc<LuaShared>) -> mlua::Result<()> {
    lua.load_std_libs(StdLib::OS)?;
    let globals = lua.globals();
    let native: Table = globals.get(OS_GLOBAL)?;
    let os = lua.create_table()?;
    for name in NATIVE_FUNCTIONS {
        os.set(name, native.get::<Function>(name)?)?;
    }

    let native_time: Function = native.get(NATIVE_TIME)?;
    os.set(
        "time",
        lua.create_function(move |_, date: Value| match date {
            Value::Nil => Ok(Value::Number(SystemTime::now().duration_since(UNIX_EPOCH).map_or(0.0, seconds))),
            date => native_time.call(date),
        })?,
    )?;

    let origin = *CLOCK_ORIGIN.get_or_init(Instant::now);
    os.set("clock", lua.create_function(move |_, ()| Ok(seconds(origin.elapsed())))?)?;
    os.set("setlocale", lua.create_function(|_, _: MultiValue| Ok(LOCALE))?)?;

    native.clear()?;
    globals.set(OS_GLOBAL, &os)?;
    package::preload(lua, OS_GLOBAL, Value::Table(os))
}

#[cfg(test)]
mod tests {
    use mlua::Lua;

    use super::local_time;

    /// A moment well away from any change of clocks: 2023-11-14 22:13:20 UTC.
    const SAMPLE_MOMENT: i64 = 1_700_000_000;

    #[test]
    fn the_local_time_is_the_one_a_script_reads_from_os_date() {
        let at = local_time(SAMPLE_MOMENT).expect("the C library has a date for an ordinary moment");
        let lua = Lua::new();
        lua.globals().set("MOMENT", SAMPLE_MOMENT).expect("a global is set");
        lua.globals().set("AT", [at.year, at.month, at.day, at.hour, at.minute, at.second]).expect("a global is set");

        let read: (i32, i32, i32, i32, i32, i32) =
            lua.load("local t = os.date('*t', MOMENT) return t.year, t.month, t.day, t.hour, t.min, t.sec").eval().expect("os.date answers a table");
        assert_eq!((at.year, at.month, at.day, at.hour, at.minute, at.second), read);

        let back: i64 = lua
            .load("return os.time({ year = AT[1], month = AT[2], day = AT[3], hour = AT[4], min = AT[5], sec = AT[6] })")
            .eval()
            .expect("os.time converts a local date");
        assert_eq!(back, SAMPLE_MOMENT, "the local date does not convert back to the moment it was read at");
    }
}
