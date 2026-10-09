//! What a skin's Lua may spend, and the meter that holds it to that.
//!
//! A skin runs in two very different regimes. Loading executes tens of thousands of lines once and
//! may take a noticeable fraction of a second; a frame calls several hundred small functions and
//! has a few milliseconds for all of them. One ceiling cannot serve both, so the budget is split:
//! [`LoadBudget`] bounds one pass of the entry file, [`FrameBudget`] bounds one frame, and the
//! memory ceiling in [`LuaBudget`] holds throughout.
//!
//! [`Meter`] is the one piece of state behind all three. The runtime tells it which regime is
//! starting; the interpreter hook and the frame's call wrapper ask it whether to go on.

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use mlua::{HookTriggers, Lua, VmState};

/// Instructions one pass of the entry file may execute.
///
/// Measured with `rbms-cli skin-dump --probe-budget` on the ten documents of a full skin pack: a
/// pass needs 8 thousand to 36 thousand instructions, and the 1,914-object song select, the heaviest,
/// needs 0.32 million. This is about three hundred times that. The ceiling is also how long a runaway
/// loop freezes a scene change before it is cut. At five hundred million, a loop that only counted
/// was cut after 0.5 seconds and one that allocated after 3 to 3.5; at a hundred million the same
/// loops are cut in a fifth of that.
const DEFAULT_LOAD_INSTRUCTIONS: u64 = 100_000_000;

/// Microseconds one pass of the entry file may run for.
///
/// A whole load of the heaviest measured document takes under 25 milliseconds with a warm disk
/// cache, most of it outside the interpreter. The ceiling is kept long on purpose: the wall clock
/// also runs while the skin reads its few hundred files, which a cold disk can stretch to seconds,
/// and the instruction ceiling above is what cuts a runaway loop promptly.
const DEFAULT_LOAD_MICROS: u64 = 10_000_000;

/// Function calls one frame may make.
///
/// Calling each of the 1,817 function values of the song select once makes 1,801 calls, nine times
/// under this ceiling. A renderer that calls a shared timer function once per object that uses it
/// makes more calls than there are functions (about 2,200 for the same screen by the survey's
/// count), and the rest of the ceiling is that headroom.
const DEFAULT_FRAME_CALLS: u32 = 16_384;

/// Microseconds the calls of one frame may spend inside Lua between them.
///
/// Calling every function value of the song select once takes 0.28 milliseconds on average, 0.34
/// at the 99th percentile and 0.65 at the worst of a thousand frames, which is 75 to 180 times under
/// this ceiling; no screen of the measured pack came near it. It is deliberately several frames long:
/// a call the meter refuses is answered with a stale value, so the ceiling is there to stop a runaway
/// skin, not to hold a slow one to a frame rate.
const DEFAULT_FRAME_MICROS: u64 = 50_000;

/// Instructions a single call made during a frame may execute.
///
/// No call of the measured pack needed more than the counting resolution, about two thousand
/// instructions, so this is some five hundred times what a call needs. A runaway call is cut after
/// about a millisecond.
const DEFAULT_CALL_INSTRUCTIONS: u64 = 1_000_000;

/// Bytes the interpreter may allocate.
///
/// The song select holds 6.1 MiB once it is loaded and every other measured screen under 2 MiB, so
/// this is some forty times the heaviest.
const DEFAULT_MEMORY_BYTES: usize = 256 * 1024 * 1024;

/// Instructions between two visits of the hook. The hook is cheap but not free, so it accounts for
/// instructions in blocks rather than one at a time.
const HOOK_INTERVAL_INSTRUCTIONS: u32 = 1_000;

/// The message of the error the hook raises. A skin never sees it: its own `pcall` passes the error
/// on, and the runtime reports the overrun as [`crate::SkinError::LuaBudget`].
pub const BUDGET_EXHAUSTED: &str = "the skin ran past its Lua budget";

/// The error a library written in Rust raises once [`Meter::charge`] has refused it. It is the
/// error the hook raises, so every catcher passes it on the same way.
pub(crate) fn budget_error() -> mlua::Error {
    mlua::Error::runtime(BUDGET_EXHAUSTED)
}

/// What one pass of a skin's entry file may spend. The header pass and the body pass are metered
/// separately, each against the whole of this.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadBudget {
    /// Instructions the pass may execute before it is cut off.
    pub max_instructions: u64,
    /// Wall clock the pass may run for. This is the only ceiling that time spent inside the
    /// standard library ever meets.
    pub max_micros: u64,
}

impl Default for LoadBudget {
    fn default() -> Self {
        Self { max_instructions: DEFAULT_LOAD_INSTRUCTIONS, max_micros: DEFAULT_LOAD_MICROS }
    }
}

