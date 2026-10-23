//! The printed report of `mirror-pack`.

use std::fmt::Display;

use super::{MirrorPackArgs, MirrorPackReport};

const BYTES_PER_GIB: f64 = 1024.0 * 1024.0 * 1024.0;

fn row(label: &str, value: impl Display) -> String {
    format!("{label:<14}: {value}")
}

fn size(bytes: u64) -> String {
    format!("{bytes} bytes ({:.2} GiB)", bytes as f64 / BYTES_PER_GIB)
}

/// The report as lines: what was found, chosen and packed, what was refused or could not be read
/// and why, and for each table the md5s that no song folder holds.
pub fn text(args: &MirrorPackArgs, report: &MirrorPackReport) -> String {
    let mut lines = vec![row("library", args.library.display()), row("out", args.out.display())];
    lines.push(row("song folders", report.folders_found));
    lines.push(row("selected", report.folders_selected));
    if args.dry_run {
        lines.push(row("dry run", "nothing is written"));
        lines.push(row("would pack", report.folders_selected - report.refused.len()));
    } else {
        lines.push(row("packs", format!("{} (written {}, reused {})", report.written + report.reused, report.written, report.reused)));
    }
    lines.push(row("refused", report.refused.len()));
    lines.extend(report.refused.iter().map(|(folder, refusal)| format!("  {folder}: {} {}", refusal.code(), refusal.detail())));
    lines.push(row("failed", report.failures.len()));
    lines.extend(report.failures.iter().map(|failure| format!("  {}: {}", failure.path.display(), failure.reason)));
    lines.push(row("files", report.total_entries));
    lines.push(row("not unpacked", format!("{} (kinds the player's unpacker leaves out)", report.skipped_by_client)));
    lines.push(row("unpacked size", size(report.total_uncompressed_bytes)));
    if !args.dry_run {
        lines.push(row("packs size", size(report.total_bytes)));
    }
    lines.push(row("duplicates", report.duplicates));
    for table in &report.tables {
        lines.push(row("table", &table.source));
        lines.push(row("table md5s", table.md5s));
        lines.push(row("matched", table.matched));
        lines.push(row("missing", table.missing.len()));
        lines.extend(table.missing.iter().map(|md5| format!("  {md5}")));
    }
    lines.join("\n")
}
