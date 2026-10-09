//! The `main_state` module, its two companion modules, and the binding that points them at a host.
//!
//! `main_state` is the skin's whole view of the running game (`MainStateAccessor`). A skin fetches
//! it once with `require("main_state")`, often caches its functions in locals while loading, and
//! then calls them from hundreds of closures every frame. So the table and every function in it
//! must be the same objects from load to unload, while the game state behind them is a borrowed
//! view that changes every frame. Three parts make that work.
//!
//! **Permanent trampolines.** `install` builds `main_state` once. Each of its functions is an
//! ordinary Lua function that forwards to a field of one hidden table, the host table, which is
//! kept in the interpreter's registry under [`HOST_REGISTRY_KEY`] and is never visible to the skin.
//! The module is published through `require` only; it is not a global unless the skin makes it one,
//! exactly as the reference leaves it for a Lua skin (`SkinLuaAccessor.exportMainStateAccessor`).
//!
//! **Scoped binding.** `bind` fills the host table with functions created on the given
//! [`mlua::Scope`], each borrowing the [`SkinHost`], and empties it again when the scope ends. So
//! nothing outlives the borrow and no `unsafe` is involved. The runtime binds once per frame and
//! once per pass of the entry file, and every Lua call of that frame or pass happens inside the one
//! scope. A function called while nothing is bound finds its field of the host table empty and
//! raises `attempt to call ... a nil value` -- the error the reference raises for a `main_state`
//! call made while it reads a header, when the module is an empty table. Bindings do not nest: a
//! second binding inside the first replaces it, and ends it when it ends.
//!
//! **What needs no host** is built once as ordinary functions: the file helpers, which only need
//! the skin's paths, `set_timer`, which writes to a store this module owns, and the two network
//! functions, which always refuse.
//!
//! The API (`MainStatePropertyLuaApiExporter`, `SkinFileLuaApiExporter`, `SkinHttpLuaApiExporter`,
//! `SkinAudioLuaApiExporter`), with every argument read through the coercions in [`super::coerce`]:
//!
//! - `option(id | name)` is [`DrawStateSource::boolean`], `false` when it answers `None`.
//!   `number(id | name)` is [`SkinHost::integer`], and zero rather than the "no value" sentinel for
//!   an id the reference has no property under; `numbers(...)` applies it to each argument.
//!   `float_number(id | name)` is [`SkinHost::float`] and `text(id | name)` is [`SkinHost::text`].
//!   A number, or a string that spells one, is an id; any other string is a name, looked up in the
//!   tables of [`crate::property::names`]; anything else is no property at all.
//! - `event_index(id)` is [`SkinHost::image_index`] and raises for an id the reference has no image
//!   index under, where the reference fails on a null property.
//! - `offset(id)` is [`OffsetSource::offset`] as a fresh `{x, y, w, h, r, a}` table, zeros for
//!   `None`. An id outside `0..=199` raises, as indexing past the reference's offset array does.
//! - `timer(id)` is [`SkinHost::timer_us`] and `time()` is [`SkinHost::now_us`], both handed over
//!   as doubles; `timer_off_value` is the constant -2^63. `timer_is_on`, `timer_is_off`,
//!   `timer_elapsed`, `timer_elapsed_ms` and `timer_elapsed_seconds` derive from those two, and an
//!   off timer's elapsed time is -1.
//! - `set_timer(id, micros)` accepts a custom timer id (10000..=19999) and raises for any other.
//!   The value is kept by this module and is what `timer(id)` answers for that id afterwards; the
//!   host is not told, and [`custom_timer_us`] is how the rest of the crate reads it.
//!   `event_exec(id, arg1, arg2)` is [`SkinHost::exec_event`] with missing arguments as zero.
//! - `key_pressed(code | name)` is [`SkinHost::key_pressed`], `false` for a negative code or an
//!   unknown name. A name is a libGDX display name such as `"Left"` (`Input.Keys.valueOf`).
//!   `screen_width()` and `screen_height()` are [`SkinHost::screen_size`].
//! - `rate`, `exscore`, `rate_best`, `exscore_best`, `rate_rival` and `exscore_rival` are
//!   [`SkinHost::score`]; `gauge`, `gauge_type` and `judge(n)` are the host methods of the same
//!   names; `volume_sys`, `volume_key`, `volume_bg` and their `set_` forms are [`SkinHost::volume`]
//!   and [`SkinHost::set_volume`].
//! - `audio_play(path, volume)`, `audio_loop`, `audio_preload`, `audio_stop` and `audio_dispose`
//!   resolve the path through [`SkinPaths::readable`], raise `skin file access denied` for a path
//!   outside the root, clamp the volume to `0.0..=2.0` with a missing one as 1, and pass an
//!   [`AudioCommand`] to [`SkinHost::audio`]. Each answers `true`.
//! - `file_exists`, `file_mkdir`, `file_list`, `file_read_lines`, `file_write`, `file_append`,
//!   `file_clear` and `file_count_lines` follow the overlay rules of [`SkinPaths`]: reads see the
//!   overlay's copy first and writes land in the overlay alone. Only `file_exists` raises, and only
//!   for a path outside the root; the others answer `false`, zero or nothing. `file_read_lines`
//!   answers nothing for a file over 64 MiB, since its lines would be held outside the
//!   interpreter's memory ceiling; `file_count_lines` reads a file of any size a block at a time
//!   and charges the meter for what it reads. `file_list` filters with a Lua pattern, where the
//!   reference rewrites the pattern into a Java regular expression and so reads some patterns
//!   differently.
//! - `http_get` and `http_get_lines` always answer `nil, message`. This build makes no request on a
//!   skin's behalf.
//!
//! **Header-only mode.** Under [`LuaMode::HeaderOnly`] the three modules are empty tables, exactly
//! as the reference leaves them while it lists skins, and `bind` does nothing.
//!
//! **The prelude.** `timer_util` and `event_util` hold state per returned function and need nothing
//! from Rust, so they are written in Lua (`prelude.lua`, compiled in as [`PRELUDE`]). The chunk
//! receives the `main_state` table as its one argument and returns the two tables in that order.
//! `timer_util` offers `now_timer`, `is_timer_on`, `is_timer_off`, `timer_function`,
//! `timer_observe_boolean` and `new_passive_timer` (`TimerUtility`); `event_util` offers
//! `event_observe_turn_true`, `event_observe_timer`, `event_observe_timer_on`,
//! `event_observe_timer_off` and `event_min_interval` (`EventUtility`).
//!
//! [`DrawStateSource::boolean`]: crate::dst::DrawStateSource::boolean
//! [`SkinPaths`]: super::SkinPaths
//! [`SkinPaths::readable`]: super::SkinPaths::readable
//! [`OffsetSource::offset`]: crate::dst::OffsetSource::offset

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs::{File, Metadata, OpenOptions};
use std::io::{Read as _, Write as _};
use std::path::Path;
use std::rc::Rc;

