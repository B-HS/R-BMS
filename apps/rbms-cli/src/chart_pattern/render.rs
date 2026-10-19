//! The printed report of `chart-pattern`.

use std::fmt::Display;

use super::{PatternArgs, PatternReport};

fn row(label: &str, value: impl Display) -> String {
    format!("{label:<14}: {value}")
}

/// The report as lines: what was read and written, the files that could not be read, and with a
/// table the md5s it lists that no chart of the mode was found for.
pub fn text(args: &PatternArgs, report: &PatternReport) -> String {
    let mut lines = vec![
        row("library", args.library.display()),
        row("out", args.out.display()),
        row("mode", report.mode.name),
        row("charts", report.found),
        row("patterns", format!("{} (written {}, unchanged {})", report.written + report.unchanged, report.written, report.unchanged)),
        row("duplicates", report.duplicates),
        row("other mode", report.other_mode),
        row("failed", report.failures.len()),
    ];
    lines.extend(report.failures.iter().map(|failure| format!("  {}: {}", failure.path.display(), failure.reason)));
    if let (Some(table), Some(coverage)) = (&args.table, &report.coverage) {
        lines.push(row("table", table.display()));
        lines.push(row("table md5s", coverage.table_md5s));
        lines.push(row("no md5", coverage.table_entries_without_md5));
        lines.push(row("matched", coverage.matched));
        lines.push(row("missing", coverage.missing.len()));
        lines.extend(coverage.missing.iter().map(|md5| format!("  {md5}")));
        lines.push(row("in other mode", coverage.other_mode.len()));
        lines.extend(coverage.other_mode.iter().map(|md5| format!("  {md5}")));
    }
    lines.join("\n")
}