/// What one frame of a loaded skin may spend, across every function it calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameBudget {
    /// Function calls the frame may make. A call past this is not made, and its caller gets the
    /// value that function last produced.
    pub max_calls: u32,
    /// Wall clock the calls of the frame may spend inside Lua altogether. Only the time between a
    /// call's start and its end is charged: whatever the host does between two calls -- drawing,
    /// most of all -- costs the skin nothing.
    pub max_micros: u64,
    /// Instructions a single call may execute before it is cut off as a runaway.
    pub max_call_instructions: u64,
}

impl Default for FrameBudget {
    fn default() -> Self {
        Self { max_calls: DEFAULT_FRAME_CALLS, max_micros: DEFAULT_FRAME_MICROS, max_call_instructions: DEFAULT_CALL_INSTRUCTIONS }
    }
}

/// Everything one skin's interpreter is held to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LuaBudget {
    pub load: LoadBudget,
    pub frame: FrameBudget,
    /// Bytes the interpreter may allocate in total, loading and drawing alike.
    pub max_memory_bytes: usize,
}

impl Default for LuaBudget {
    fn default() -> Self {
        Self { load: LoadBudget::default(), frame: FrameBudget::default(), max_memory_bytes: DEFAULT_MEMORY_BYTES }
    }
}

/// Which regime the interpreter is being metered under.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Phase {
    /// Nothing is running, so nothing is metered.
    #[default]
    Idle,
    /// One pass of the entry file, against [`LoadBudget`].
    Load,
    /// One frame of function calls, against [`FrameBudget`].
    Frame,
}

/// The cells the meter and the interpreter hook both write.
#[derive(Debug, Default)]
struct Account {
    phase: Cell<Phase>,
    exhausted: Cell<bool>,
    /// Instructions the running pass, or the running call of a frame, may still execute.
    instructions_left: Cell<u64>,
    /// When the code now running is out of wall clock: the end of the pass while loading, the end of
    /// the running call during a frame. `None` while idle and between the calls of a frame.
    deadline: Cell<Option<Instant>>,
    /// Wall clock the calls of the running frame may still spend.
    frame_time_left: Cell<Duration>,
    /// When the running call of a frame began. `None` between calls.
    call_started: Cell<Option<Instant>>,
    /// Calls the running frame has made.
    calls: Cell<u32>,
    /// Whether the running frame has already been counted as over budget.
    frame_counted: Cell<bool>,
    frames_over_budget: Cell<u64>,
}

impl Account {
    /// Resets the account for a pass that may execute `instructions` and run for `micros`.
    fn begin_load(&self, instructions: u64, micros: u64) {
        self.phase.set(Phase::Load);
        self.exhausted.set(false);
        self.instructions_left.set(instructions);
        self.deadline.set(Instant::now().checked_add(Duration::from_micros(micros)));
        self.call_started.set(None);
    }

    /// Resets the account for a frame whose calls may each execute `call_instructions` and may run
    /// for `micros` between them. No clock runs until the first call begins.
    fn begin_frame(&self, call_instructions: u64, micros: u64) {
        self.phase.set(Phase::Frame);
        self.exhausted.set(false);
        self.instructions_left.set(call_instructions);
        self.deadline.set(None);
        self.frame_time_left.set(Duration::from_micros(micros));
        self.call_started.set(None);
        self.calls.set(0);
        self.frame_counted.set(false);
    }

    /// Starts the clock of one call of a frame, against what the frame has left.
    fn begin_call(&self, instructions: u64) {
        let now = Instant::now();
        self.calls.set(self.calls.get() + 1);
        self.instructions_left.set(instructions);
        self.exhausted.set(false);
        self.call_started.set(Some(now));
        self.deadline.set(now.checked_add(self.frame_time_left.get()));
    }

    /// Stops the clock of the running call and takes what it spent out of what the frame has left.
    fn end_call(&self) {
        if let Some(started) = self.call_started.take() {
            self.frame_time_left.set(self.frame_time_left.get().saturating_sub(started.elapsed()));
            self.deadline.set(None);
        }
    }

    /// Whether the wall clock of the running pass or call is spent.
    fn overtime(&self) -> bool {
        self.deadline.get().is_some_and(|deadline| Instant::now() >= deadline)
    }

    /// Counts the running frame as over budget, once.
    fn count_frame(&self) {
        if self.phase.get() == Phase::Frame && !self.frame_counted.replace(true) {
            self.frames_over_budget.set(self.frames_over_budget.get() + 1);
        }
    }

    /// Charges `instructions` worth of work and says whether the interpreter may go on. This is one
    /// visit of the interpreter hook, and what a library written in Rust calls for the work it does
    /// where the hook cannot see.
    ///
    /// Once the account is exhausted every later charge refuses too, so code that keeps running after
    /// the first refusal -- a coroutine's resumer, say -- is cut off again within one block.
    fn charge(&self, instructions: u64) -> bool {
        if self.phase.get() == Phase::Idle {
            return true;
        }
        let left = self.instructions_left.get();
        if self.exhausted.get() || left < instructions || self.overtime() {
            self.exhausted.set(true);
            return false;
        }
        self.instructions_left.set(left - instructions);
        true
    }
}