use mlua::chunk::ChunkMode;
use mlua::{FromLuaMulti, IntoLuaMulti, Lua, Scope, Table, Value, Variadic};

use super::{LuaFnKind, LuaMode, LuaShared, coerce, package};
use crate::dst::SkinOffset;
use crate::property::generated::OFFSET_MAX;
use crate::property::{AudioCommand, NameSpace, ScoreSlot, SkinHost, VolumeBus, id_of_name, reference_implements, reference_writes};
use crate::timer::{TIMER_OFF, TimerId, timer_id};

/// The module name of the game state accessor.
pub const MAIN_STATE_MODULE: &str = "main_state";

/// The module name of the timer helpers.
pub const TIMER_UTIL_MODULE: &str = "timer_util";

/// The module name of the event helpers.
pub const EVENT_UTIL_MODULE: &str = "event_util";

/// The registry key of the hidden table the trampolines forward to.
pub const HOST_REGISTRY_KEY: &str = "rbms.skin.host";

/// The Lua source of `timer_util` and `event_util`.
pub const PRELUDE: &str = include_str!("prelude.lua");

/// The name the prelude carries in the interpreter's own messages.
const PRELUDE_CHUNK_NAME: &str = "=[skin prelude]";

/// The name the trampolines carry in the interpreter's own messages.
const TRAMPOLINE_CHUNK_NAME: &str = "=[skin main_state]";

/// What an off timer's elapsed time reads as (`TimerElapsedFunction`).
const ELAPSED_WHEN_OFF: f64 = -1.0;

/// Microseconds in the millisecond `timer_elapsed_ms` counts in.
const MICROS_PER_MILLI: i64 = 1_000;

/// Microseconds in the second `timer_elapsed_seconds` counts in.
const MICROS_PER_SECOND: f64 = 1_000_000.0;

/// What `number` answers for an id the reference has no property under (`getNumberValue`).
const NUMBER_WITHOUT_PROPERTY: i32 = 0;

/// The volume a sound plays at when the skin names none (`SkinAudioLuaApiExporter.play`).
const AUDIO_VOLUME_DEFAULT: f32 = 1.0;

/// The quietest a skin may ask a sound to play.
const AUDIO_VOLUME_MIN: f32 = 0.0;

/// The loudest a skin may ask a sound to play.
const AUDIO_VOLUME_MAX: f32 = 2.0;

/// The fewest arguments `event_exec` takes: the event id.
const EVENT_ARGUMENTS_MIN: usize = 1;

/// The most arguments `event_exec` takes: the id and the event's two arguments. The reference's
/// function does not return at all for any other count.
const EVENT_ARGUMENTS_MAX: usize = 3;

/// The code `Input.Keys.valueOf` answers for a name it does not know.
const UNKNOWN_KEY: i32 = -1;

/// What a path outside the skin root is refused with (`SkinLuaPathResolver.resolve`).
const ACCESS_DENIED: &str = "skin file access denied: ";

/// What `set_timer` refuses a timer outside the custom band with.
const TIMER_NOT_WRITABLE: &str = "this timer cannot be changed by a skin: ";

/// What `event_exec` refuses a call with no id, or with more than two arguments, with.
const EVENT_ARGUMENT_COUNT: &str = "event_exec takes an event id and at most two arguments";

/// What `event_index` refuses an id with no image index behind it with.
const NO_IMAGE_INDEX: &str = "no image index property has the id ";

/// What `offset` refuses an id outside the offset table with.
const OFFSET_OUT_OF_RANGE: &str = "offset id out of range: ";

/// What the two network functions answer in place of a response.
const NETWORK_DISABLED: &str = "network access is not available to skins";

/// Bytes of the longest property name the name cache keeps. The longest name the reference knows is
/// half of this; a longer string is looked up every time it is asked for.
const MAX_CACHED_NAME_BYTES: usize = 64;

/// Names the cache keeps altogether, over every id space. The reference knows a few hundred, and a
/// boolean name is also known with its negation mark.
const MAX_CACHED_NAMES: usize = 4_096;

/// Bytes of a file `file_read_lines` will read. Its lines are held outside the interpreter until
/// they are handed over, so they cannot be left to the interpreter's own memory ceiling.
const MAX_LINES_FILE_BYTES: u64 = 64 * 1024 * 1024;

/// Bytes `file_count_lines` reads at a time.
const COUNT_BLOCK_BYTES: usize = 64 * 1024;

/// Bytes of a file whose reading is charged as one instruction.
const READ_BYTES_PER_INSTRUCTION: usize = 16;

/// The longest tail of a block that can be the start of a character the next block finishes.
const MAX_SPLIT_CHARACTER_BYTES: usize = 3;

const LINE_FEED: u8 = b'\n';
const CARRIAGE_RETURN: u8 = b'\r';

/// What the trampolines are told when the module was installed without its state.
const STATE_MISSING: &str = "main_state is not installed in this interpreter";

