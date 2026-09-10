//! The Lua sandbox a document's expression-typed fields are evaluated in.
//!
//! A document may write `"timer": 41` or `"timer": "skin.number(70) > 0"`; the second form is a Lua
//! expression. Those expressions come from files a player downloads, so this interpreter is built
//! to be dull: three standard libraries, six functions of our own, no way to reach a file, a
//! memory ceiling and an instruction ceiling. An expression that breaks a rule hides its object and
//! warns once; it never fails the skin and never escapes.
//!
//! Expressions are compiled once at load time and only called afterwards, so a frame costs a call
//! rather than a parse.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use mlua::chunk::ChunkMode;
use mlua::{HookTriggers, Lua, LuaOptions, StdLib, Table, Value, VmState};

use crate::SkinError;
use crate::dst::{LuaDrawEval, LuaExprId, WarnOnce};
pub use crate::loader::Budget;
/// The whole game state a skin expression can read.
///
/// This is the property registry's own trait (spec section 4.3), re-exported here because the six
/// whitelisted `skin.*` functions are its accessors and nothing else. It is deliberately not a
/// second trait of the same name: one implementation serves the registry, the interpolator's draw
/// gating and this evaluator alike.
pub use crate::property::SkinStateSource;

/// Instructions between two budget checks. The hook is cheap but not free, so it fires in blocks
/// rather than on every instruction.
const HOOK_INTERVAL_INSTRUCTIONS: u32 = 1_000;

/// The seed `math.randomseed` is pinned to, so a document that reaches for randomness still draws
/// the same picture on every machine and in every golden run.
const RANDOM_SEED: i64 = 0x5eed_5eed;

/// The global table the whitelisted API is published under.
const SKIN_TABLE: &str = "skin";

/// The name a compiled expression carries in the interpreter's own messages.
///
/// Without one, the interpreter names the Rust call site, which would put a path inside this crate
/// into a warning a player reads. The leading `=` is Lua's marker for a name to be shown verbatim
/// rather than quoted as a source string, and [`SkinError::Lua`] already carries the expression the
/// message belongs to.
const EXPRESSION_CHUNK_NAME: &str = "=[skin expression]";

/// The globals removed before any document is compiled.
///
/// Loading code at runtime, reading a file, requiring a module and rewriting another table's
/// metatable are all off the table; the rest were never loaded to begin with and are nilled anyway
/// so that a test can assert on one list.
const FORBIDDEN_GLOBALS: &[&str] = &[
    "dofile",
    "loadfile",
    "load",
    "loadstring",
    "require",
    "collectgarbage",
    "rawset",
    "rawget",
    "rawequal",
    "rawlen",
    "setmetatable",
    "getmetatable",
    "newproxy",
    "print",
    "io",
    "os",
    "package",
    "debug",
];

/// The `string` functions removed before any document is compiled.
///
/// `dump` hands out bytecode, which is the other half of keeping the loader off binary chunks. The
/// four pattern functions are removed because a pattern with several `.-` captures backtracks for
/// as long as its subject is long, entirely inside C, where neither the instruction hook nor the
/// allocator ever sees it -- the one place in this interpreter where a short expression can run
/// unbounded. Formatting, slicing and case folding are left alone.
const FORBIDDEN_STRING_FUNCTIONS: &[&str] = &["dump", "find", "gmatch", "gsub", "match"];

/// The interpreter a skin's expressions live in.
pub struct LuaSandbox {
    lua: Lua,
    root: PathBuf,
    budget: Budget,
    remaining: std::rc::Rc<Cell<u32>>,
    exhausted: std::rc::Rc<Cell<bool>>,
    started: std::rc::Rc<Cell<Instant>>,
    functions: RefCell<Vec<mlua::Function>>,
    by_source: RefCell<BTreeMap<String, LuaExprId>>,
    warned: WarnOnce,
}

impl std::fmt::Debug for LuaSandbox {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("LuaSandbox").field("root", &self.root).field("budget", &self.budget).field("compiled", &self.functions.borrow().len()).finish()
    }
}

