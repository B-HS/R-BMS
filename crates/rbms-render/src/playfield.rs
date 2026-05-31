use rbms_chart::scroll::{constant_offsets, visible_offsets};
use rbms_model::{NoteKind, TimeLine};

use crate::skin::Skin;
use crate::{Color, Rect, Renderer};

/// Release-fade length of a key beam after a lane is let go (µs). Mirrors a skin's keyoff
/// animation: the beam fades from full to invisible over this span.
const BEAM_RELEASE_US: i64 = 120_000;

/// Vertical gradient slices per beam — more slices = smoother brightest-at-the-judge-line falloff.
const BEAM_SLICES: usize = 6;

/// Draw the playfield for the current play time using a `Skin`: lane backgrounds, key beams
/// for held/just-pressed lanes, the judgment line, and every visible note at its scroll
/// offset. `beam_on`/`beam_off` are the per-lane press/release timestamps (µs, `i64::MIN`
/// when inactive); pass `&[]` to omit beams. Backend-agnostic.
pub fn render_playfield<R: Renderer>(r: &mut R, timelines: &[TimeLine], microtime: i64, hispeed: f64, skin: &Skin, beam_on: &[i64], beam_off: &[i64], constant: bool) {
    r.clear(skin.bg);

    let n = skin.lane_count();
    let lane_h = skin.judge_y - skin.top_y;
    for lane in 0..n {
        r.fill_rect(Rect::new(skin.x[lane], skin.top_y, skin.w[lane], lane_h), skin.lane_bg);
    }

    draw_key_beams(r, skin, beam_on, beam_off, microtime);
    draw_field_decor(r, skin);

    // Judgment line, drawn per field so it does not bridge the gap between DP fields.
    for &(fx, fw) in &skin.fields {
        r.fill_rect(Rect::new(fx, skin.judge_y, fw, 4.0), skin.judge_line);
    }

    let offsets = if constant {
        constant_offsets(timelines, microtime, hispeed, skin.lane_height())
    } else {
        visible_offsets(timelines, microtime, hispeed, skin.lane_height())
    };
    for (i, off) in offsets {
        let tl = &timelines[i];
        for lane in 0..n {
            let Some(note) = &tl.notes[lane] else { continue };
            if matches!(note.kind, NoteKind::LongEnd { .. }) {
                continue;
            }
            let color = match note.kind {
                NoteKind::Mine { .. } => skin.mine_color,
                _ => skin.note_color(lane),
            };
            // Clip the note to the field's top edge: a note still partly above `top_y` is drawn only
            // for the portion inside the field, so it slides out from under the frame instead of
            // popping in whole above it. The field rect [top_y, judge_y] is the real boundary — no
            // occluding overlay needed.
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

    #[test]
    fn note_drawn_at_expected_lane_and_offset() {
        let timelines = tls(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let skin = skin();
        let note_t = 2_000_000;
        let micro = note_t - 300_000;
        let mut canvas = CpuCanvas::new(1280, 720);
        render_playfield(&mut canvas, &timelines, micro, 1.0, &skin, &[], &[], false);

        let off = closed_form_offset(note_t, micro, 120.0, 1.0, 1.0, skin.lane_height());
        let cx = skin.lane_center(0) as u32;
        let cy = (skin.judge_y - off - skin.note_height * 0.5) as u32;
        assert_eq!(canvas.pixel_at(cx, cy), Color::WHITE, "note should be white at lane 0");
        assert_ne!(canvas.pixel_at(skin.lane_center(3) as u32, cy), Color::WHITE, "lane 3 has no note here");
    }

    #[test]
    fn judgment_line_is_drawn() {
        let timelines = tls(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let skin = skin();
        let mut canvas = CpuCanvas::new(1280, 720);
        render_playfield(&mut canvas, &timelines, 0, 1.0, &skin, &[], &[], false);
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
            render_playfield(&mut c, &timelines, micro, hispeed, &skin, &[], &[], false);
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
        render_playfield(&mut off, &timelines, 0, 1.0, &skin, &[], &[], false);

        let mut on = CpuCanvas::new(1280, 720);
        render_playfield(&mut on, &timelines, 0, 1.0, &skin, &[0], &[], false);

        assert_ne!(on.pixel_at(cx0, cy), off.pixel_at(cx0, cy), "held lane 0 beam brightens near the judge line");
        assert_eq!(on.pixel_at(cx3, cy), off.pixel_at(cx3, cy), "lane 3 with no beam is unchanged");
    }

    #[test]
    fn key_beam_fades_to_nothing_after_release() {
        assert_eq!(beam_intensity(i64::MIN, 0, BEAM_RELEASE_US + 1), 0.0, "fully faded past the release window");
        assert_eq!(beam_intensity(i64::MIN, 0, 0), 1.0, "full at the moment of release");
        assert_eq!(beam_intensity(5, i64::MIN, 9_999), 1.0, "held lane stays full");
    }
}

/// Draw the key bomb: for each lane whose most recent note hit is within the skin's bomb window, an
/// expanding judge-coloured burst with a shrinking white core at the judgment line. `bomb[lane]` is
/// `(hit_us, judge_index)` (`i64::MIN` = none). Data-driven via `Skin::bomb_*`; no-op when disabled.
pub fn render_key_bomb<R: Renderer>(r: &mut R, skin: &Skin, bomb: &[(i64, u8)], microtime: i64) {
    if !skin.bomb_enabled || skin.bomb_us <= 0 {
        return;
    }
    for lane in 0..skin.lane_count().min(bomb.len()) {
        let (hit_us, judge) = bomb[lane];
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
        // Outer burst: grows and fades, tinted by the judgment colour.
        let size = base * (0.45 + 0.75 * p);
        let jc = skin.judge_colors[(judge as usize).min(5)];
        r.fill_rect(Rect::new(cx - size * 0.5, cy - size * 0.5, size, size), Color { a: ((1.0 - p) * 200.0) as u8, ..jc });
        // Inner core: bright white, shrinks to a point.
        let core = base * (1.0 - p) * 0.55;
        if core > 1.0 {
            r.fill_rect(Rect::new(cx - core * 0.5, cy - core * 0.5, core, core), Color { r: 255, g: 255, b: 255, a: ((1.0 - p) * 235.0) as u8 });
        }
    }
}