/// The key codes of libGDX 1.9.9 and the display name `Input.Keys.toString` gives each, which is
/// what `Input.Keys.valueOf` looks a name up by. Read out of the reference's `gdx.jar`.
const GDX_KEY_NAMES: &[(i32, &str)] = &[
    (0, "Unknown"),
    (1, "Soft Left"),
    (2, "Soft Right"),
    (3, "Home"),
    (4, "Back"),
    (5, "Call"),
    (6, "End Call"),
    (7, "0"),
    (8, "1"),
    (9, "2"),
    (10, "3"),
    (11, "4"),
    (12, "5"),
    (13, "6"),
    (14, "7"),
    (15, "8"),
    (16, "9"),
    (17, "*"),
    (18, "#"),
    (19, "Up"),
    (20, "Down"),
    (21, "Left"),
    (22, "Right"),
    (23, "Center"),
    (24, "Volume Up"),
    (25, "Volume Down"),
    (26, "Power"),
    (27, "Camera"),
    (28, "Clear"),
    (29, "A"),
    (30, "B"),
    (31, "C"),
    (32, "D"),
    (33, "E"),
    (34, "F"),
    (35, "G"),
    (36, "H"),
    (37, "I"),
    (38, "J"),
    (39, "K"),
    (40, "L"),
    (41, "M"),
    (42, "N"),
    (43, "O"),
    (44, "P"),
    (45, "Q"),
    (46, "R"),
    (47, "S"),
    (48, "T"),
    (49, "U"),
    (50, "V"),
    (51, "W"),
    (52, "X"),
    (53, "Y"),
    (54, "Z"),
    (55, ","),
    (56, "."),
    (57, "L-Alt"),
    (58, "R-Alt"),
    (59, "L-Shift"),
    (60, "R-Shift"),
    (61, "Tab"),
    (62, "Space"),
    (63, "SYM"),
    (64, "Explorer"),
    (65, "Envelope"),
    (66, "Enter"),
    (67, "Delete"),
    (68, "`"),
    (69, "-"),
    (70, "="),
    (71, "["),
    (72, "]"),
    (73, "\\"),
    (74, ";"),
    (75, "'"),
    (76, "/"),
    (77, "@"),
    (78, "Num"),
    (79, "Headset Hook"),
    (80, "Focus"),
    (81, "Plus"),
    (82, "Menu"),
    (83, "Notification"),
    (84, "Search"),
    (85, "Play/Pause"),
    (86, "Stop Media"),
    (87, "Next Media"),
    (88, "Prev Media"),
    (89, "Rewind"),
    (90, "Fast Forward"),
    (91, "Mute"),
    (92, "Page Up"),
    (93, "Page Down"),
    (94, "PICTSYMBOLS"),
    (95, "SWITCH_CHARSET"),
    (96, "A Button"),
    (97, "B Button"),
    (98, "C Button"),
    (99, "X Button"),
    (100, "Y Button"),
    (101, "Z Button"),
    (102, "L1 Button"),
    (103, "R1 Button"),
    (104, "L2 Button"),
    (105, "R2 Button"),
    (106, "Left Thumb"),
    (107, "Right Thumb"),
    (108, "Start"),
    (109, "Select"),
    (110, "Button Mode"),
    (112, "Forward Delete"),
    (129, "L-Ctrl"),
    (130, "R-Ctrl"),
    (131, "Escape"),
    (132, "End"),
    (133, "Insert"),
    (144, "Numpad 0"),
    (145, "Numpad 1"),
    (146, "Numpad 2"),
    (147, "Numpad 3"),
    (148, "Numpad 4"),
    (149, "Numpad 5"),
    (150, "Numpad 6"),
    (151, "Numpad 7"),
    (152, "Numpad 8"),
    (153, "Numpad 9"),
    (243, ":"),
    (244, "F1"),
    (245, "F2"),
    (246, "F3"),
    (247, "F4"),
    (248, "F5"),
    (249, "F6"),
    (250, "F7"),
    (251, "F8"),
    (252, "F9"),
    (253, "F10"),
    (254, "F11"),
    (255, "F12"),
];

/// The three score slots with the names of the two functions that read each.
const SCORE_FUNCTIONS: [(&str, &str, ScoreSlot); 3] =
    [("rate", "exscore", ScoreSlot::Current), ("rate_best", "exscore_best", ScoreSlot::Best), ("rate_rival", "exscore_rival", ScoreSlot::Rival)];

/// The three volumes with the names of the function that reads and the function that sets each.
const VOLUME_FUNCTIONS: [(&str, &str, VolumeBus); 3] = [
    ("volume_sys", "set_volume_sys", VolumeBus::System),
    ("volume_key", "set_volume_key", VolumeBus::Key),
    ("volume_bg", "set_volume_bg", VolumeBus::Background),
];

/// The trampolines: `main_state` itself.
///
/// The chunk receives the host table, whose fields [`bind`] fills for the length of a binding, and
/// the table of functions that need no host. A function that can fail gets the message back from
/// its Rust half and raises it here, so that the error is a string carrying the position of the
/// skin's own call.
const TRAMPOLINES: &str = r##"
local host, core = ...
local error, pcall, tostring, find, sub, concat = error, pcall, tostring, string.find, string.sub, table.concat
local OFF = core.timer_off_value

local main_state = { timer_off_value = OFF }

function main_state.option(id) return host.option(id) end
function main_state.number(id) return host.number(id) end
function main_state.numbers(...) return host.numbers(...) end
function main_state.float_number(id) return host.float_number(id) end
function main_state.text(id) return host.text(id) end

function main_state.offset(id)
    local offset, message = host.offset(id)
    if offset == nil then error(message, 2) end
    return offset
end

function main_state.timer(id) return host.timer(id) end
function main_state.timer_is_on(id) return host.timer(id) ~= OFF end
function main_state.timer_is_off(id) return host.timer(id) == OFF end
function main_state.timer_elapsed(id) return host.timer_elapsed(id) end
function main_state.timer_elapsed_ms(id) return host.timer_elapsed_ms(id) end
function main_state.timer_elapsed_seconds(id) return host.timer_elapsed_seconds(id) end
function main_state.time() return host.time() end

function main_state.set_timer(id, value)
    local message = core.set_timer(id, value)
    if message then error(message, 2) end
    return true
end

function main_state.event_exec(...)
    local message = host.event_exec(...)
    if message then error(message, 2) end
    return true
end

function main_state.event_index(id)
    local index, message = host.event_index(id)
    if index == nil then error(message, 2) end
    return index
end

function main_state.key_pressed(key) return host.key_pressed(key) end
function main_state.screen_width() return host.screen_width() end
function main_state.screen_height() return host.screen_height() end

function main_state.rate() return host.rate() end
function main_state.exscore() return host.exscore() end
function main_state.rate_best() return host.rate_best() end
function main_state.exscore_best() return host.exscore_best() end
function main_state.rate_rival() return host.rate_rival() end
function main_state.exscore_rival() return host.exscore_rival() end

function main_state.volume_sys() return host.volume_sys() end
function main_state.volume_key() return host.volume_key() end
function main_state.volume_bg() return host.volume_bg() end

function main_state.set_volume_sys(volume)
    host.set_volume_sys(volume)
    return true
end

function main_state.set_volume_key(volume)
    host.set_volume_key(volume)
    return true
end

function main_state.set_volume_bg(volume)
    host.set_volume_bg(volume)
    return true
end

function main_state.gauge() return host.gauge() end
function main_state.gauge_type() return host.gauge_type() end
function main_state.judge(judge) return host.judge(judge) end

