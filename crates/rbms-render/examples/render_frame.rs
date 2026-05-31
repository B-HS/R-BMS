use std::io::Write;

use rbms_chart::{detect_mode, to_model};
use rbms_model::NoteKind;
use rbms_render::{CpuCanvas, HudView, Skin, SkinConfig, render_hud, render_key_bomb, render_playfield};

fn main() {
    let path = std::env::args().nth(1).expect("usage: render_frame <chart> [out.ppm] [--sc-left] [--lift F]");
    let args: Vec<String> = std::env::args().collect();
    let out = std::env::args().nth(2).filter(|s| !s.starts_with("--")).unwrap_or_else(|| "/tmp/rbms_frame.ppm".into());
    let sc_left = args.iter().any(|a| a == "--sc-left");
    let lift = args.iter().position(|a| a == "--lift").and_then(|i| args.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(0.0f32);

    let bytes = std::fs::read(&path).expect("read");
    let src = rbms_parser::parse_with(&bytes, Default::default());
    let mode = detect_mode(&src, &path);
    let model = to_model(&src, mode);

    let mut note_times: Vec<i64> = model
        .timelines
        .iter()
        .filter(|tl| tl.notes.iter().flatten().any(|n| !matches!(n.kind, NoteKind::LongEnd { .. })))
        .map(|tl| tl.time_us)
        .collect();
    note_times.sort_unstable();
    let pick = note_times.get(note_times.len() / 3).copied().unwrap_or(0);
    let microtime = pick - 350_000;

    let (w, h) = (1280u32, 720u32);
    let mut cfg = SkinConfig::default();
    cfg.scratch_left = sc_left;
    cfg.lift = lift;
    let skin = Skin::build(&cfg, mode, w as f32, h as f32);
    let mut canvas = CpuCanvas::new(w, h);
    let mut beam_on = vec![i64::MIN; mode.key];
    for lane in [0usize, 2, mode.key.saturating_sub(1)] {
        if lane < beam_on.len() {
            beam_on[lane] = microtime;
        }
    }
    render_playfield(&mut canvas, &model.timelines, microtime, 1.5, &skin, &beam_on, &[], false);
    // Three key bombs at different ages (0/35/70 ms) and judgments (PG/GR/GD) to show the burst.
    let mut bomb = vec![(i64::MIN, 0u8); mode.key];
    for (k, lane) in [0usize, 2, mode.key.saturating_sub(1)].into_iter().enumerate() {
        if lane < bomb.len() {
            bomb[lane] = (microtime - k as i64 * 35_000, k as u8);
        }
    }
    render_key_bomb(&mut canvas, &skin, &bomb, microtime);
    let hud = HudView { combo: 123, last_judge: Some(0), last_fast: false, fast: 30, slow: 40, counts: [712, 64, 21, 8, 5, 2], ex_score: 1488, gauge: 78.0, green_number: 310.0, max_ex: 1624, best_ex: Some(1502) };
    render_hud(&mut canvas, &skin, &hud);

    let mut f = std::fs::File::create(&out).expect("create");
    write!(f, "P6\n{w} {h}\n255\n").unwrap();
    let px = canvas.pixels();
    let mut rgb = Vec::with_capacity((w * h * 3) as usize);
    for chunk in px.chunks_exact(4) {
        rgb.extend_from_slice(&chunk[..3]);
    }
    f.write_all(&rgb).unwrap();

    let visible = rbms_chart::scroll::visible_offsets(&model.timelines, microtime, 1.5, skin.lane_height()).len();
    println!("title    : {}", model.meta.title);
    println!("microtime: {:.2}s  hispeed 1.5", microtime as f64 / 1e6);
    println!("visible timelines: {visible}");
    println!("wrote {out} ({w}x{h})");
}