impl LuaSandbox {
    /// A sandbox for the skin rooted at `root`.
    ///
    /// `root` is recorded for the loader's own file resolution; nothing inside the interpreter can
    /// reach it, because nothing inside the interpreter can open a file at all.
    pub fn new(root: &Path, budget: Budget) -> Result<Self, SkinError> {
        let lua = Lua::new_with(StdLib::MATH | StdLib::STRING | StdLib::TABLE, LuaOptions::default()).map_err(|error| lua_error("sandbox", &error))?;
        lua.set_memory_limit(budget.max_memory_bytes).map_err(|error| lua_error("memory limit", &error))?;

        let globals = lua.globals();
        for name in FORBIDDEN_GLOBALS {
            globals.set(*name, Value::Nil).map_err(|error| lua_error(name, &error))?;
        }
        let strings: Table = globals.get("string").map_err(|error| lua_error("string", &error))?;
        for name in FORBIDDEN_STRING_FUNCTIONS {
            strings.set(*name, Value::Nil).map_err(|error| lua_error(name, &error))?;
        }
        lua.load("math.randomseed(...)").call::<()>(RANDOM_SEED).map_err(|error| lua_error("randomseed", &error))?;

        let remaining = std::rc::Rc::new(Cell::new(budget.max_instructions));
        let exhausted = std::rc::Rc::new(Cell::new(false));
        let started = std::rc::Rc::new(Cell::new(Instant::now()));
        let hook_remaining = std::rc::Rc::clone(&remaining);
        let hook_exhausted = std::rc::Rc::clone(&exhausted);
        let hook_started = std::rc::Rc::clone(&started);
        let call_micros = budget.max_call_micros;
        lua.set_hook(HookTriggers::new().every_nth_instruction(HOOK_INTERVAL_INSTRUCTIONS), move |_, _| {
            let left = hook_remaining.get();
            let overtime = hook_started.get().elapsed().as_micros() as u64 > call_micros;
            if left < HOOK_INTERVAL_INSTRUCTIONS || overtime {
                hook_exhausted.set(true);
                return Err(mlua::Error::RuntimeError("expression budget exhausted".to_owned()));
            }
            hook_remaining.set(left - HOOK_INTERVAL_INSTRUCTIONS);
            Ok(VmState::Continue)
        })
        .map_err(|error| lua_error("hook", &error))?;

        Ok(Self {
            lua,
            root: root.to_path_buf(),
            budget,
            remaining,
            exhausted,
            started,
            functions: RefCell::new(Vec::new()),
            by_source: RefCell::new(BTreeMap::new()),
            warned: WarnOnce::new(),
        })
    }

    /// The skin root this sandbox was built for.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The budget every expression is held to.
    pub fn budget(&self) -> Budget {
        self.budget
    }

    /// How many distinct expressions have been compiled.
    pub fn compiled_count(&self) -> usize {
        self.functions.borrow().len()
    }

    /// Compiles one expression and returns its handle, reusing the handle for a repeated source.
    ///
    /// A document writes an expression, not a chunk, so the source is wrapped in a `return` the way
    /// `SkinLuaAccessor` wraps it. A source that already forms a whole chunk is compiled as written
    /// instead, which the reference does not allow and which costs nothing to accept.
    pub fn compile(&self, source: &str) -> Result<LuaExprId, SkinError> {
        if let Some(id) = self.by_source.borrow().get(source) {
            return Ok(*id);
        }
        let function = match self.lua.load(format!("return {source}")).set_name(EXPRESSION_CHUNK_NAME).set_mode(ChunkMode::Text).into_function() {
            Ok(function) => function,
            Err(_) => {
                self.lua.load(source).set_name(EXPRESSION_CHUNK_NAME).set_mode(ChunkMode::Text).into_function().map_err(|error| lua_error(source, &error))?
            }
        };
        let mut functions = self.functions.borrow_mut();
        let id = LuaExprId(functions.len() as u32);
        functions.push(function);
        self.by_source.borrow_mut().insert(source.to_owned(), id);
        Ok(id)
    }

