use std::{env, fs};

fn main() {
    let path = match env::args().nth(1) {
        Some(p) => p,
        None => {
            eprintln!("usage: rbms-cli <chart.bms|.bme|.bml|.pms>");
            std::process::exit(2);
        }
    };

    let bytes = match fs::read(&path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("read error: {e}");
            std::process::exit(1);
        }
    };

    let src = rbms_parser::parse(&bytes);
    let mode = rbms_chart::detect_mode(&src, &path);
    let model = rbms_chart::to_model(&src, mode);
    let length_s = model.timelines.last().map(|t| t.time_us).unwrap_or(0) as f64 / 1_000_000.0;

    println!("file       : {path}");
    println!("title      : {}", model.meta.title);
    println!("artist     : {}", model.meta.artist);
    println!("genre      : {}", model.meta.genre);
    println!("level      : {}", model.meta.play_level);
    println!("init bpm   : {}", model.init_bpm);
    println!("md5        : {}", model.md5);
    println!("sha256     : {}", model.sha256);
    println!("wav defs   : {}", src.wav.len());
    println!("measures   : {}", src.measures.len());
    println!("timelines  : {}", model.timelines.len());
    println!("mode       : {}", mode.name);
    println!("notes      : {}", rbms_chart::count_playable_notes(&model));
    println!("length     : {length_s:.1}s");
}
