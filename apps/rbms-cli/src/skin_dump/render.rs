//! A dump as text a person reads: one block per document, then one table across all of them.

use super::report::{BodyReport, CountReport, DocumentReport, DumpReport, EntryReport, FileReport, FilesReport, FramesReport, HeaderReport, ProbeReport};

const BYTES_PER_KIBIBYTE: usize = 1024;
const MICROS_PER_MILLI: f64 = 1_000.0;

/// How many entries of one diagnostics list are printed before the rest is only counted.
const ENTRIES_SHOWN: usize = 12;

/// Longest a diagnostics message is printed at, so one stack trace does not fill a screen.
const MESSAGE_CHARS: usize = 220;

/// Instructions between two counts of the interpreter. An allowance is only resolved to about two of
/// these, so anything at or under them is printed as the resolution rather than as a measurement.
const COUNTING_RESOLUTION: u64 = 2_000;

fn millis(micros: f64) -> String {
    format!("{:.3} ms", micros / MICROS_PER_MILLI)
}

fn shorten(text: &str) -> String {
    let first_line = text.lines().next().unwrap_or_default();
    match first_line.char_indices().nth(MESSAGE_CHARS) {
        Some((end, _)) => format!("{}...", &first_line[..end]),
        None => first_line.to_owned(),
    }
}

fn nonzero(counts: &[CountReport]) -> String {
    let parts: Vec<String> = counts.iter().filter(|entry| entry.count > 0).map(|entry| format!("{} {}", entry.kind, entry.count)).collect();
    if parts.is_empty() { "none".to_owned() } else { parts.join(", ") }
}

fn instructions(value: Option<u64>, limit: u64) -> String {
    match value {
        Some(value) if value <= COUNTING_RESOLUTION => {
            format!("at most {COUNTING_RESOLUTION} instructions, the counting resolution; the default allows {limit}")
        }
        Some(value) => format!("about {value} instructions; the default allows {limit}, {:.0}x as many", limit as f64 / value as f64),
        None => format!("more than the {limit} instructions the default allows"),
    }
}

fn entries(label: &str, list: &[EntryReport], overflow: u64, lines: &mut Vec<String>) {
    let total: u64 = list.iter().map(|entry| entry.count).sum::<u64>() + overflow;
    lines.push(format!("  {label}: {} distinct, {total} in all", list.len()));
    for entry in list.iter().take(ENTRIES_SHOWN) {
        lines.push(format!("    x{} {}", entry.count, shorten(&entry.text)));
    }
    if list.len() > ENTRIES_SHOWN {
        lines.push(format!("    ... {} more", list.len() - ENTRIES_SHOWN));
    }
}

fn header_lines(header: &HeaderReport, lines: &mut Vec<String>) {
    lines.push(format!(
        "  header      type {} ({})  {:?} by {:?}  {}x{}  read as {}",
        header.skin_type, header.type_label, header.name, header.author, header.width, header.height, header.parser
    ));
    lines.push(format!(
        "  rows        options {} ({} with a default)  file slots {} ({} with a default)  offsets {} (+{} automatic)  categories {}",
        header.options.len(),
        HeaderReport::defaults(&header.options),
        header.file_slots.len(),
        HeaderReport::defaults(&header.file_slots),
        header.offsets.len() - header.automatic_offsets,
        header.automatic_offsets,
        header.categories
    ));
}

fn files_lines(files: &FilesReport, lines: &mut Vec<String>) {
    let section = |label: &str, declared: usize, resolved: &[FileReport]| {
        format!(
            "{label} {declared} declared, {} resolved, {} on disk, {} missing",
            resolved.len(),
            resolved.len() - FilesReport::missing(resolved),
            FilesReport::missing(resolved)
        )
    };
    lines.push(format!(
        "  files       {}  |  {}",
        section("sources", files.sources_declared, &files.sources),
        section("fonts", files.fonts_declared, &files.fonts)
    ));
    for file in files.sources.iter().chain(&files.fonts).filter(|file| !file.exists) {
        lines.push(format!("    missing {}: {}", file.id, file.path));
    }
}