function main_state.audio_play(path, volume)
    local message = host.audio_play(path, volume)
    if message then error(message, 2) end
    return true
end

function main_state.audio_loop(path, volume)
    local message = host.audio_loop(path, volume)
    if message then error(message, 2) end
    return true
end

function main_state.audio_preload(path)
    local message = host.audio_preload(path)
    if message then error(message, 2) end
    return true
end

function main_state.audio_stop(path)
    local message = host.audio_stop(path)
    if message then error(message, 2) end
    return true
end

function main_state.audio_dispose(path)
    local message = host.audio_dispose(path)
    if message then error(message, 2) end
    return true
end

function main_state.file_exists(path)
    local exists, message = core.file_exists(path)
    if exists == nil then error(message, 2) end
    return exists
end

main_state.file_mkdir = core.file_mkdir
main_state.file_read_lines = core.file_read_lines
main_state.file_write = core.file_write
main_state.file_append = core.file_append
main_state.file_clear = core.file_clear
main_state.file_count_lines = core.file_count_lines

local function matching(entries, pattern)
    local lines, count = {}, 0
    for index = 1, #entries do
        local entry = entries[index]
        if pattern then
            local first, last = find(entry, pattern)
            entry = first and sub(entry, first, last)
        end
        if entry then
            count = count + 1
            lines[count] = entry .. "\n"
        end
    end
    return concat(lines), count
end

function main_state.file_list(directory, pattern)
    local entries = core.file_entries(directory)
    if not entries then
        return "", 0
    end
    if pattern ~= nil then
        pattern = tostring(pattern)
        if pattern == "" then
            pattern = nil
        end
    end
    local listed, text, count = pcall(matching, entries, pattern)
    if not listed then
        return "", 0
    end
    return text, count
end

function main_state.http_get_lines() return nil, core.network_disabled end
function main_state.http_get() return nil, core.network_disabled end

return main_state
"##;

/// What the module keeps for as long as the interpreter lives.
#[derive(Debug)]
struct ModuleState {
    /// The skin's paths, for the file helpers and the audio paths, and the meter the file helpers
    /// charge.
    shared: Rc<LuaShared>,
    /// The custom timers a skin switched with `set_timer`, by id.
    custom_timers: RefCell<BTreeMap<i32, i64>>,
    /// The property names a script has asked for and the id each stands for. The reference keeps
    /// the same cache, because a script names its properties on every frame.
    ///
    /// Only a name that stands for a property is kept, only up to [`MAX_CACHED_NAME_BYTES`] long,
    /// and only [`MAX_CACHED_NAMES`] of them: the cache lives outside the interpreter's memory
    /// ceiling, and a skin can ask for any string it can build.
    names: RefCell<BTreeMap<(NameSpace, String), i32>>,
}

impl ModuleState {
    fn new(shared: &Rc<LuaShared>) -> Self {
        Self { shared: Rc::clone(shared), custom_timers: RefCell::new(BTreeMap::new()), names: RefCell::new(BTreeMap::new()) }
    }

    /// The id `name` stands for in `space`, remembered after the first lookup that found one.
    fn named(&self, space: NameSpace, name: &str) -> Option<i32> {
        if name.len() > MAX_CACHED_NAME_BYTES {
            return id_of_name(space, name);
        }
        let key = (space, name.to_owned());
        if let Some(known) = self.names.borrow().get(&key) {
            return Some(*known);
        }
        let id = id_of_name(space, name)?;
        let mut names = self.names.borrow_mut();
        if names.len() < MAX_CACHED_NAMES {
            names.insert(key, id);
        }
        Some(id)
    }

    /// The property a script addressed: a number, or a string that spells one, is an id; any other
    /// string is a name; anything else is no property (`MainStatePropertyLuaApiExporter.getProperty`).
    fn property_id(&self, space: NameSpace, value: &Value) -> Option<i32> {
        if coerce::to_number(value).is_some() {
            return Some(coerce::to_int(value));
        }
        match value {
            Value::String(name) => self.named(space, &name.to_string_lossy()),
            _ => None,
        }
    }
}

/// The module state of this interpreter, when it was installed in full.
fn module_state(lua: &Lua) -> Option<Rc<ModuleState>> {
    lua.app_data_ref::<Rc<ModuleState>>().map(|state| Rc::clone(&state))
}

/// The id a property name stands for in one id space, or `None` when the reference knows no such
/// name. A boolean name's leading `!` comes back as a negative id. The answer is remembered in the
/// interpreter, so asking on every frame costs one map lookup.
pub fn named_id(lua: &Lua, space: NameSpace, name: &str) -> Option<i32> {
    match module_state(lua) {
        Some(state) => state.named(space, name),
        None => id_of_name(space, name),
    }
}

/// Whether the reference knows `name` as a property a field read as `kind` may name, which is what
/// decides between looking a string up and compiling it as a script.
///
/// A float field names a rate, never a float; a writer field names a property that has a writer;
/// a timer field has no names at all. A boolean name is known with or without its `!`.
pub fn is_property_name(kind: LuaFnKind, name: &str) -> bool {
    let known = |space: NameSpace| id_of_name(space, name);
    match kind {
        LuaFnKind::Boolean => known(NameSpace::Boolean).is_some(),
        LuaFnKind::Integer => known(NameSpace::Integer).is_some(),
        LuaFnKind::Float => known(NameSpace::Rate).is_some(),
        LuaFnKind::Text => known(NameSpace::Text).is_some(),
        LuaFnKind::Event => known(NameSpace::Event).is_some(),
        LuaFnKind::FloatWriter => known(NameSpace::Rate).is_some_and(|id| reference_writes(NameSpace::Rate, id)),
        LuaFnKind::TextWriter => known(NameSpace::Text).is_some_and(|id| reference_writes(NameSpace::Text, id)),
        LuaFnKind::Timer => false,
    }
}

/// The microsecond a skin switched the custom timer under `id` on with `main_state.set_timer`, or
/// `None` when it never wrote that id. A timer the skin switched off answers [`TIMER_OFF`].
///
/// These values live in the interpreter and not in the host, so whoever resolves a destination's
/// timer id asks here before it asks the host.
pub fn custom_timer_us(lua: &Lua, id: i32) -> Option<i64> {
    module_state(lua).and_then(|state| state.custom_timers.borrow().get(&id).copied())
}