    /// Runs one compiled expression and returns its raw Lua value.
    fn call(&self, expr: LuaExprId, state: &dyn SkinStateSource) -> Result<Value, SkinError> {
        let function = self.functions.borrow().get(expr.0 as usize).cloned();
        let Some(function) = function else {
            return Err(SkinError::Lua { expr: format!("expression {}", expr.0), message: "no such compiled expression".to_owned() });
        };
        self.remaining.set(self.budget.max_instructions);
        self.exhausted.set(false);
        self.started.set(Instant::now());

        let outcome = self.lua.scope(|scope| {
            let table = self.lua.create_table()?;
            table.set("boolean", scope.create_function(move |_, id: i32| Ok(state.boolean(id)))?)?;
            table.set("number", scope.create_function(move |_, id: i32| Ok(state.integer(id)))?)?;
            table.set("float", scope.create_function(move |_, id: i32| Ok(state.float(id)))?)?;
            table.set("text", scope.create_function(move |_, id: i32| Ok(state.string(id).to_owned()))?)?;
            table.set("timer", scope.create_function(move |_, id: i32| Ok(state.timer(id)))?)?;
            table.set("time", scope.create_function(move |_, (): ()| Ok(state.now_ms()))?)?;
            self.lua.globals().set(SKIN_TABLE, table)?;
            function.call::<Value>(())
        });
        let cleared = self.lua.globals().set(SKIN_TABLE, Value::Nil);

        match outcome {
            Ok(value) => {
                cleared.map_err(|error| lua_error(SKIN_TABLE, &error))?;
                Ok(value)
            }
            Err(error) if self.exhausted.get() || matches!(error, mlua::Error::MemoryError(_)) => Err(SkinError::LuaBudget { expr: self.source_of(expr) }),
            Err(error) => Err(lua_error(&self.source_of(expr), &error)),
        }
    }

    /// The source an expression handle was compiled from, for error messages.
    fn source_of(&self, expr: LuaExprId) -> String {
        self.by_source.borrow().iter().find(|(_, id)| **id == expr).map(|(source, _)| source.clone()).unwrap_or_else(|| format!("expression {}", expr.0))
    }

    /// Runs a compiled expression as a condition, with Lua's own notion of truth.
    pub fn eval_bool_id(&self, expr: LuaExprId, state: &dyn SkinStateSource) -> Result<bool, SkinError> {
        let value = self.call(expr, state)?;
        Ok(!matches!(value, Value::Nil | Value::Boolean(false)))
    }

    /// Runs a compiled expression as an integer.
    pub fn eval_int_id(&self, expr: LuaExprId, state: &dyn SkinStateSource) -> Result<i32, SkinError> {
        let value = self.call(expr, state)?;
        as_number(&value).map(|number| number.trunc() as i32).ok_or_else(|| type_error(&self.source_of(expr), &value, "a number"))
    }

    /// Runs a compiled expression as a float.
    pub fn eval_float_id(&self, expr: LuaExprId, state: &dyn SkinStateSource) -> Result<f32, SkinError> {
        let value = self.call(expr, state)?;
        as_number(&value).map(|number| number as f32).ok_or_else(|| type_error(&self.source_of(expr), &value, "a number"))
    }

    /// Runs a compiled expression as text.
    pub fn eval_string_id(&self, expr: LuaExprId, state: &dyn SkinStateSource) -> Result<String, SkinError> {
        let value = self.call(expr, state)?;
        match &value {
            Value::String(text) => Ok(text.to_string_lossy()),
            Value::Integer(number) => Ok(number.to_string()),
            Value::Number(number) => Ok(number.to_string()),
            _ => Err(type_error(&self.source_of(expr), &value, "a string")),
        }
    }

    /// Compiles `source` if it is new, then runs it as a condition.
    pub fn eval_bool(&self, source: &str, state: &dyn SkinStateSource) -> Result<bool, SkinError> {
        self.eval_bool_id(self.compile(source)?, state)
    }

    /// Compiles `source` if it is new, then runs it as an integer.
    pub fn eval_int(&self, source: &str, state: &dyn SkinStateSource) -> Result<i32, SkinError> {
        self.eval_int_id(self.compile(source)?, state)
    }

    /// Compiles `source` if it is new, then runs it as a float.
    pub fn eval_float(&self, source: &str, state: &dyn SkinStateSource) -> Result<f32, SkinError> {
        self.eval_float_id(self.compile(source)?, state)
    }

    /// Compiles `source` if it is new, then runs it as text.
    pub fn eval_string(&self, source: &str, state: &dyn SkinStateSource) -> Result<String, SkinError> {
        self.eval_string_id(self.compile(source)?, state)
    }

    /// Binds this sandbox to one frame's state, giving the interpolator something to ask.
    pub fn frame<'a>(&'a self, state: &'a dyn SkinStateSource) -> LuaFrame<'a> {
        LuaFrame { sandbox: self, state, calls: Cell::new(0), spent_micros: Cell::new(0) }
    }
}

