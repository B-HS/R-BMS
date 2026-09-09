use rbms_chart::scroll::{constant_offsets, visible_offsets};
use rbms_model::{NoteKind, TimeLine};

use crate::skin::Skin;
use crate::{Color, Rect, Renderer};

/// Release-fade length of a key beam after a lane is let go (µs). Mirrors a skin's keyoff
/// animation: the beam fades from full to invisible over this span.
const BEAM_RELEASE_US: i64 = 120_000;

/// Vertical gradient slices per beam — more slices = smoother brightest-at-the-judge-line falloff.
const BEAM_SLICES: usize = 6;

/// Everything a playfield frame needs beyond the skin: the chart, where in it we are, and the
/// per-lane beam state. `beam_on`/`beam_off` are press/release timestamps (µs, `i64::MIN` when
/// inactive); pass `&[]` to omit beams.
pub struct PlayfieldView<'a> {
    pub timelines: &'a [TimeLine],
    pub microtime: i64,
    pub hispeed: f64,
    pub beam_on: &'a [i64],
    pub beam_off: &'a [i64],
    /// Constant-velocity scrolling (ignore per-section BPM/STOP), i.e. the CONSTANT option.
    pub constant: bool,
}

/// Draw the playfield for the current play time using a `Skin`: lane backgrounds, key beams
/// for held/just-pressed lanes, the judgment line, and every visible note at its scroll
/// offset. Backend-agnostic.
pub fn render_playfield_view<R: Renderer>(r: &mut R, skin: &Skin, view: &PlayfieldView<'_>) {
    let PlayfieldView { timelines, microtime, hispeed, beam_on, beam_off, constant } = *view;
    r.clear(skin.bg);

    let n = skin.lane_count();
    let lane_h = skin.judge_y - skin.top_y;
    for lane in 0..n {
        r.fill_rect(Rect::new(skin.x[lane], skin.top_y, skin.w[lane], lane_h), skin.lane_bg);
    }

    draw_key_beams(r, skin, beam_on, beam_off, microtime);
    draw_field_decor(r, skin);

    for &(fx, fw) in &skin.fields {
        r.fill_rect(Rect::new(fx, skin.judge_y, fw, 4.0), skin.judge_line);
    }

    let offsets = if constant {
        constant_offsets(timelines, microtime, hispeed, skin.lane_height())
    } else {
        visible_offsets(timelines, microtime, hispeed, skin.lane_height())
    };

    let pos: std::collections::HashMap<usize, f32> = offsets.iter().copied().collect();
    let last_visible = offsets.last().map(|&(i, _)| i).unwrap_or(0);
    let mut ln_open: Vec<Option<usize>> = vec![None; n];
    for (idx, tl) in timelines.iter().enumerate() {
        for (lane, open_head) in ln_open.iter_mut().enumerate() {
            let Some(note) = &tl.notes[lane] else {
                continue;
            };
            match note.kind {
                NoteKind::LongStart { .. } => *open_head = Some(idx),
                NoteKind::LongEnd { .. } => {
                    let Some(head_idx) = open_head.take() else {
                        continue;
                    };
                    if tl.time_us < microtime {
                        continue;
                    }
                    let off_head = match pos.get(&head_idx) {
                        Some(&o) => o,
                        None if timelines[head_idx].time_us <= microtime => 0.0,
                        None => continue,
                    };
                    let body_bottom = (skin.judge_y - off_head).min(skin.judge_y);
                    let body_top = pos.get(&idx).map(|o| skin.judge_y - o).unwrap_or(skin.top_y).max(skin.top_y);
                    if body_bottom > body_top {
                        let c = skin.note_color(lane);
                        let body = Color { r: c.r, g: c.g, b: c.b, a: 90 };
                        r.fill_rect(Rect::new(skin.x[lane], body_top, skin.w[lane], body_bottom - body_top), body);
                    }
                }
                _ => {}
            }
        }
        if idx >= last_visible && ln_open.iter().all(Option::is_none) {
            break;
        }
    }

    for (i, off) in offsets {
        let tl = &timelines[i];
        for lane in 0..n {
            let Some(note) = &tl.notes[lane] else {
                continue;
            };
            if matches!(note.kind, NoteKind::LongEnd { .. }) {
                continue;
            }
            let color = match note.kind {
                NoteKind::Mine { .. } => skin.mine_color,
                _ => skin.note_color(lane),
            };
            let note_top = skin.judge_y - off - skin.note_height;
            let top = note_top.max(skin.top_y);
            let h = (note_top + skin.note_height) - top;
            if h > 0.0 {
                r.fill_rect(Rect::new(skin.x[lane], top, skin.w[lane], h), color);
            }
        }
    }
}