/// The libGDX key code a display name stands for, or [`UNKNOWN_KEY`].
pub(crate) fn key_code(name: &str) -> i32 {
    GDX_KEY_NAMES.iter().find(|(_, known)| *known == name).map_or(UNKNOWN_KEY, |(code, _)| *code)
}

/// The lines of a text as `Files.readAllLines` splits them: at `\n`, `\r` and `\r\n`, with no empty
/// line after a final terminator.
fn java_lines(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        let end = rest.find(['\n', '\r']).unwrap_or(rest.len());
        lines.push(&rest[..end]);
        rest = &rest[end..];
        rest = rest.strip_prefix("\r\n").or_else(|| rest.strip_prefix('\n')).or_else(|| rest.strip_prefix('\r')).unwrap_or(rest);
    }
    lines
}

/// The lines of a file the skin may read, or `None` when it cannot be read as UTF-8 text or is
/// larger than [`MAX_LINES_FILE_BYTES`].
fn read_lines(shared: &LuaShared, path: &Value) -> Option<Vec<String>> {
    let file = shared.paths.readable(&coerce::to_jstring(path)).ok()?;
    let size = std::fs::metadata(&file).ok()?.len();
    if size > MAX_LINES_FILE_BYTES || !shared.meter.charge(size / READ_BYTES_PER_INSTRUCTION as u64) {
        return None;
    }
    let text = String::from_utf8(std::fs::read(file).ok()?).ok()?;
    Some(java_lines(&text).into_iter().map(str::to_owned).collect())
}

/// Counts the lines of a text as [`java_lines`] splits them, a block at a time.
#[derive(Default)]
struct LineCounter {
    lines: usize,
    /// Whether anything stands after the last line end.
    in_line: bool,
    /// Whether the last byte was a carriage return, which a line feed right after it belongs to.
    after_carriage_return: bool,
}

impl LineCounter {
    fn feed(&mut self, block: &[u8]) {
        for byte in block {
            match *byte {
                LINE_FEED if self.after_carriage_return => {}
                LINE_FEED | CARRIAGE_RETURN => self.lines += 1,
                _ => {}
            }
            self.in_line = !matches!(*byte, LINE_FEED | CARRIAGE_RETURN);
            self.after_carriage_return = *byte == CARRIAGE_RETURN;
        }
    }

    /// The count, with a last line that no line end closed.
    fn finish(self) -> usize {
        self.lines + usize::from(self.in_line)
    }
}

/// How many lines a file the skin may read has, or `None` when it cannot be read as UTF-8 text or
/// the meter stops the reading. The file is never held whole.
fn count_lines(shared: &LuaShared, path: &Value) -> Option<usize> {
    let mut file = File::open(shared.paths.readable(&coerce::to_jstring(path)).ok()?).ok()?;
    let mut counter = LineCounter::default();
    let mut block = vec![0; COUNT_BLOCK_BYTES + MAX_SPLIT_CHARACTER_BYTES];
    let mut carried = 0;
    loop {
        let read = file.read(&mut block[carried..]).ok()?;
        if read == 0 {
            return (carried == 0).then(|| counter.finish());
        }
        if !shared.meter.charge((read / READ_BYTES_PER_INSTRUCTION) as u64) {
            return None;
        }
        let filled = carried + read;
        let whole = match std::str::from_utf8(&block[..filled]) {
            Ok(_) => filled,
            Err(error) if error.error_len().is_none() => error.valid_up_to(),
            Err(_) => return None,
        };
        counter.feed(&block[..whole]);
        block.copy_within(whole..filled, 0);
        carried = filled - whole;
    }
}

/// Writes `text` to a file in the overlay, creating the directories above it. An append to a file
/// only the skin root holds starts from a copy of it, so the skin reads back what the reference
/// would have left in the skin folder. The copy is the overlay's own file, writable whatever the
/// original's permissions.
///
/// The directories, the file and every byte it gains are admitted together before the first
/// directory is made, so a write that is refused leaves the overlay exactly as it was.
fn write_file(shared: &LuaShared, path: &Value, text: &str, append: bool) -> std::io::Result<()> {
    let paths = &shared.paths;
    let quota = &shared.quota;
    let name = coerce::to_jstring(path);
    let target = paths.writable(&name).map_err(std::io::Error::other)?;
    let mut present = std::fs::metadata(&target).ok().filter(Metadata::is_file).map(|metadata| metadata.len());
    let original =
        if append && !target.exists() { Some(paths.readable(&name).map_err(std::io::Error::other)?).filter(|original| original.is_file()) } else { None };
    let copied = original.as_deref().map(std::fs::metadata).transpose()?.map_or(0, |metadata| metadata.len());
    let held = present.unwrap_or(0);
    let start = present.unwrap_or(copied);
    let after = if append { start.saturating_add(text.len() as u64) } else { text.len() as u64 };
    let directories = target.parent().map_or(0, |parent| quota.missing_directories(parent));
    quota.admit(directories + u64::from(present.is_none()), held, after)?;
    if let Some(parent) = target.parent() {
        quota.create_directories(parent)?;
    }
    if let Some(original) = &original {
        present = Some(quota.copy_original(original, &target)?);
    }
    let before = present.unwrap_or(0);
    quota.admit_change(before, after)?;
    let created = present.is_none();
    if created {
        quota.admit_entries(1)?;
    }
    let mut file = OpenOptions::new().create(true).write(true).append(append).truncate(!append).open(target)?;
    if created {
        quota.record_entries(1);
    }
    if let Err(error) = file.write_all(text.as_bytes()) {
        quota.invalidate();
        return Err(error);
    }
    quota.record_change(before, after);
    Ok(())
}

