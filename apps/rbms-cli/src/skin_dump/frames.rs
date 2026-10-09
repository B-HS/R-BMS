//! Calling every function value of a loaded skin, frame after frame, and timing it.
//!
//! A screen draws by calling the functions its skin handed over: conditions, timers and values. A
//! frame here does the same with every one of them exactly once, against the scenario's host with
//! the scene clock moved on by one frame, and the time the whole frame takes is what is recorded.
//! The functions that react to a click or to input (events and writers) are not called, because
//! nothing in a frame calls them.

use std::time::{Duration, Instant};

use rbms_skin::dst::LuaFnId;
use rbms_skin::lua::{BoundFrame, LuaDiagnostics, LuaFnKind, SkinLua};
use rbms_skin::property::MapHost;

use super::report::{EntryReport, FramesReport, SlowCallReport};

/// Microseconds the scene clock advances by in every frame: sixty frames a second.
pub const FRAME_MICROS: i64 = 16_667;

/// The share of frames at or under the percentile the report calls p99, as numerator and
/// denominator.
const P99: (usize, usize) = (99, 100);

/// The share of frames at or under the median.
const MEDIAN: (usize, usize) = (1, 2);

/// How many of the slowest functions the report names.
const SLOWEST_CALLS_SHOWN: usize = 5;

const MICROS_PER_SECOND: f64 = 1_000_000.0;

/// What a function value of this kind is called as, and so whether a frame calls it.
pub const fn is_frame_kind(kind: LuaFnKind) -> bool {
    matches!(kind, LuaFnKind::Boolean | LuaFnKind::Integer | LuaFnKind::Float | LuaFnKind::Text | LuaFnKind::Timer)
}

pub const fn kind_label(kind: LuaFnKind) -> &'static str {
    match kind {
        LuaFnKind::Boolean => "boolean",
        LuaFnKind::Integer => "integer",
        LuaFnKind::Float => "float",
        LuaFnKind::Text => "text",
        LuaFnKind::Timer => "timer",
        LuaFnKind::Event => "event",
        LuaFnKind::FloatWriter => "float writer",
        LuaFnKind::TextWriter => "text writer",
    }
}

/// Every kind of function value, in the order the report lists them.
pub const ALL_KINDS: [LuaFnKind; 8] = [
    LuaFnKind::Boolean,
    LuaFnKind::Integer,
    LuaFnKind::Float,
    LuaFnKind::Text,
    LuaFnKind::Timer,
    LuaFnKind::Event,
    LuaFnKind::FloatWriter,
    LuaFnKind::TextWriter,
];

/// The function values a frame calls, in the order the skin registered them.
pub fn frame_functions(runtime: &SkinLua) -> Vec<(LuaFnId, LuaFnKind)> {
    (0..runtime.function_count())
        .filter_map(|index| {
            let id = LuaFnId(u32::try_from(index).ok()?);
            runtime.kind_of(id).filter(|kind| is_frame_kind(*kind)).map(|kind| (id, kind))
        })
        .collect()
}

fn call(frame: &BoundFrame<'_>, function: LuaFnId, kind: LuaFnKind) {
    match kind {
        LuaFnKind::Boolean => {
            frame.call_boolean(function);
        }
        LuaFnKind::Integer => {
            frame.call_integer(function);
        }
        LuaFnKind::Float => {
            frame.call_float(function);
        }
        LuaFnKind::Text => {
            frame.call_text(function);
        }
        LuaFnKind::Timer => {
            frame.call_timer(function);
        }
        LuaFnKind::Event | LuaFnKind::FloatWriter | LuaFnKind::TextWriter => {}
    }
}

/// The nearest-rank percentile of an ascending list: the smallest value that at least `share` of the
/// list is at or under.
pub fn percentile(sorted: &[Duration], share: (usize, usize)) -> Duration {
    let rank = (sorted.len() * share.0).div_ceil(share.1).max(1);
    sorted.get(rank - 1).copied().unwrap_or_default()
}

fn micros(duration: Duration) -> f64 {
    duration.as_secs_f64() * MICROS_PER_SECOND
}