/// Draw the per-lane key beams: a vertical gradient bar anchored at the judgment line and
/// rising up the lane, brightest at the bottom. Full while a lane is held, fading out over
/// `BEAM_RELEASE_US` after release.
fn draw_key_beams<R: Renderer>(r: &mut R, skin: &Skin, beam_on: &[i64], beam_off: &[i64], microtime: i64) {
    if skin.beam_alpha == 0 || skin.beam_height <= 0.0 {
        return;
    }
    let slice_h = skin.beam_height / BEAM_SLICES as f32;
    for lane in 0..skin.lane_count() {
        let on = beam_on.get(lane).copied().unwrap_or(i64::MIN);
        let off = beam_off.get(lane).copied().unwrap_or(i64::MIN);
        let intensity = beam_intensity(on, off, microtime);
        if intensity <= 0.0 {
            continue;
        }
        let base = skin.beam_alpha as f32 * intensity;
        for s in 0..BEAM_SLICES {
            let falloff = 1.0 - s as f32 / BEAM_SLICES as f32;
            let a = (base * falloff).round() as u8;
            if a == 0 {
                continue;
            }
            let y = skin.judge_y - (s as f32 + 1.0) * slice_h;
            let color = Color { r: skin.beam_color.r, g: skin.beam_color.g, b: skin.beam_color.b, a };
            r.fill_rect(Rect::new(skin.x[lane], y, skin.w[lane], slice_h), color);
        }
    }
}

/// Draw the field decoration: per-lane divider lines and a border outline around each field
/// (one for SP, two for a dual-layout DP mode). Each is skipped when its colour alpha is 0 (the
/// skin's on/off + opacity control). Lanes are uniform width, so each field holds `n / fields` of them.
fn draw_field_decor<R: Renderer>(r: &mut R, skin: &Skin) {
    let n = skin.lane_count();
    if n == 0 || skin.fields.is_empty() {
        return;
    }
    let lanes_per_field = (n / skin.fields.len()).max(1);
    let top = skin.top_y;
    let h = skin.judge_y - skin.top_y;
    for &(field_x0, field_w) in &skin.fields {
        let lane_w = field_w / lanes_per_field as f32;
        if skin.divider.a > 0 {
            for k in 1..lanes_per_field {
                let x = field_x0 + k as f32 * lane_w;
                r.fill_rect(Rect::new(x - 0.5, top, 1.0, h), skin.divider);
            }
        }
        if skin.outline.a > 0 {
            let t = 2.0;
            r.fill_rect(Rect::new(field_x0 - t, top - t, field_w + 2.0 * t, t), skin.outline);
            r.fill_rect(Rect::new(field_x0 - t, skin.judge_y, field_w + 2.0 * t, t), skin.outline);
            r.fill_rect(Rect::new(field_x0 - t, top - t, t, h + 2.0 * t), skin.outline);
            r.fill_rect(Rect::new(field_x0 + field_w, top - t, t, h + 2.0 * t), skin.outline);
        }
    }
}

/// Draw a lane cover (sudden+): an opaque rect over the top `cover_frac` of the field that
/// hides approaching notes until they emerge below it. Call AFTER `render_playfield` so it
/// occludes the notes, and BEFORE the HUD so combo/judgment stay on top.
pub fn render_lane_cover<R: Renderer>(r: &mut R, skin: &Skin, cover_frac: f32) {
    let f = cover_frac.clamp(0.0, 0.9);
    if f <= 0.0 {
        return;
    }
    let h = (skin.judge_y - skin.top_y) * f;
    for &(fx, fw) in &skin.fields {
        r.fill_rect(Rect::new(fx, skin.top_y, fw, h), skin.bg);
    }
}