/// The functions that need no host, as the table the trampolines receive.
fn core_table(lua: &Lua, state: &Rc<ModuleState>) -> mlua::Result<Table> {
    let core = lua.create_table()?;
    core.set("timer_off_value", TIMER_OFF as f64)?;
    core.set("network_disabled", NETWORK_DISABLED)?;

    let timers = Rc::clone(state);
    let set_timer = lua.create_function(move |_, (id, value): (Value, Value)| {
        let id = coerce::to_int(&id);
        if !TimerId(id).is_custom() {
            return Ok(Some(format!("{TIMER_NOT_WRITABLE}{id} ({}..={})", timer_id::CUSTOM_BEGIN.get(), timer_id::CUSTOM_END.get())));
        }
        timers.custom_timers.borrow_mut().insert(id, coerce::to_long(&value));
        Ok(None)
    })?;
    core.set("set_timer", set_timer)?;

    let files = Rc::clone(state);
    let file_exists = lua.create_function(move |_, path: Value| {
        let name = coerce::to_jstring(&path);
        Ok(match files.shared.paths.readable(&name) {
            Ok(file) => (Some(file.exists()), None),
            Err(_) => (None, Some(format!("{ACCESS_DENIED}{name}"))),
        })
    })?;
    core.set("file_exists", file_exists)?;

    let files = Rc::clone(state);
    let file_mkdir = lua.create_function(move |_, path: Value| {
        Ok(files.shared.paths.writable(&coerce::to_jstring(&path)).is_ok_and(|directory| files.shared.quota.create_directories(&directory).is_ok()))
    })?;
    core.set("file_mkdir", file_mkdir)?;

    let files = Rc::clone(state);
    let file_entries = lua.create_function(move |lua, directory: Value| match files.shared.paths.entries(&coerce::to_jstring(&directory)) {
        Ok(entries) => lua.create_sequence_from(entries).map(Some),
        Err(_) => Ok(None),
    })?;
    core.set("file_entries", file_entries)?;

    let files = Rc::clone(state);
    let file_read_lines = lua.create_function(move |lua, path: Value| lua.create_sequence_from(read_lines(&files.shared, &path).unwrap_or_default()))?;
    core.set("file_read_lines", file_read_lines)?;

    let files = Rc::clone(state);
    let file_count_lines = lua.create_function(move |_, path: Value| Ok(count_lines(&files.shared, &path).unwrap_or(0)))?;
    core.set("file_count_lines", file_count_lines)?;

    let files = Rc::clone(state);
    let file_write =
        lua.create_function(move |_, (path, text): (Value, Value)| Ok(write_file(&files.shared, &path, &coerce::to_jstring(&text), false).is_ok()))?;
    core.set("file_write", file_write)?;

    let files = Rc::clone(state);
    let file_append =
        lua.create_function(move |_, (path, text): (Value, Value)| Ok(write_file(&files.shared, &path, &coerce::to_jstring(&text), true).is_ok()))?;
    core.set("file_append", file_append)?;

    let files = Rc::clone(state);
    let file_clear = lua.create_function(move |_, path: Value| Ok(write_file(&files.shared, &path, "", false).is_ok()))?;
    core.set("file_clear", file_clear)?;

    Ok(core)
}

/// Builds `main_state`, `timer_util` and `event_util` and publishes them through
/// [`package::preload`].
pub(crate) fn install(lua: &Lua, shared: &Rc<LuaShared>) -> mlua::Result<()> {
    let host = lua.create_table()?;
    lua.set_named_registry_value(HOST_REGISTRY_KEY, &host)?;
    let (main_state, timer_util, event_util) = match shared.mode {
        LuaMode::HeaderOnly => (lua.create_table()?, lua.create_table()?, lua.create_table()?),
        LuaMode::Full => {
            let state = Rc::new(ModuleState::new(shared));
            let core = core_table(lua, &state)?;
            lua.set_app_data(state);
            let main_state: Table = lua.load(TRAMPOLINES).set_name(TRAMPOLINE_CHUNK_NAME).set_mode(ChunkMode::Text).call((host, core))?;
            let (timer_util, event_util): (Table, Table) = lua.load(PRELUDE).set_name(PRELUDE_CHUNK_NAME).set_mode(ChunkMode::Text).call(&main_state)?;
            (main_state, timer_util, event_util)
        }
    };
    package::preload(lua, MAIN_STATE_MODULE, Value::Table(main_state))?;
    package::preload(lua, TIMER_UTIL_MODULE, Value::Table(timer_util))?;
    package::preload(lua, EVENT_UTIL_MODULE, Value::Table(event_util))
}

/// One binding: the host a frame or a pass reads, with the state the module keeps between them.
struct Binding<'env> {
    host: &'env dyn SkinHost,
    state: Rc<ModuleState>,
}

impl Binding<'_> {
    /// `option`: `false` for an option nothing implements, whatever its sign.
    fn option(&self, value: &Value) -> bool {
        self.state.property_id(NameSpace::Boolean, value).and_then(|id| self.host.boolean(id)).unwrap_or(false)
    }

    /// `number`: the host's value, sentinel included, for a property the reference has, and zero for
    /// one it does not.
    fn number(&self, value: &Value) -> i32 {
        match self.state.property_id(NameSpace::Integer, value) {
            Some(id) if reference_implements(NameSpace::Integer, id) => self.host.integer(id),
            _ => NUMBER_WITHOUT_PROPERTY,
        }
    }

    /// `float_number`.
    fn float_number(&self, value: &Value) -> f32 {
        self.state.property_id(NameSpace::Float, value).map_or(crate::property::FLOAT_ABSENT, |id| self.host.float(id))
    }

    /// `text`, as a string of the interpreter.
    fn text(&self, lua: &Lua, value: &Value) -> mlua::Result<mlua::LuaString> {
        match self.state.property_id(NameSpace::Text, value) {
            Some(id) => lua.create_string(self.host.text(id).as_bytes()),
            None => lua.create_string(crate::property::TEXT_ABSENT),
        }
    }

    /// `offset`: a fresh table, or nothing and the message to raise.
    fn offset(&self, lua: &Lua, value: &Value) -> mlua::Result<(Option<Table>, Option<String>)> {
        let id = coerce::to_int(value);
        if !(0..=OFFSET_MAX).contains(&id) {
            return Ok((None, Some(format!("{OFFSET_OUT_OF_RANGE}{id}"))));
        }
        let SkinOffset { x, y, w, h, r, a } = self.host.offset(id).unwrap_or_default();
        lua.create_table_from([("x", x), ("y", y), ("w", w), ("h", h), ("r", r), ("a", a)]).map(|offset| (Some(offset), None))
    }

    /// The microsecond the timer a script named switched on: a custom timer the skin wrote, or the
    /// host's.
    fn timer(&self, value: &Value) -> i64 {
        let id = coerce::to_int(value);
        self.state.custom_timers.borrow().get(&id).copied().unwrap_or_else(|| self.host.timer_us(id))
    }

    /// `timer_elapsed` and `timer_elapsed_ms`: whole units since the timer switched on.
    fn elapsed(&self, value: &Value, micros_per_unit: i64) -> f64 {
        match self.timer(value) {
            TIMER_OFF => ELAPSED_WHEN_OFF,
            since => (self.host.now_us().wrapping_sub(since) / micros_per_unit) as f64,
        }
    }

    /// `timer_elapsed_seconds`.
    fn elapsed_seconds(&self, value: &Value) -> f64 {
        match self.timer(value) {
            TIMER_OFF => ELAPSED_WHEN_OFF,
            since => self.host.now_us().wrapping_sub(since) as f64 / MICROS_PER_SECOND,
        }
    }

    /// `event_exec`: nothing, or the message to raise.
    fn event_exec(&self, arguments: &[Value]) -> Option<String> {
        if !(EVENT_ARGUMENTS_MIN..=EVENT_ARGUMENTS_MAX).contains(&arguments.len()) {
            return Some(EVENT_ARGUMENT_COUNT.to_owned());
        }
        let [id, arg1, arg2] = [0, 1, 2].map(|index| arguments.get(index).map_or(0, coerce::to_int));
        self.host.exec_event(id, arg1, arg2);
        None
    }

    /// `event_index`: the index, or nothing and the message to raise.
    fn event_index(&self, value: &Value) -> (Option<i32>, Option<String>) {
        let id = coerce::to_int(value);
        if reference_implements(NameSpace::ImageIndex, id) { (Some(self.host.image_index(id)), None) } else { (None, Some(format!("{NO_IMAGE_INDEX}{id}"))) }
    }

    /// `key_pressed`.
    fn key_pressed(&self, value: &Value) -> bool {
        let code = if coerce::to_number(value).is_some() { coerce::to_int(value) } else { key_code(&coerce::to_jstring(value)) };
        code >= 0 && self.host.key_pressed(code)
    }

    /// The `audio_*` functions: resolves the path and hands the host the command `build` makes of
    /// it. Nothing, or the message to raise.
    fn audio(&self, path: &Value, build: impl for<'path> FnOnce(&'path Path) -> AudioCommand<'path>) -> Option<String> {
        let name = coerce::to_jstring(path);
        match self.state.shared.paths.readable(&name) {
            Ok(file) => {
                self.host.audio(build(&file));
                None
            }
            Err(_) => Some(format!("{ACCESS_DENIED}{name}")),
        }
    }

    /// `audio_play` and `audio_loop`.
    fn play(&self, path: &Value, volume: &Value, looped: bool) -> Option<String> {
        let volume = if volume.is_nil() { AUDIO_VOLUME_DEFAULT } else { coerce::to_float(volume) }.clamp(AUDIO_VOLUME_MIN, AUDIO_VOLUME_MAX);
        self.audio(path, |path| AudioCommand::Play { path, volume, looped })
    }
}

