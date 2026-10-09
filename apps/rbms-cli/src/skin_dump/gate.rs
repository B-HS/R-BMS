//! What a strict dump fails on.
//!
//! A dump that is only read reports a document that cannot load and carries on. A dump used as a
//! gate has to fail instead, and has to know which failures are expected: a pack written for another
//! program may ship a document its own author left broken. So a strict dump is told the documents
//! that are allowed not to load and holds every document to exactly that, in both directions.

use super::report::{DocumentReport, DumpReport};

/// Why one document is not clean, or nothing when it is.
fn document_findings(document: &DocumentReport, may_fail: bool, findings: &mut Vec<String>) {
    let file = &document.file;
    let Some(body) = &document.body else {
        if !may_fail {
            let reason = document.load_error.as_deref().or(document.header_error.as_deref()).and_then(|error| error.lines().next()).unwrap_or_default();
            findings.push(format!("{file} did not load: {reason}"));
        }
        return;
    };
    if may_fail {
        findings.push(format!("{file} was allowed to fail and loaded"));
    }
    if let Some(error) = &document.header_error {
        findings.push(format!("{file}: the header pass failed: {}", error.lines().next().unwrap_or_default()));
    }
    let diagnostics = &body.diagnostics;
    let swallowed = diagnostics.swallowed.iter().map(|entry| entry.count).sum::<u64>() + diagnostics.swallowed_overflow;
    if swallowed > 0 {
        findings.push(format!("{file}: {swallowed} error(s) were caught by the skin's own pcall while it loaded"));
    }
    if !diagnostics.function_failures.is_empty() {
        findings.push(format!("{file}: {} function value(s) failed while it loaded", diagnostics.function_failures.len()));
    }
    if let Some(error) = &document.frames_error {
        findings.push(format!("{file}: the frames did not run: {error}"));
    }
    let Some(frames) = &document.frames else {
        return;
    };
    if !frames.function_failures.is_empty() {
        findings.push(format!("{file}: {} function value(s) failed during the frames", frames.function_failures.len()));
    }
    if frames.swallowed_during_frames > 0 {
        findings.push(format!("{file}: {} error(s) were caught by the skin's own pcall during the frames", frames.swallowed_during_frames));
    }
    if frames.frames_over_budget > 0 {
        findings.push(format!("{file}: the budget cut a call off in {} frame(s)", frames.frames_over_budget));
    }
}

/// Everything a strict dump fails on, one line each: a document that did not load and was not
/// allowed to, one that was allowed to and loaded, an allowance that names no document, an error a
/// skin caught in its own `pcall`, a function value that failed, and a frame the budget cut short.
pub fn findings(report: &DumpReport, allowed_failures: &[String]) -> Vec<String> {
    let mut findings = Vec::new();
    for allowed in allowed_failures {
        if !report.documents.iter().any(|document| document.file == *allowed) {
            findings.push(format!("{allowed} was allowed to fail and is not one of the documents dumped"));
        }
    }
    for document in &report.documents {
        document_findings(document, allowed_failures.contains(&document.file), &mut findings);
    }
    findings
}
