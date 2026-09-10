use std::io::Write;

use rbms_render::{
    Color, CoverState, CpuCanvas, DensityView, DetailView, RecordRowView, RecordsView, SelectDetail, SelectModal, SelectRow, SelectView, StatCell,
    render_select,
};

/// A song-bar row's mode/level badge pair, so `row` stays a short call.
struct RowBadges {
    mode: &'static str,
    mode_color: Color,
    level: &'static str,
    difficulty_color: Color,
}

fn row(folder: bool, title: &str, badges: RowBadges, lamp: Color, count: Option<usize>) -> SelectRow {
    SelectRow {
        folder,
        title: title.into(),
        mode_short: badges.mode,
        mode_color: badges.mode_color,
        level: badges.level.into(),
        difficulty_color: badges.difficulty_color,
        lamp,
        folder_count: count,
        dj_level: (!folder).then_some("AA"),
        favorite: !folder,
    }
}

fn rec(when: &str, lamp: Color, label: &'static str, ex: u32, max: u32, bp: u32, trend: Option<(String, Color)>) -> RecordRowView {
    RecordRowView { when: when.into(), lamp, lamp_label: label, ex, max_ex: max, bp, trend }
}

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "/tmp/rbms_select.ppm".into());
    let blue = Color::BLUE;
    let red = Color::RED;
    let green = Color::GREEN;
    let pink = Color::rgb(230, 120, 200);
    let normal = Color::rgb(0, 192, 240);
    let hard = Color::WHITE;
    let nolamp = Color::rgb(44, 44, 54);

    let n_bins = if std::env::args().any(|a| a == "long") { 900 } else { 130 };
    let half = n_bins as f64 / 2.0;
    let bins: Vec<u32> = (0..n_bins)
        .map(|i| {
            let base = (8.0 * (1.0 - ((i as f64 - half) / half).powi(2))).max(0.0);
            let spike = if i % 23 == 0 { 9.0 } else { 0.0 };
            (base + spike) as u32
        })
        .collect();

    let detail = DetailView {
        accent: blue,
        title: "Legend of Eastern Rabbit -SKY DEFENDER- [7K ANOTHER]".into(),
        subtitle: "東方兎傳説".into(),
        artist: "ost1 Visualized by raw_png".into(),
        genre_maker: "Oriental Rock  ·  notecharter".into(),
        mode_short: "7K",
        mode_color: blue,
        level: "12".into(),
        difficulty_color: red,
        difficulty_name: "ANOTHER",
        cover: CoverState::None,
        stats: vec![
            StatCell { label: "BPM", value: "145–190".into() },
            StatCell { label: "DENSITY", value: "9/s".into() },
            StatCell { label: "NOTES", value: "1357 (48LN)".into() },
            StatCell { label: "JUDGE", value: "NORMAL 75%".into() },
            StatCell { label: "LENGTH", value: "2:11".into() },
            StatCell { label: "TOTAL", value: "320".into() },
        ],
        density: Some(DensityView { bins, peak: 17.0, avg: 9.0, end: 11.0 }),
        records: RecordsView {
            plays: 12,
            clears: 8,
            best: Some(rec("2026-05-31 01:02", hard, "HARD", 1602, 1848, 12, None)),
            rank_bar: Some((1602, 1848)),
            recent: vec![
                rec("2026-05-31 01:02", hard, "HARD", 1602, 1848, 12, Some(("+59".into(), green))),
                rec("2026-05-30 22:14", normal, "CLEAR", 1543, 1848, 41, Some(("-7".into(), red))),
                rec("2026-05-29 19:40", red, "FAILED", 1490, 1848, 88, None),
            ],
        },
    };

    let badges = |mode, mode_color, level, difficulty_color| RowBadges { mode, mode_color, level, difficulty_color };
    let rows = vec![
        row(true, "INSANE BMS DIFFICULTY TABLE", badges("", Color::GRAY, "", Color::GRAY), nolamp, Some(86)),
        row(false, "Legend of Eastern Rabbit -SKY DEFENDER-", badges("7K", blue, "7", red), green, None),
        row(false, "Another Song With A Fairly Long Title Indeed", badges("5K", green, "5", normal), normal, None),
        row(false, "9 Button Madness", badges("9K", pink, "9", blue), nolamp, None),
        row(false, "DP Chart Sample -DOUBLE-", badges("14K", Color::ORANGE, "11", red), hard, None),
        row(false, "Legend of Eastern Rabbit -SKY DEFENDER- [ANOTHER]", badges("7K", blue, "12", red), green, None),
        row(false, "Short", badges("7K", blue, "3", green), nolamp, None),
        row(false, "Mid Tier Track", badges("7K", blue, "8", Color::rgb(240, 200, 70)), normal, None),
        row(false, "Hardest In The Pack", badges("7K", blue, "★12", pink), red, None),
    ];

    let modal = std::env::args().any(|a| a == "modal").then(|| SelectModal {
        title: "Legend of Eastern Rabbit -SKY DEFENDER- [7K ANOTHER]".into(),
        clear_label: "HARD",
        clear_color: Color::WHITE,
        when: "2026-05-31 01:02".into(),
        sub: "7K   RANDOM   GAUGE HARD".into(),
        counts: [1280, 64, 21, 8, 5, 2],
        ex: 1602,
        max_ex: 1848,
        rank: Some("AA"),
        rank_color: Color::rgb(210, 210, 235),
        max_combo: 612,
        total_notes: 1357,
        bp: 12,
        empty_poor: 3,
        gauge_value: 82,
        index: 0,
        total: 3,
        has_replay: true,
    });

    let view = SelectView {
        rows,
        sel: 5,
        header: "ROOT / INSANE TABLE / LEVEL 12".into(),
        guide: "UP DOWN  LEFT BACK  RIGHT/ENTER OPEN  TAB SETTINGS  O FOLDER  T TABLES  R RECORDS",
        detail: SelectDetail::Song(Box::new(detail)),
        modal,
        score_graph: true,
        search: None,
        sort: "DEFAULT",
        filter: None,
        empty_hint: None,
    };

    let (w, h) = (1280u32, 720u32);
    let mut canvas = CpuCanvas::new(w, h);
    let hot = render_select(&mut canvas, &view);

    let mut f = std::fs::File::create(&out).expect("create");
    write!(f, "P6\n{w} {h}\n255\n").unwrap();
    let mut rgb = Vec::with_capacity((w * h * 3) as usize);
    for chunk in canvas.pixels().chunks_exact(4) {
        rgb.extend_from_slice(&chunk[..3]);
    }
    f.write_all(&rgb).unwrap();
    println!("wrote {out} ({w}x{h}); {} hot regions", hot.len());
}