fn body_lines(body: &BodyReport, lines: &mut Vec<String>) {
    let timings = &body.timings;
    lines.push(format!(
        "  timings     scene {}  input {}  fadeout {}  loadend {}  playstart {}  close {}",
        timings.scene, timings.input, timings.fadeout, timings.loadend, timings.playstart, timings.close
    ));
    lines.push(format!("  objects     destinations {} (assembled {}); {}", body.destinations, body.destinations_assembled, nonzero(&body.objects)));
    lines.push(format!("  functions   {} registered: {}", body.functions, nonzero(&body.function_kinds)));
    files_lines(&body.files, lines);
    let memory = body.lua_memory_bytes.map_or_else(|| "none".to_owned(), |bytes| format!("{} KiB", bytes / BYTES_PER_KIBIBYTE));
    lines.push(format!("  lua memory  {memory}  |  customisation rows selected: {}", body.options_selected));
    let diagnostics = &body.diagnostics;
    lines.push(format!("  diagnostics warnings {}", diagnostics.warnings.len()));
    for warning in diagnostics.warnings.iter().take(ENTRIES_SHOWN) {
        lines.push(format!("    {}", shorten(warning)));
    }
    entries("caught by the skin's own pcall", &diagnostics.swallowed, diagnostics.swallowed_overflow, lines);
    entries("function failures", &diagnostics.function_failures, 0, lines);
    entries("printed", &diagnostics.prints, diagnostics.prints_overflow, lines);
}

fn frames_lines(frames: &FramesReport, lines: &mut Vec<String>) {
    lines.push(format!(
        "  frames      {} frames x {} calls  mean {}  median {}  p99 {}  max {}  over budget {}",
        frames.frames,
        frames.calls_per_frame,
        millis(frames.mean_micros),
        millis(frames.median_micros),
        millis(frames.p99_micros),
        millis(frames.max_micros),
        frames.frames_over_budget
    ));
    entries("function failures during the frames", &frames.function_failures, 0, lines);
    lines.push(format!("  caught by pcall during the frames: {}", frames.swallowed_during_frames));
    for call in &frames.slowest_calls {
        lines.push(format!("    slowest #{} ({}): max {}  mean {}", call.function, call.kind, millis(call.max_micros), millis(call.mean_micros)));
    }
}

fn probe_lines(probe: &ProbeReport, lines: &mut Vec<String>) {
    if probe.load_instructions.is_none()
        && let Some(note) = &probe.note
    {
        lines.push(format!("  probe       {note}"));
        return;
    }
    lines.push(format!("  load pass   needs {}", instructions(probe.load_instructions, probe.load_instructions_limit)));
    lines.push(format!(
        "  one call    the heaviest call of {} frames needs {}",
        probe.probe_frames,
        instructions(probe.call_instructions, probe.call_instructions_limit)
    ));
    if let Some(note) = &probe.note {
        lines.push(format!("  probe note  {note}"));
    }
}

fn document_lines(document: &DocumentReport, lines: &mut Vec<String>) {
    lines.push(format!("== {} ==", document.file));
    match (&document.header, &document.header_error) {
        (Some(header), _) => header_lines(header, lines),
        (None, Some(error)) => lines.push(format!("  header      FAILED: {}", shorten(error))),
        (None, None) => {}
    }
    match (&document.body, &document.load_error) {
        (Some(body), _) => {
            lines.push(format!("  load        ok in {:.1} ms  scenario: {}", document.load_millis, document.scenario));
            body_lines(body, lines);
        }
        (None, Some(error)) => {
            lines.push(format!("  load        FAILED after {:.1} ms  scenario: {}", document.load_millis, document.scenario));
            for line in error.lines().take(ENTRIES_SHOWN) {
                lines.push(format!("    {line}"));
            }
        }
        (None, None) => {}
    }
    if let Some(frames) = &document.frames {
        frames_lines(frames, lines);
    }
    if let Some(error) = &document.frames_error {
        lines.push(format!("  frames      FAILED: {error}"));
    }
    if let Some(probe) = &document.probe {
        probe_lines(probe, lines);
    }
    lines.push(String::new());
}