fn failure_entries(after: &LuaDiagnostics, before: &LuaDiagnostics) -> Vec<EntryReport> {
    after
        .function_failures
        .iter()
        .filter_map(|failure| {
            let earlier = before.function_failures.iter().find(|candidate| candidate.function == failure.function).map_or(0, |candidate| candidate.count);
            let count = failure.count.saturating_sub(earlier);
            (count > 0)
                .then(|| EntryReport { text: format!("function #{} ({}): {}", failure.function.0, kind_label(failure.kind), failure.first_message), count })
        })
        .collect()
}

fn swallowed_total(diagnostics: &LuaDiagnostics) -> u64 {
    diagnostics.swallowed.iter().map(|error| error.count).sum::<u64>() + diagnostics.swallowed_overflow
}

/// Moves the scene clock to the start of frame `index`.
fn advance(host: &mut MapHost, first_us: i64, index: usize) {
    host.now_us = first_us + i64::try_from(index).unwrap_or(i64::MAX) * FRAME_MICROS;
}

/// Runs `frames` untimed frames and answers in how many of them the budget refused or cut off a
/// call, or `None` when a frame could not run at all.
pub fn count_over_budget(runtime: &SkinLua, host: &mut MapHost, frames: usize) -> Option<u64> {
    let functions = frame_functions(runtime);
    let first_us = host.now_us;
    for index in 0..frames {
        advance(host, first_us, index);
        runtime
            .frame(&*host, |frame| {
                for (function, kind) in &functions {
                    call(frame, *function, *kind);
                }
            })
            .ok()?;
    }
    Some(runtime.diagnostics().frames_over_budget)
}

/// Runs `frames` frames and answers what they cost.
///
/// The frame times come from one pass that times only whole frames. A second pass over the same
/// frames then times each call on its own, which is dearer per call and so is kept out of the
/// frame times, to name the slowest functions.
pub fn measure(runtime: &SkinLua, host: &mut MapHost, frames: usize) -> Result<FramesReport, String> {
    let functions = frame_functions(runtime);
    let before = runtime.diagnostics();
    let first_us = host.now_us;

    let mut durations = Vec::with_capacity(frames);
    for index in 0..frames {
        advance(host, first_us, index);
        let started = Instant::now();
        runtime
            .frame(&*host, |frame| {
                for (function, kind) in &functions {
                    call(frame, *function, *kind);
                }
            })
            .map_err(|e| format!("frame {index} did not run: {e}"))?;
        durations.push(started.elapsed());
    }
    let after = runtime.diagnostics();

    let slowest_calls = slowest(runtime, host, &functions, first_us, frames)?;

    let total: Duration = durations.iter().sum();
    durations.sort_unstable();
    Ok(FramesReport {
        frames,
        calls_per_frame: functions.len(),
        mean_micros: micros(total) / frames.max(1) as f64,
        median_micros: micros(percentile(&durations, MEDIAN)),
        p99_micros: micros(percentile(&durations, P99)),
        max_micros: micros(durations.last().copied().unwrap_or_default()),
        frames_over_budget: after.frames_over_budget.saturating_sub(before.frames_over_budget),
        function_failures: failure_entries(&after, &before),
        swallowed_during_frames: swallowed_total(&after).saturating_sub(swallowed_total(&before)),
        slowest_calls,
    })
}

fn slowest(runtime: &SkinLua, host: &mut MapHost, functions: &[(LuaFnId, LuaFnKind)], first_us: i64, frames: usize) -> Result<Vec<SlowCallReport>, String> {
    let mut longest = vec![Duration::ZERO; functions.len()];
    let mut total = vec![Duration::ZERO; functions.len()];
    for index in 0..frames {
        advance(host, first_us, index);
        runtime
            .frame(&*host, |frame| {
                for (slot, (function, kind)) in functions.iter().enumerate() {
                    let started = Instant::now();
                    call(frame, *function, *kind);
                    let spent = started.elapsed();
                    longest[slot] = longest[slot].max(spent);
                    total[slot] += spent;
                }
            })
            .map_err(|e| format!("frame {index} did not run: {e}"))?;
    }
    let mut ranked: Vec<usize> = (0..functions.len()).collect();
    ranked.sort_unstable_by(|a, b| longest[*b].cmp(&longest[*a]));
    Ok(ranked
        .into_iter()
        .take(SLOWEST_CALLS_SHOWN)
        .map(|slot| SlowCallReport {
            function: functions[slot].0.0,
            kind: kind_label(functions[slot].1),
            max_micros: micros(longest[slot]),
            mean_micros: micros(total[slot]) / frames.max(1) as f64,
        })
        .collect())
}
