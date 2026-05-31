use rbms_chart::{count_playable_notes, detect_mode, to_model};
use rbms_play::simulate_autoplay;

fn main() {
    let path = std::env::args().nth(1).expect("usage: autoplay_score <chart>");
    let bytes = std::fs::read(&path).unwrap();
    let src = rbms_parser::parse_with(&bytes, Default::default());
    let mode = detect_mode(&src, &path);
    let model = to_model(&src, mode);
    let n = count_playable_notes(&model);
    let e = simulate_autoplay(&model);
    println!("title    : {} [{}]", model.meta.title, mode.name);
    println!("notes    : {n}");
    println!("PG/GR/GD/BD/POOR/MISS: {:?}", e.counts);
    println!("ex_score : {} / {} (max)", e.ex_score, 2 * n);
    println!("max_combo: {} / {}", e.max_combo, n);
    println!("gauge    : {:.1}% (NORMAL)", e.gauge.value());
    println!("clear    : {:?}", e.clear_lamp());
    let perfect = e.counts[0] == n as u32 && e.counts[5] == 0 && e.max_combo == n as u32;
    println!("RESULT   : {}", if perfect { "PERFECT" } else { "MISMATCH" });
}