fn summary_row(document: &DocumentReport) -> Vec<String> {
    let type_text = document.header.as_ref().map_or_else(|| "-".to_owned(), |header| format!("{} {}", header.skin_type, header.type_label));
    let Some(body) = &document.body else {
        let mut row = vec![document.file.clone(), type_text, "FAILED".to_owned()];
        row.resize(SUMMARY_HEADER.len(), "-".to_owned());
        return row;
    };
    let frames = document.frames.as_ref();
    let frame_cell = |pick: fn(&FramesReport) -> f64| frames.map_or_else(|| "-".to_owned(), |frames| millis(pick(frames)));
    vec![
        document.file.clone(),
        type_text,
        "ok".to_owned(),
        body.destinations.to_string(),
        body.functions.to_string(),
        body.diagnostics.swallowed.len().to_string(),
        (body.diagnostics.function_failures.len() + frames.map_or(0, |frames| frames.function_failures.len())).to_string(),
        body.diagnostics.warnings.len().to_string(),
        body.lua_memory_bytes.map_or_else(|| "-".to_owned(), |bytes| (bytes / BYTES_PER_KIBIBYTE).to_string()),
        format!("{:.1}", document.load_millis),
        frames.map_or_else(|| "-".to_owned(), |frames| frames.calls_per_frame.to_string()),
        frame_cell(|frames| frames.mean_micros),
        frame_cell(|frames| frames.p99_micros),
        frame_cell(|frames| frames.max_micros),
        frames.map_or_else(|| "-".to_owned(), |frames| frames.frames_over_budget.to_string()),
    ]
}

const SUMMARY_HEADER: [&str; 15] =
    ["document", "type", "load", "dest", "fns", "pcall", "fn fail", "warn", "lua KiB", "load ms", "calls", "mean", "p99", "max", "over"];

fn table(rows: &[Vec<String>]) -> Vec<String> {
    let widths: Vec<usize> = (0..SUMMARY_HEADER.len()).map(|column| rows.iter().map(|row| row[column].chars().count()).max().unwrap_or(0)).collect();
    rows.iter()
        .map(|row| format!("  {}", row.iter().zip(&widths).map(|(cell, width)| format!("{cell:<width$}")).collect::<Vec<_>>().join("  ").trim_end()))
        .collect()
}

/// The whole dump as text.
pub fn text(report: &DumpReport) -> String {
    let mut lines = vec![
        format!("skin dump of {}  (seed {}, scene clock +{} us per frame)", report.target, report.seed, report.frame_micros),
        "frames call every boolean, integer, float, text and timer function once, whatever its draw conditions say: an upper bound of a real frame".to_owned(),
        String::new(),
    ];
    for document in &report.documents {
        document_lines(document, &mut lines);
    }
    lines.push("summary".to_owned());
    let mut rows = vec![SUMMARY_HEADER.iter().map(|cell| (*cell).to_owned()).collect::<Vec<_>>()];
    rows.extend(report.documents.iter().map(summary_row));
    lines.extend(table(&rows));
    lines.push(String::new());
    let pack = &report.pack;
    lines.push(format!(
        "pack folder {}: {} files before, {} after, {}",
        pack.folder,
        pack.files_before,
        pack.files_after,
        if pack.unchanged() {
            "unchanged".to_owned()
        } else {
            format!("CHANGED: {}", pack.changed.iter().take(ENTRIES_SHOWN).cloned().collect::<Vec<_>>().join(", "))
        }
    ));
    lines.join("\n")
}