/// The running account of what the current pass or frame has spent.
///
/// The runtime owns one and shares it with every installed library:
///
/// - [`Meter::install`] sets the interpreter's memory limit and an instruction-count hook that
///   raises a Lua error once the active phase is over budget and latches [`Meter::exhausted`]. The
///   hook checks the wall clock as well as the instruction count. It is a global hook, so a
///   coroutine a skin creates is metered against the same account as the code that resumes it.
/// - [`Meter::begin_load`] and [`Meter::begin_frame`] reset the account and select the phase;
///   [`Meter::finish`] returns to [`Phase::Idle`], in which nothing is metered.
/// - [`Meter::begin_call`] is asked before each function call a frame makes. It answers `false`
///   once the frame's call count or wall clock is spent, and the caller then reuses the function's
///   previous value instead of calling it. Each call it allows gets its own instruction allowance,
///   and [`Meter::end_call`] closes it. A frame's wall clock runs only between those two, so the
///   time the host spends drawing between two calls is not the skin's.
/// - `Meter::charge` is how a library written in Rust accounts for work the hook cannot see: the
///   pattern matcher charges its steps, the file helpers the bytes they read.
/// - [`Meter::frames_over_budget`] counts the frames in which at least one call was refused or cut
///   off, for the diagnostics.
///
/// What the hook cannot see is time spent inside one call of the C library: one enormous
/// `string.rep` or `table.concat` runs to its end before the next visit. The memory ceiling bounds
/// how long any one of them can take.
#[derive(Debug)]
pub struct Meter {
    budget: LuaBudget,
    account: Rc<Account>,
}

impl Meter {
    /// A meter for `budget`, idle.
    pub fn new(budget: LuaBudget) -> Self {
        Self { budget, account: Rc::new(Account::default()) }
    }

    /// The budget this meter enforces.
    pub fn budget(&self) -> LuaBudget {
        self.budget
    }

    /// Attaches the memory limit and the instruction hook to `lua`.
    pub fn install(&self, lua: &Lua) -> mlua::Result<()> {
        lua.set_memory_limit(self.budget.max_memory_bytes)?;
        let account = Rc::clone(&self.account);
        lua.set_global_hook(HookTriggers::new().every_nth_instruction(HOOK_INTERVAL_INSTRUCTIONS), move |_, _| {
            if account.charge(u64::from(HOOK_INTERVAL_INSTRUCTIONS)) { Ok(VmState::Continue) } else { Err(budget_error()) }
        })
    }

    /// Starts metering one pass of the entry file.
    pub fn begin_load(&self) {
        self.account.begin_load(self.budget.load.max_instructions, self.budget.load.max_micros);
    }

    /// Starts metering one frame.
    pub fn begin_frame(&self) {
        self.account.begin_frame(self.budget.frame.max_call_instructions, self.budget.frame.max_micros);
    }

    /// Stops metering.
    pub fn finish(&self) {
        self.account.phase.set(Phase::Idle);
        self.account.exhausted.set(false);
        self.account.deadline.set(None);
        self.account.call_started.set(None);
    }

    /// The regime being metered.
    pub fn phase(&self) -> Phase {
        self.account.phase.get()
    }

    /// Whether the frame may make one more call. Outside a frame every call is allowed and none is
    /// counted.
    pub fn begin_call(&self) -> bool {
        let account = &self.account;
        if account.phase.get() != Phase::Frame {
            return true;
        }
        if account.calls.get() >= self.budget.frame.max_calls || account.frame_time_left.get().is_zero() {
            account.count_frame();
            return false;
        }
        account.begin_call(self.budget.frame.max_call_instructions);
        true
    }

    /// Closes the call [`Meter::begin_call`] opened, counting the frame as over budget when the call
    /// was cut off.
    pub fn end_call(&self) {
        self.account.end_call();
        if self.account.exhausted.get() {
            self.account.count_frame();
        }
    }

    /// Charges `instructions` worth of work done outside the interpreter's own count, and answers
    /// whether the caller may go on. A refusal latches [`Meter::exhausted`], so the caller raises
    /// [`budget_error`] and nothing in the skin can catch it. Nothing is charged while idle.
    pub(crate) fn charge(&self, instructions: u64) -> bool {
        self.account.charge(instructions)
    }

    /// Whether the hook has cut off the running pass, or the running call of a frame. It is cleared
    /// when the next pass, frame or call begins and when metering stops.
    pub fn exhausted(&self) -> bool {
        self.account.exhausted.get()
    }

    /// Latches [`Meter::exhausted`] for an overrun the hook did not see itself: the allocator
    /// refusing memory, or a library holding more outside the interpreter than the interpreter may
    /// hold. Nothing is latched while idle.
    pub(crate) fn cut_off(&self) {
        if self.account.phase.get() != Phase::Idle {
            self.account.exhausted.set(true);
        }
    }

    /// How many frames have had at least one call refused or cut off.
    pub fn frames_over_budget(&self) -> u64 {
        self.account.frames_over_budget.get()
    }
}