/// Publishes one function of a binding under `name` in the host table.
fn expose<'scope, 'env, A, R>(
    scope: &'scope Scope<'scope, 'env>,
    table: &Table,
    name: &str,
    binding: &Rc<Binding<'env>>,
    call: impl Fn(&Binding<'env>, &Lua, A) -> mlua::Result<R> + 'scope,
) -> mlua::Result<()>
where
    A: FromLuaMulti,
    R: IntoLuaMulti,
{
    let binding = Rc::clone(binding);
    table.raw_set(name, scope.create_function(move |lua, arguments: A| call(&binding, lua, arguments))?)
}

/// Points the trampolines at `host` for as long as `scope` lasts.
pub(crate) fn bind<'scope, 'env>(lua: &Lua, shared: &LuaShared, scope: &'scope Scope<'scope, 'env>, host: &'env dyn SkinHost) -> mlua::Result<()> {
    if shared.mode == LuaMode::HeaderOnly {
        return Ok(());
    }
    let table: Table = lua.named_registry_value(HOST_REGISTRY_KEY)?;
    let state = module_state(lua).ok_or_else(|| mlua::Error::runtime(STATE_MISSING))?;
    let binding = Rc::new(Binding { host, state });
    let table = &table;
    let binding = &binding;

    expose(scope, table, "option", binding, |bound, _, value: Value| Ok(bound.option(&value)))?;
    expose(scope, table, "number", binding, |bound, _, value: Value| Ok(bound.number(&value)))?;
    expose(scope, table, "numbers", binding, |bound, _, values: Variadic<Value>| {
        Ok(values.iter().map(|value| bound.number(value)).collect::<Variadic<i32>>())
    })?;
    expose(scope, table, "float_number", binding, |bound, _, value: Value| Ok(bound.float_number(&value)))?;
    expose(scope, table, "text", binding, |bound, lua, value: Value| bound.text(lua, &value))?;
    expose(scope, table, "offset", binding, |bound, lua, value: Value| bound.offset(lua, &value))?;
    expose(scope, table, "timer", binding, |bound, _, value: Value| Ok(bound.timer(&value) as f64))?;
    expose(scope, table, "timer_elapsed", binding, |bound, _, value: Value| Ok(bound.elapsed(&value, 1)))?;
    expose(scope, table, "timer_elapsed_ms", binding, |bound, _, value: Value| Ok(bound.elapsed(&value, MICROS_PER_MILLI)))?;
    expose(scope, table, "timer_elapsed_seconds", binding, |bound, _, value: Value| Ok(bound.elapsed_seconds(&value)))?;
    expose(scope, table, "time", binding, |bound, _, (): ()| Ok(bound.host.now_us() as f64))?;
    expose(scope, table, "event_exec", binding, |bound, _, arguments: Variadic<Value>| Ok(bound.event_exec(&arguments)))?;
    expose(scope, table, "event_index", binding, |bound, _, value: Value| Ok(bound.event_index(&value)))?;
    expose(scope, table, "key_pressed", binding, |bound, _, value: Value| Ok(bound.key_pressed(&value)))?;
    expose(scope, table, "screen_width", binding, |bound, _, (): ()| Ok(bound.host.screen_size().0))?;
    expose(scope, table, "screen_height", binding, |bound, _, (): ()| Ok(bound.host.screen_size().1))?;
    for (rate, exscore, slot) in SCORE_FUNCTIONS {
        expose(scope, table, rate, binding, move |bound, _, (): ()| Ok(bound.host.score(slot).rate))?;
        expose(scope, table, exscore, binding, move |bound, _, (): ()| Ok(bound.host.score(slot).exscore))?;
    }
    for (read, write, bus) in VOLUME_FUNCTIONS {
        expose(scope, table, read, binding, move |bound, _, (): ()| Ok(bound.host.volume(bus)))?;
        expose(scope, table, write, binding, move |bound, _, value: Value| {
            bound.host.set_volume(bus, coerce::to_float(&value));
            Ok(())
        })?;
    }
    expose(scope, table, "gauge", binding, |bound, _, (): ()| Ok(bound.host.gauge()))?;
    expose(scope, table, "gauge_type", binding, |bound, _, (): ()| Ok(bound.host.gauge_type()))?;
    expose(scope, table, "judge", binding, |bound, _, value: Value| Ok(bound.host.judge(coerce::to_int(&value))))?;
    expose(scope, table, "audio_play", binding, |bound, _, (path, volume): (Value, Value)| Ok(bound.play(&path, &volume, false)))?;
    expose(scope, table, "audio_loop", binding, |bound, _, (path, volume): (Value, Value)| Ok(bound.play(&path, &volume, true)))?;
    expose(scope, table, "audio_preload", binding, |bound, _, path: Value| Ok(bound.audio(&path, |path| AudioCommand::Preload { path })))?;
    expose(scope, table, "audio_stop", binding, |bound, _, path: Value| Ok(bound.audio(&path, |path| AudioCommand::Stop { path })))?;
    expose(scope, table, "audio_dispose", binding, |bound, _, path: Value| Ok(bound.audio(&path, |path| AudioCommand::Dispose { path })))?;

    let unbound = table.clone();
    scope.add_destructor(move || {
        let _ = unbound.clear();
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::rc::Rc;

    use super::{LineCounter, LuaFnKind, MAX_CACHED_NAME_BYTES, MAX_CACHED_NAMES, ModuleState, UNKNOWN_KEY, is_property_name, java_lines, key_code};
    use crate::lua::{LuaBudget, LuaLog, LuaMode, LuaShared, Meter, OverlayLimits, SkinPaths, io};
    use crate::property::{NEGATION_MARK, NameSpace, id_of_name};

    /// The state of a module with nothing bound, over this crate's own folder.
    fn module_state() -> ModuleState {
        let paths = SkinPaths::new(Path::new(env!("CARGO_MANIFEST_DIR")), None).expect("the crate folder exists");
        let shared = LuaShared {
            paths,
            log: LuaLog::default(),
            meter: Meter::new(LuaBudget::default()),
            mode: LuaMode::Full,
            seed: None,
            open_files: io::OpenFiles::default(),
            quota: Rc::new(io::OverlayQuota::measure(None, OverlayLimits::default())),
        };
        ModuleState::new(&Rc::new(shared))
    }

    #[test]
    fn the_name_cache_keeps_only_names_that_stand_for_a_property_and_only_short_ones() {
        let state = module_state();
        for index in 0..MAX_CACHED_NAMES * 2 {
            assert_eq!(state.named(NameSpace::Integer, &format!("no_such_property_{index}")), None);
        }
        assert!(state.names.borrow().is_empty(), "a name that stands for nothing is not remembered");

        let known = id_of_name(NameSpace::Boolean, "autoplay_on").expect("the reference knows the name");
        let negations = MAX_CACHED_NAME_BYTES * 2;
        for count in 0..negations {
            let name = format!("{}autoplay_on", NEGATION_MARK.to_string().repeat(count));
            assert_eq!(state.named(NameSpace::Boolean, &name), Some(if count % 2 == 1 { -known } else { known }), "{name}");
            assert_eq!(state.named(NameSpace::Boolean, &name), state.named(NameSpace::Boolean, &name), "{name} answers the same from the cache");
        }
        let names = state.names.borrow();
        assert!(!names.is_empty() && names.len() < negations, "a name longer than the cache keeps is looked up each time");
        assert!(names.keys().all(|(_, name)| name.len() <= MAX_CACHED_NAME_BYTES));
        assert!(names.len() <= MAX_CACHED_NAMES);
    }

    #[test]
    fn lines_are_counted_a_block_at_a_time_as_they_are_split() {
        for text in ["", "a", "a\n", "a\n\nb", "\n", "a\rb", "a\r\nb", "\r\r\n", "a\nb\r\nc\rd", "\r", "x\r\n\r\ny\n"] {
            let whole = {
                let mut counter = LineCounter::default();
                counter.feed(text.as_bytes());
                counter.finish()
            };
            let by_byte = {
                let mut counter = LineCounter::default();
                text.as_bytes().chunks(1).for_each(|block| counter.feed(block));
                counter.finish()
            };
            assert_eq!(whole, java_lines(text).len(), "{text:?}");
            assert_eq!(by_byte, whole, "{text:?} split between any two bytes");
        }
    }

    #[test]
    fn lines_end_at_any_terminator_and_a_final_one_adds_no_line() {
        assert_eq!(java_lines("a\nb\r\nc\rd"), ["a", "b", "c", "d"]);
        assert_eq!(java_lines("a\n"), ["a"]);
        assert_eq!(java_lines("a\n\nb"), ["a", "", "b"]);
        assert_eq!(java_lines("\n"), [""]);
        assert!(java_lines("").is_empty());
    }

    #[test]
    fn a_string_is_a_name_only_where_the_field_can_take_it() {
        assert!(is_property_name(LuaFnKind::Boolean, "autoplay_on"));
        assert!(is_property_name(LuaFnKind::Boolean, "!autoplay_on"));
        assert!(!is_property_name(LuaFnKind::Integer, "autoplay_on"));
        assert!(is_property_name(LuaFnKind::Integer, "playlevel"));
        assert!(is_property_name(LuaFnKind::Float, "musicselect_position"));
        assert!(!is_property_name(LuaFnKind::Float, "hispeed"), "a float field names a rate, never a float");
        assert!(is_property_name(LuaFnKind::Text, "key12"));
        assert!(is_property_name(LuaFnKind::Event, "keyassign40"));
        assert!(is_property_name(LuaFnKind::FloatWriter, "musicselect_position"));
        assert!(!is_property_name(LuaFnKind::FloatWriter, "scorerate"), "a rate nothing writes to is no writer");
        assert!(is_property_name(LuaFnKind::TextWriter, "searchword"));
        assert!(!is_property_name(LuaFnKind::TextWriter, "title"));
        assert!(!is_property_name(LuaFnKind::Timer, "play"), "a timer has no names");
        assert!(!is_property_name(LuaFnKind::Boolean, "main_state.option(33)"));
    }

    #[test]
    fn a_key_is_named_the_way_the_reference_displays_it() {
        assert_eq!(key_code("Left"), 21);
        assert_eq!(key_code("F12"), 255);
        assert_eq!(key_code("LEFT"), UNKNOWN_KEY, "the constant's name is not the display name");
        assert_eq!(key_code("nil"), UNKNOWN_KEY);
    }
}
