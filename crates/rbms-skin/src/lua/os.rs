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
//! Known differences from the reference, all in `os.date`: `%c` pads the day with a blank where the
//! reference pads with a zero, `%I` counts 01 to 12 where the reference counts 00 to 11, and the
//! conversions the reference does not know are formatted rather than refused.

use std::rc::Rc;
use std::sync::OnceLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use mlua::{Function, Lua, MultiValue, StdLib, Table, Value};

use super::{LuaShared, package};

/// The global, and the module name, the library is published under.
const OS_GLOBAL: &str = "os";

/// The functions taken from the C library as they are.
const NATIVE_FUNCTIONS: [&str; 2] = ["date", "difftime"];

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
