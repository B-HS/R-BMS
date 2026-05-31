use std::io::Write;

use rbms_render::{Color, CpuCanvas, Renderer, draw_text, draw_text_centered, text_width};

/// Renders multilingual UI text to a PPM so font coverage/look can be eyeballed headlessly
/// (no GUI/window needed). Usage: `cargo run --example text_render -p rbms-render -- out.ppm`.
fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "/tmp/rbms_text.ppm".into());
    let (w, h) = (1280u32, 560u32);
    let mut c = CpuCanvas::new(w, h);
    c.clear(Color::rgb(12, 14, 22));

    let lines: &[(&str, f32, Color)] = &[
        ("ENGLISH  abcdefg  0123456789", 2.4, Color::WHITE),
        ("日本語 東方Project 黒うさ feat.", 2.4, Color::rgb(150, 220, 255)),
        ("한국어 가나다 발광 난이도표 ★★", 2.4, Color::rgb(150, 255, 180)),
        ("中文 简体 繁體 节奏游戏", 2.4, Color::rgb(255, 220, 150)),
        ("Кириллица  Ελληνικά  Tiếng Việt", 2.0, Color::rgb(230, 180, 255)),
        ("ภาษาไทย  العربية  देवनागरी", 2.0, Color::rgb(255, 180, 180)),
        ("mixed: SunnyShinyRing -Another- ☆7", 1.8, Color::rgb(200, 200, 215)),
        ("symbols  ()[]+-/%:.!?#*  ←→↑↓", 1.6, Color::GRAY),
    ];
    let mut y = 28.0;
    for (s, scale, col) in lines {
        draw_text(&mut c, 40.0, y, *scale, *col, s);
        y += scale * 8.5 * 1.7 + 14.0;
    }
    draw_text_centered(&mut c, w as f32 * 0.5, h as f32 - 40.0, 1.4, Color::rgb(120, 240, 140), &format!("text_width('東方')={:.0}px  cosmic-text + Inter + system fallback", text_width("東方", 2.4)));

    let mut f = std::fs::File::create(&out).expect("create");
    write!(f, "P6\n{w} {h}\n255\n").unwrap();
    let mut rgb = Vec::with_capacity((w * h * 3) as usize);
    for chunk in c.pixels().chunks_exact(4) {
        rgb.extend_from_slice(&chunk[..3]);
    }
    f.write_all(&rgb).unwrap();
    println!("wrote {out} ({w}x{h})");
}
