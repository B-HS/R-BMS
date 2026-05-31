use rbms_table::DifficultyTable;

fn main() {
    let url = std::env::args().nth(1).expect("usage: table_probe <header-or-data-url>");
    let table = DifficultyTable::fetch(&url).expect("fetch table");
    println!("name   : {:?}", table.name);
    println!("symbol : {:?}", table.symbol);
    println!("entries: {}", table.entries.len());
    println!("levels : {:?}", table.level_order);
    for (level, idxs) in table.by_level().iter().take(5) {
        println!("  level {level}: {} charts (e.g. {:?})", idxs.len(), table.entries[idxs[0]].title);
    }
}
