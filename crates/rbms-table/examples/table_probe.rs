use rbms_table::DifficultyTable;

fn main() {
    let url = std::env::args().nth(1).expect("usage: table_probe <page-header-or-data-url> [file of library md5s, one a line]");
    let table = match DifficultyTable::fetch(&url) {
        Ok(table) => table,
        Err(error) => {
            println!("failed : {error}");
            return;
        }
    };
    println!("name   : {:?}", table.name);
    println!("symbol : {:?}", table.symbol);
    println!("entries: {} ({} skipped)", table.entries.len(), table.skipped);
    println!("levels : {} {:?}", table.by_level().len(), table.level_order);
    for (level, idxs) in table.by_level().iter().take(5) {
        println!("  level {level}: {} charts (e.g. {:?})", idxs.len(), table.entries[idxs[0]].title);
    }
    let Some(md5_file) = std::env::args().nth(2) else {
        return;
    };
    let md5s = std::fs::read_to_string(&md5_file).expect("read the md5 list");
    let owned = table.match_levels(md5s.lines().map(str::trim));
    let charts: usize = owned.iter().map(|(_, charts)| charts.len()).sum();
    println!("owned  : {charts} charts in {} of {} level folders", owned.iter().filter(|(_, charts)| !charts.is_empty()).count(), owned.len());
    println!("         {}", owned.iter().map(|(level, charts)| format!("{level}:{}", charts.len())).collect::<Vec<_>>().join(" "));
}
