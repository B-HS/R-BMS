//! Fetch one difficulty table by hand and say what it is and where the time went.
//!
//! `table_probe <page-header-or-data-url> [library]`, where `library` is a file of library md5s, one
//! a line, or a number: a library of that many charts made up for the run, which owns every
//! [`OWNED_EVERY`]th chart of the table. The three steps a load is made of are timed apart: the
//! fetch (the network and the parse of what it answered), reading the table back from the copy
//! that is kept of it, and matching it against the library.

use std::time::Instant;

use rbms_table::{DifficultyTable, Md5Index, SavedTable};

/// Of a made-up library, which of the table's charts it owns: every this-many-th.
const OWNED_EVERY: usize = 10;

/// How many times the steps that take no network are repeated, so the time printed is not the time
/// of one cold run.
const REPEATS: u32 = 20;

fn made_up_library(table: &DifficultyTable, charts: usize) -> Vec<String> {
    let owned = table.entries.iter().step_by(OWNED_EVERY).map(|entry| entry.md5.clone());
    let others = (0..charts).map(|at| format!("{at:032x}"));
    owned.chain(others).take(charts).collect()
}

fn main() {
    let url = std::env::args().nth(1).expect("usage: table_probe <page-header-or-data-url> [file of library md5s, one a line | number of charts]");
    let started = Instant::now();
    let table = match DifficultyTable::fetch(&url) {
        Ok(table) => table,
        Err(error) => {
            println!("failed after {:?}: {error}", started.elapsed());
            return;
        }
    };
    println!("fetch  : {:?} (every request of the table, and the parse of what they answered)", started.elapsed());
    println!("name   : {:?}", table.name);
    println!("symbol : {:?}", table.symbol);
    println!("entries: {} ({} skipped)", table.entries.len(), table.skipped);
    println!("levels : {} {:?}", table.by_level().len(), table.level_order);
    println!("courses: {}", if table.course.is_some() { "listed" } else { "none" });
    for (level, idxs) in table.by_level().iter().take(5) {
        println!("  level {level}: {} charts (e.g. {:?})", idxs.len(), table.entries[idxs[0]].title);
    }

    let copy = SavedTable::json_of(&table, &url, None).expect("a table is JSON");
    let started = Instant::now();
    for _ in 0..REPEATS {
        SavedTable::from_bytes(copy.as_bytes()).expect("what was written is a copy");
    }
    println!("copy   : {} bytes, read back in {:?}", copy.len(), started.elapsed() / REPEATS);

    let Some(library) = std::env::args().nth(2) else {
        return;
    };
    let md5s: Vec<String> = match library.parse::<usize>() {
        Ok(charts) => made_up_library(&table, charts),
        Err(_) => std::fs::read_to_string(&library).expect("read the md5 list").lines().map(|line| line.trim().to_string()).collect(),
    };
    let started = Instant::now();
    for _ in 0..REPEATS {
        Md5Index::new(md5s.iter().map(String::as_str));
    }
    println!("index  : {} library charts indexed in {:?}", md5s.len(), started.elapsed() / REPEATS);
    let index = Md5Index::new(md5s.iter().map(String::as_str));
    let started = Instant::now();
    for _ in 0..REPEATS {
        table.match_levels_in(&index);
    }
    println!("match  : {:?} against the index", started.elapsed() / REPEATS);

    let owned = table.match_levels_in(&index);
    let charts: usize = owned.iter().map(|(_, charts)| charts.len()).sum();
    println!("owned  : {charts} charts in {} of {} level folders", owned.iter().filter(|(_, charts)| !charts.is_empty()).count(), owned.len());
    println!("         {}", owned.iter().map(|(level, charts)| format!("{level}:{}", charts.len())).collect::<Vec<_>>().join(" "));
}
