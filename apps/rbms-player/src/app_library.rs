//! `AppShared` wiring for the song library's *sources*: adding a difficulty table also matches it
//! against the library right away, so the browse view can show its levels without a rescan.
//!
//! The two management screens that edit these lists live in `stage::tables` and `stage::folders`.
#![allow(clippy::wildcard_imports)]
use crate::*;

impl AppShared {
    /// Add a table source and match it against the library, so its levels are browsable at once.
    pub(crate) fn add_table_source(&mut self, src: TableSource) {
        let (name, levels) = load_and_match(&src, &self.library);
        self.config.library.tables.push(src);
        self.table_names.push(name);
        self.table_levels.push(levels);
        self.save_settings();
    }
}
