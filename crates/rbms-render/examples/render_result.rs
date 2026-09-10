use std::io::Write;

use rbms_render::{Color, CpuCanvas, ResultView, render_result};

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "/tmp/rbms_result.ppm".into());
    let (w, h) = (1280u32, 720u32);
    let mut canvas = CpuCanvas::new(w, h);
    let view = ResultView {
        title: "AltMirrorBell (moon)".into(),
        mode_label: "7K",
        counts: [712, 64, 21, 8, 5, 2],
        ex_score: 1488,
        max_score: 1624,
        max_combo: 540,
        total_notes: 812,
        fast: [30, 0],
        slow: [40, 0],
        gauge: 86.0,
        clear_label: "CLEAR",
        clear_color: Color::BLUE,
        prev_best_ex: Some(1450),
        prev_ex: Some(1402),
        show_graph: true,
        show_result_graphs: true,
        gauge_series: Vec::new(),
        timing_hist: Box::new([]),
        judge_dist: [0; 6],
    };
    render_result(&mut canvas, &view);

    let mut f = std::fs::File::create(&out).unwrap();
    write!(f, "P6\n{w} {h}\n255\n").unwrap();
    let mut rgb = Vec::with_capacity((w * h * 3) as usize);
    for chunk in canvas.pixels().chunks_exact(4) {
        rgb.extend_from_slice(&chunk[..3]);
    }
    f.write_all(&rgb).unwrap();
    println!("wrote {out}");
}