/// Beam brightness in `0.0..=1.0`: full while held (`on` set), then a linear fade over
/// `BEAM_RELEASE_US` from the release timestamp (`off`).
fn beam_intensity(on: i64, off: i64, now: i64) -> f32 {
    if on != i64::MIN {
        return 1.0;
    }
    if off != i64::MIN {
        let age = now - off;
        if age < 0 {
            return 1.0;
        }
        if age < BEAM_RELEASE_US {
            return 1.0 - age as f32 / BEAM_RELEASE_US as f32;
        }
    }
    0.0
}

/// Draw the key bomb: for each lane whose most recent note hit is within the skin's bomb window, an
/// expanding judge-coloured burst with a shrinking white core at the judgment line. `bomb[lane]` is
/// `(hit_us, judge_index)` (`i64::MIN` = none). Data-driven via `Skin::bomb_*`; no-op when disabled.
pub fn render_key_bomb<R: Renderer>(r: &mut R, skin: &Skin, bomb: &[(i64, u8)], microtime: i64) {
    if !skin.bomb_enabled || skin.bomb_us <= 0 {
        return;
    }
    for (lane, &(hit_us, judge)) in bomb.iter().enumerate().take(skin.lane_count()) {
        if hit_us == i64::MIN {
            continue;
        }
        let age = microtime - hit_us;
        if age < 0 || age >= skin.bomb_us {
            continue;
        }
        let p = age as f32 / skin.bomb_us as f32;
        let cx = skin.x[lane] + skin.w[lane] * 0.5;
        let cy = skin.judge_y;
        let base = skin.bomb_size.min(skin.w[lane] * 1.7);
        let size = base * (0.45 + 0.75 * p);
        let jc = skin.judge_colors[(judge as usize).min(5)];
        r.fill_rect(Rect::new(cx - size * 0.5, cy - size * 0.5, size, size), Color { a: ((1.0 - p) * 200.0) as u8, ..jc });
        let core = base * (1.0 - p) * 0.55;
        if core > 1.0 {
            r.fill_rect(Rect::new(cx - core * 0.5, cy - core * 0.5, core, core), Color { r: 255, g: 255, b: 255, a: ((1.0 - p) * 235.0) as u8 });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skin::Skin;
    use crate::{Color, CpuCanvas};
    use rbms_chart::scroll::closed_form_offset;
    use rbms_chart::to_model;
    use rbms_model::Mode;
    use rbms_parser::parse;

    fn tls(bms: &[u8]) -> Vec<TimeLine> {
        to_model(&parse(bms), Mode::BEAT_7K).timelines
    }

    fn skin() -> Skin {
        Skin::default_for(Mode::BEAT_7K, 1280.0, 720.0)
    }

    /// One frame's playfield inputs, so a test reads as a call rather than a struct literal.
    fn view<'a>(timelines: &'a [TimeLine], microtime: i64, hispeed: f64, beam_on: &'a [i64], beam_off: &'a [i64], constant: bool) -> PlayfieldView<'a> {
        PlayfieldView { timelines, microtime, hispeed, beam_on, beam_off, constant }
    }

    #[test]
    fn note_drawn_at_expected_lane_and_offset() {
        let timelines = tls(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let skin = skin();
        let note_t = 2_000_000;
        let micro = note_t - 300_000;
        let mut canvas = CpuCanvas::new(1280, 720);
        render_playfield_view(&mut canvas, &skin, &view(&timelines, micro, 1.0, &[], &[], false));

        let off = closed_form_offset(note_t, micro, 120.0, 1.0, 1.0, skin.lane_height());
        let cx = skin.lane_center(0) as u32;
        let cy = (skin.judge_y - off - skin.note_height * 0.5) as u32;
        assert_eq!(canvas.pixel_at(cx, cy), Color::WHITE, "note should be white at lane 0");
        assert_ne!(canvas.pixel_at(skin.lane_center(3) as u32, cy), Color::WHITE, "lane 3 has no note here");
    }

    #[test]
    fn long_note_draws_a_held_body_bar() {
        let timelines = tls(b"#LNTYPE 1\r\n#BPM 120\r\n#WAV01 a.wav\r\n#00151:01000001\r\n");
        let skin = skin();
        let head = timelines.iter().filter_map(|t| t.notes[0].as_ref().map(|nn| nn.time_us)).next().unwrap();
        let micro = head - 200_000;
        let mut c = CpuCanvas::new(1280, 720);
        render_playfield_view(&mut c, &skin, &view(&timelines, micro, 1.0, &[], &[], false));
        let (cx0, cx3) = (skin.lane_center(0) as u32, skin.lane_center(3) as u32);
        let lane0_drawn = |canvas: &CpuCanvas| (skin.top_y as u32..skin.judge_y as u32).filter(|&y| canvas.pixel_at(cx0, y) != canvas.pixel_at(cx3, y)).count();
        let filled = lane0_drawn(&c);
        assert!(filled > skin.note_height as usize * 4, "LN body should fill many rows in its lane, not just a tap cap (got {filled}px)");
        let normal = tls(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let nt = normal.iter().find_map(|t| t.notes[0].as_ref().map(|nn| nn.time_us)).unwrap();
        let mut c2 = CpuCanvas::new(1280, 720);
        render_playfield_view(&mut c2, &skin, &view(&normal, nt - 200_000, 1.0, &[], &[], false));
        let tap_filled = lane0_drawn(&c2);
        assert!(filled > tap_filled * 2, "LN body (filled={filled}) must be much taller than a tap (filled={tap_filled})");
    }

    #[test]
    fn judgment_line_is_drawn() {
        let timelines = tls(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let skin = skin();
        let mut canvas = CpuCanvas::new(1280, 720);
        render_playfield_view(&mut canvas, &skin, &view(&timelines, 0, 1.0, &[], &[], false));
        let cx = skin.lane_center(0) as u32;
        assert_eq!(canvas.pixel_at(cx, skin.judge_y as u32 + 1), Color::JUDGE_LINE);
    }

    #[test]
    fn higher_hispeed_pushes_note_further_from_line() {
        let timelines = tls(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let skin = skin();
        let note_t = 2_000_000;
        let micro = note_t - 300_000;
        let cx = skin.lane_center(0) as u32;

        let find_note_y = |hispeed: f64| -> u32 {
            let mut c = CpuCanvas::new(1280, 720);
            render_playfield_view(&mut c, &skin, &view(&timelines, micro, hispeed, &[], &[], false));
            (0..skin.judge_y as u32).find(|&y| c.pixel_at(cx, y) == Color::WHITE).unwrap()
        };
        assert!(find_note_y(2.0) < find_note_y(1.0));
    }

    #[test]
    fn key_beam_brightens_held_lane_only() {
        let timelines = tls(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let skin = skin();
        let cx0 = skin.lane_center(0) as u32;
        let cx3 = skin.lane_center(3) as u32;
        let cy = skin.judge_y as u32 - 4;

        let mut off = CpuCanvas::new(1280, 720);
        render_playfield_view(&mut off, &skin, &view(&timelines, 0, 1.0, &[], &[], false));

        let mut on = CpuCanvas::new(1280, 720);
        render_playfield_view(&mut on, &skin, &view(&timelines, 0, 1.0, &[0], &[], false));

        assert_ne!(on.pixel_at(cx0, cy), off.pixel_at(cx0, cy), "held lane 0 beam brightens near the judge line");
        assert_eq!(on.pixel_at(cx3, cy), off.pixel_at(cx3, cy), "lane 3 with no beam is unchanged");
    }

    #[test]
    fn key_beam_fades_to_nothing_after_release() {
        assert_eq!(beam_intensity(i64::MIN, 0, BEAM_RELEASE_US + 1), 0.0, "fully faded past the release window");
        assert_eq!(beam_intensity(i64::MIN, 0, 0), 1.0, "full at the moment of release");
        assert_eq!(beam_intensity(5, i64::MIN, 9_999), 1.0, "held lane stays full");
    }

    #[test]
    fn beam_intensity_inactive_lane_is_zero() {
        assert_eq!(beam_intensity(i64::MIN, i64::MIN, 1000), 0.0, "never pressed -> no beam");
    }

    #[test]
    fn beam_intensity_held_overrides_release() {
        assert_eq!(beam_intensity(100, 0, 10_000_000), 1.0);
    }

    #[test]
    fn beam_intensity_before_release_timestamp_is_full() {
        assert_eq!(beam_intensity(i64::MIN, 1_000_000, 500_000), 1.0, "now before release -> full");
    }

    #[test]
    fn beam_intensity_decays_linearly_through_release_window() {
        let half = beam_intensity(i64::MIN, 0, BEAM_RELEASE_US / 2);
        assert!((half - 0.5).abs() < 1e-3, "linear fade at the midpoint (got {half})");
        let quarter = beam_intensity(i64::MIN, 0, BEAM_RELEASE_US / 4);
        assert!(quarter > half, "fade is monotonic decreasing with age");
        assert_eq!(beam_intensity(i64::MIN, 0, BEAM_RELEASE_US), 0.0, "exactly at the window end -> 0");
    }

    #[test]
    fn note_just_before_its_time_rests_at_the_judge_line() {
        let timelines = tls(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let skin = skin();
        let note_t = timelines.iter().find_map(|t| t.notes[0].as_ref().map(|n| n.time_us)).unwrap();
        let mut c = CpuCanvas::new(1280, 720);
        render_playfield_view(&mut c, &skin, &view(&timelines, note_t - 1_000, 1.0, &[], &[], false));
        let cx = skin.lane_center(0) as u32;
        let row = skin.judge_y as u32 - 1;
        assert_eq!(c.pixel_at(cx, row), Color::WHITE, "note bottom rests at the judge line");
        assert_eq!(c.pixel_at(cx, skin.judge_y as u32 + 1), Color::JUDGE_LINE);
    }

    #[test]
    fn note_exactly_at_its_time_is_excluded_from_offsets() {
        let timelines = tls(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let skin = skin();
        let note_t = timelines.iter().find_map(|t| t.notes[0].as_ref().map(|n| n.time_us)).unwrap();
        let mut c = CpuCanvas::new(1280, 720);
        render_playfield_view(&mut c, &skin, &view(&timelines, note_t, 1.0, &[], &[], false));
        let cx = skin.lane_center(0) as u32;
        let drawn = (skin.top_y as u32..skin.judge_y as u32).any(|y| c.pixel_at(cx, y) == Color::WHITE);
        assert!(!drawn, "the note at exactly its time is no longer in the visible set");
    }

    #[test]
    fn mine_note_uses_mine_color_not_key_color() {
        let timelines = tls(b"#BPM 120\r\n#WAV01 a.wav\r\n#001D1:01\r\n");
        let skin = skin();
        let has_mine = timelines.iter().any(|t| t.notes[0].as_ref().is_some_and(|n| matches!(n.kind, NoteKind::Mine { .. })));
        assert!(has_mine, "test data must contain a mine");
        let note_t = timelines.iter().find_map(|t| t.notes[0].as_ref().map(|n| n.time_us)).unwrap();
        let mut c = CpuCanvas::new(1280, 720);
        render_playfield_view(&mut c, &skin, &view(&timelines, note_t - 1_000, 1.0, &[], &[], false));
        let cx = skin.lane_center(0) as u32;
        let row = skin.judge_y as u32 - 1;
        assert_eq!(c.pixel_at(cx, row), skin.mine_color, "mine draws in mine_color");
        assert_ne!(skin.mine_color, skin.note_color(0), "mine colour differs from a tap on lane 0");
    }

    fn lane_drawn_rows(c: &CpuCanvas, skin: &Skin, lane: usize, ref_lane: usize) -> usize {
        let (cx, cref) = (skin.lane_center(lane) as u32, skin.lane_center(ref_lane) as u32);
        (skin.top_y as u32..skin.judge_y as u32).filter(|&y| c.pixel_at(cx, y) != c.pixel_at(cref, y)).count()
    }

    #[test]
    fn ln_straddling_the_line_draws_body_down_to_the_line() {
        let timelines = tls(b"#LNTYPE 1\r\n#BPM 120\r\n#WAV01 a.wav\r\n#00151:01000001\r\n");
        let skin = skin();
        let head = timelines.iter().find_map(|t| t.notes[0].as_ref().filter(|n| matches!(n.kind, NoteKind::LongStart { .. })).map(|n| n.time_us)).unwrap();
        let micro = head + 100_000;
        let mut c = CpuCanvas::new(1280, 720);
        render_playfield_view(&mut c, &skin, &view(&timelines, micro, 1.0, &[], &[], false));
        let cx = skin.lane_center(0) as u32;
        assert_ne!(
            c.pixel_at(cx, skin.judge_y as u32 - 2),
            c.pixel_at(skin.lane_center(3) as u32, skin.judge_y as u32 - 2),
            "LN body reaches the line while head is past it"
        );
        let filled = lane_drawn_rows(&c, &skin, 0, 3);
        assert!(filled > skin.note_height as usize * 3, "straddling LN still draws a tall body (got {filled}px)");
    }

    #[test]
    fn ln_with_end_above_window_extends_body_to_top_edge() {
        let timelines = tls(b"#LNTYPE 1\r\n#BPM 120\r\n#WAV01 a.wav\r\n#00151:01000001\r\n");
        let skin = skin();
        let head = timelines.iter().find_map(|t| t.notes[0].as_ref().filter(|n| matches!(n.kind, NoteKind::LongStart { .. })).map(|n| n.time_us)).unwrap();
        let micro = head - 50_000;
        let mut c = CpuCanvas::new(1280, 720);
        render_playfield_view(&mut c, &skin, &view(&timelines, micro, 4.0, &[], &[], false));
        let cx = skin.lane_center(0) as u32;
        assert_ne!(
            c.pixel_at(cx, skin.top_y as u32 + 1),
            c.pixel_at(skin.lane_center(3) as u32, skin.top_y as u32 + 1),
            "LN body fills up to the top edge when the end is above the window"
        );
    }

    #[test]
    fn off_screen_ln_whole_above_window_is_not_drawn() {
        let timelines = tls(b"#LNTYPE 1\r\n#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n#00351:0100000000000001\r\n");
        let skin = skin();
        let mut c = CpuCanvas::new(1280, 720);
        render_playfield_view(&mut c, &skin, &view(&timelines, 0, 8.0, &[], &[], false));
        let filled = lane_drawn_rows(&c, &skin, 0, 3);
        assert_eq!(filled, 0, "off-screen LN must not tint the lane (got {filled}px)");
    }

    #[test]
    fn ln_already_past_the_line_is_not_drawn() {
        let timelines = tls(b"#LNTYPE 1\r\n#BPM 120\r\n#WAV01 a.wav\r\n#00151:01000001\r\n");
        let skin = skin();
        let end = timelines.iter().find_map(|t| t.notes[0].as_ref().filter(|n| matches!(n.kind, NoteKind::LongEnd { .. })).map(|n| n.time_us)).unwrap();
        let mut c = CpuCanvas::new(1280, 720);
        render_playfield_view(&mut c, &skin, &view(&timelines, end + 500_000, 1.0, &[], &[], false));
        let filled = lane_drawn_rows(&c, &skin, 0, 3);
        assert_eq!(filled, 0, "an LN fully behind the line draws nothing in its lane (got {filled}px)");
    }

    #[test]
    fn long_end_cap_is_not_drawn_as_a_note() {
        let timelines = tls(b"#LNTYPE 1\r\n#BPM 120\r\n#WAV01 a.wav\r\n#00151:01000001\r\n");
        let skin = skin();
        let head = timelines.iter().find_map(|t| t.notes[0].as_ref().filter(|n| matches!(n.kind, NoteKind::LongStart { .. })).map(|n| n.time_us)).unwrap();
        let mut c = CpuCanvas::new(1280, 720);
        render_playfield_view(&mut c, &skin, &view(&timelines, head - 200_000, 1.0, &[], &[], false));
        let cx = skin.lane_center(0) as u32;
        let cx3 = skin.lane_center(3) as u32;
        let drawn: Vec<u32> = (skin.top_y as u32..skin.judge_y as u32).filter(|&y| c.pixel_at(cx, y) != c.pixel_at(cx3, y)).collect();
        assert!(!drawn.is_empty(), "LN draws something");
        let gaps = drawn.windows(2).filter(|w| w[1] - w[0] > 1).count();
        assert_eq!(gaps, 0, "the LN renders as one contiguous bar (no body/cap gap)");
    }

    #[test]
    fn lane_cover_occludes_top_fraction_of_field() {
        let timelines = tls(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let skin = skin();
        let mut c = CpuCanvas::new(1280, 720);
        render_playfield_view(&mut c, &skin, &view(&timelines, 0, 1.0, &[], &[], false));
        render_lane_cover(&mut c, &skin, 0.5);
        let cx = skin.lane_center(0) as u32;
        let cover_h = ((skin.judge_y - skin.top_y) * 0.5) as u32;
        assert_eq!(c.pixel_at(cx, skin.top_y as u32 + 2), skin.bg, "cover paints field bg over the top fraction");
        assert_ne!(c.pixel_at(cx, skin.top_y as u32 + cover_h + 4), skin.bg, "below the cover the lane bg shows through");
    }

    #[test]
    fn lane_cover_zero_fraction_is_noop() {
        let skin = skin();
        let mut a = CpuCanvas::new(1280, 720);
        let mut b = CpuCanvas::new(1280, 720);
        let timelines = tls(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        render_playfield_view(&mut a, &skin, &view(&timelines, 0, 1.0, &[], &[], false));
        render_playfield_view(&mut b, &skin, &view(&timelines, 0, 1.0, &[], &[], false));
        render_lane_cover(&mut b, &skin, 0.0);
        assert_eq!(a.pixels(), b.pixels(), "cover_frac 0 leaves the canvas unchanged");
    }

    #[test]
    fn lane_cover_clamps_above_point_nine() {
        let skin = skin();
        let mut c = CpuCanvas::new(1280, 720);
        let timelines = tls(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        render_playfield_view(&mut c, &skin, &view(&timelines, 0, 1.0, &[], &[], false));
        render_lane_cover(&mut c, &skin, 5.0);
        let cx = skin.lane_center(0) as u32;
        assert_ne!(c.pixel_at(cx, skin.judge_y as u32 - 3), skin.bg, "clamped cover leaves the bottom of the field visible");
    }

    #[test]
    fn render_is_deterministic_for_identical_inputs() {
        let timelines = tls(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:0101\r\n");
        let skin = skin();
        let mut a = CpuCanvas::new(1280, 720);
        let mut b = CpuCanvas::new(1280, 720);
        render_playfield_view(&mut a, &skin, &view(&timelines, 500_000, 1.5, &[0], &[], false));
        render_playfield_view(&mut b, &skin, &view(&timelines, 500_000, 1.5, &[0], &[], false));
        assert_eq!(a.pixels(), b.pixels(), "same inputs produce byte-identical frames");
    }

    #[test]
    fn empty_timelines_just_clears_and_draws_decor() {
        let skin = skin();
        let mut c = CpuCanvas::new(1280, 720);
        render_playfield_view(&mut c, &skin, &view(&[], 0, 1.0, &[], &[], false));
        assert_eq!(c.pixel_at(1, 1), skin.bg, "outside the field is the cleared bg");
        let cx = skin.lane_center(0) as u32;
        assert_eq!(c.pixel_at(cx, skin.judge_y as u32 + 1), skin.judge_line);
    }

    #[test]
    fn constant_and_floating_scroll_both_place_a_note_above_the_line() {
        let timelines = tls(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let skin = skin();
        let note_t = timelines.iter().find_map(|t| t.notes[0].as_ref().map(|n| n.time_us)).unwrap();
        let micro = note_t - 300_000;
        let cx = skin.lane_center(0) as u32;
        for constant in [false, true] {
            let mut c = CpuCanvas::new(1280, 720);
            render_playfield_view(&mut c, &skin, &view(&timelines, micro, 1.0, &[], &[], constant));
            let found = (skin.top_y as u32..skin.judge_y as u32).find(|&y| c.pixel_at(cx, y) == Color::WHITE);
            assert!(found.is_some(), "note visible above the line under constant={constant}");
        }
    }

    #[test]
    fn key_bomb_disabled_is_noop() {
        let mut skin = skin();
        skin.bomb_enabled = false;
        let mut a = CpuCanvas::new(640, 720);
        let mut b = CpuCanvas::new(640, 720);
        a.clear(Color::rgb(0, 0, 0));
        b.clear(Color::rgb(0, 0, 0));
        render_key_bomb(&mut b, &skin, &[(0, 0)], 1000);
        assert_eq!(a.pixels(), b.pixels(), "disabled bomb draws nothing");
    }

    #[test]
    fn key_bomb_skips_inactive_and_expired_lanes() {
        let skin = skin();
        let mut c = CpuCanvas::new(1280, 720);
        c.clear(Color::rgb(0, 0, 0));
        let bomb = vec![(i64::MIN, 0u8), (0i64, 0u8), (10_000_000i64, 0u8)];
        render_key_bomb(&mut c, &skin, &bomb, skin.bomb_us + 5_000_000);
        let lit = (0..1280 * 720).any(|i| c.pixel_at((i % 1280) as u32, (i / 1280) as u32).r > 5);
        assert!(!lit, "no active bomb in window -> nothing drawn");
    }

    #[test]
    fn key_bomb_draws_for_a_fresh_hit() {
        let skin = skin();
        let mut c = CpuCanvas::new(1280, 720);
        c.clear(Color::rgb(0, 0, 0));
        render_key_bomb(&mut c, &skin, &[(0i64, 0u8)], skin.bomb_us / 4);
        let lit = (0..1280 * 720).any(|i| c.pixel_at((i % 1280) as u32, (i / 1280) as u32).r > 5);
        assert!(lit, "a fresh hit draws a visible bomb");
    }

    #[test]
    fn key_bomb_clamps_judge_index_out_of_range() {
        let skin = skin();
        let mut c = CpuCanvas::new(1280, 720);
        c.clear(Color::rgb(0, 0, 0));
        render_key_bomb(&mut c, &skin, &[(0i64, 200u8)], skin.bomb_us / 4);
        let lit = (0..1280 * 720).any(|i| c.pixel_at((i % 1280) as u32, (i / 1280) as u32).a == 255);
        assert!(lit);
    }

    #[test]
    fn key_bomb_ignores_extra_bomb_entries_beyond_lane_count() {
        let skin = skin();
        let mut c = CpuCanvas::new(1280, 720);
        c.clear(Color::rgb(0, 0, 0));
        let mut bomb = vec![(i64::MIN, 0u8); skin.lane_count()];
        bomb.push((0i64, 0u8));
        render_key_bomb(&mut c, &skin, &bomb, skin.bomb_us / 4);
        let lit = (0..1280 * 720).any(|i| c.pixel_at((i % 1280) as u32, (i / 1280) as u32).r > 5);
        assert!(!lit, "extra bomb entries past lane_count are ignored");
    }

    /// The struct entry point and the spread-argument shim must produce the same frame under either
    /// scroll mode, so the player can migrate to [`PlayfieldView`] without a visual change.
    #[test]
    fn the_view_entry_point_and_the_spread_argument_shim_agree() {
        let timelines = tls(b"#LNTYPE 1\r\n#BPM 150\r\n#WAV01 a.wav\r\n#00111:01\r\n#00151:01000001\r\n");
        let skin = skin();
        let micro = 1_000_000;
        let beam_on = vec![0i64; skin.lane_count()];
        let beam_off = vec![i64::MIN; skin.lane_count()];

        for constant in [false, true] {
            let mut spread = CpuCanvas::new(1280, 720);
            render_playfield_view(&mut spread, &skin, &view(&timelines, micro, 1.25, &beam_on, &beam_off, constant));

            let mut view = CpuCanvas::new(1280, 720);
            let field = PlayfieldView { timelines: &timelines, microtime: micro, hispeed: 1.25, beam_on: &beam_on, beam_off: &beam_off, constant };
            render_playfield_view(&mut view, &skin, &field);

            assert_eq!(spread.pixels(), view.pixels(), "the compatibility shim only spreads the view fields (constant={constant})");
        }
    }
}