/// One frame's view of a sandbox: the same compiled expressions, bound to this frame's state.
///
/// Every read a document makes goes through here -- draw gating, and the `value`, `floatvalue` and
/// `text` fields alike -- because this is what holds the per-frame ceilings. An expression that a
/// hundred objects point at is cut off rather than stalling the frame, and a frame that spends its
/// whole Lua allowance stops evaluating rather than running over.
pub struct LuaFrame<'a> {
    sandbox: &'a LuaSandbox,
    state: &'a dyn SkinStateSource,
    calls: Cell<u32>,
    spent_micros: Cell<u64>,
}

impl LuaFrame<'_> {
    /// How many expressions this frame has evaluated.
    pub fn calls(&self) -> u32 {
        self.calls.get()
    }

    /// How long this frame has spent inside Lua, in microseconds.
    pub fn spent_micros(&self) -> u64 {
        self.spent_micros.get()
    }

    /// Whether this frame still has budget, warning once the first time it does not.
    fn may_evaluate(&self) -> bool {
        let budget = self.sandbox.budget;
        if self.calls.get() >= budget.max_calls_per_frame {
            if self.sandbox.warned.should_warn() {
                eprintln!("skin Lua expressions passed {} evaluations in one frame; the rest keep their defaults", budget.max_calls_per_frame);
            }
            return false;
        }
        if self.spent_micros.get() >= budget.max_frame_micros {
            if self.sandbox.warned.should_warn() {
                eprintln!("skin Lua expressions passed {}us in one frame; the rest keep their defaults", budget.max_frame_micros);
            }
            return false;
        }
        true
    }

    /// Runs one evaluation against this frame's budget, or reports that it was not run.
    ///
    /// A failure is warned about once and answered with `None`, which each caller turns into the
    /// default its field wants: a hidden object for a draw condition, an unread value elsewhere.
    fn spend<T>(&self, run: impl FnOnce(&LuaSandbox, &dyn SkinStateSource) -> Result<T, SkinError>) -> Option<T> {
        if !self.may_evaluate() {
            return None;
        }
        self.calls.set(self.calls.get() + 1);
        let started = Instant::now();
        let outcome = run(self.sandbox, self.state);
        self.spent_micros.set(self.spent_micros.get().saturating_add(started.elapsed().as_micros() as u64));
        match outcome {
            Ok(value) => Some(value),
            Err(error) => {
                if self.sandbox.warned.should_warn() {
                    eprintln!("skin Lua expression failed and its object keeps its default: {error}");
                }
                None
            }
        }
    }

    /// The expression's whole number this frame.
    pub fn eval_int(&self, expr: LuaExprId) -> Option<i32> {
        self.spend(|sandbox, state| sandbox.eval_int_id(expr, state))
    }

    /// The expression's number this frame.
    pub fn eval_float(&self, expr: LuaExprId) -> Option<f32> {
        self.spend(|sandbox, state| sandbox.eval_float_id(expr, state))
    }

    /// The expression's text this frame.
    pub fn eval_string(&self, expr: LuaExprId) -> Option<String> {
        self.spend(|sandbox, state| sandbox.eval_string_id(expr, state))
    }
}

impl std::fmt::Debug for LuaFrame<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("LuaFrame").field("calls", &self.calls.get()).field("spent_micros", &self.spent_micros.get()).finish()
    }
}

impl LuaDrawEval for LuaFrame<'_> {
    fn eval_draw(&self, expr: LuaExprId) -> Option<bool> {
        self.spend(|sandbox, state| sandbox.eval_bool_id(expr, state))
    }
}

/// The numeric value of a Lua result, following Lua's own string-to-number coercion.
fn as_number(value: &Value) -> Option<f64> {
    match value {
        Value::Integer(number) => Some(*number as f64),
        Value::Number(number) => Some(*number),
        Value::Boolean(flag) => Some(f64::from(u8::from(*flag))),
        Value::String(text) => text.to_string_lossy().trim().parse().ok(),
        _ => None,
    }
}

/// A compile or runtime failure, named by the expression that caused it.
fn lua_error(expr: &str, error: &mlua::Error) -> SkinError {
    SkinError::Lua { expr: expr.to_owned(), message: error.to_string() }
}

/// A result of the wrong shape, named by the expression that produced it.
fn type_error(expr: &str, value: &Value, expected: &str) -> SkinError {
    SkinError::Lua { expr: expr.to_owned(), message: format!("returned {} where {expected} was expected", value.type_name()) }
}
