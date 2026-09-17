//! UI colour theme for all the non-skin chrome — the song-select, result, HUD overlays, and the
//! app's menu screens. (The in-play note field is themed separately by [`crate::SkinConfig`].)
//!
//! A [`ThemeConfig`] is loaded from a RON file (every field optional, so a partial theme keeps the
//! rest of the defaults) and [`ThemeConfig::resolve`]d into a concrete [`Theme`]. The app calls
//! [`set_theme`] once at startup; every renderer reads the current palette via [`theme`]. A future
//! web front-end can author themes against this same schema — see `docs/theme.md`.

use std::cell::RefCell;

use serde::Deserialize;

use crate::{Color, Rect};

const SELECT_CANVAS_WIDTH: f32 = 1280.0;
const SELECT_CANVAS_HEIGHT: f32 = 720.0;

pub const OPTIONS_ROW_COUNT: usize = 11;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OptionsLayout {
    pub rect: Option<Rect>,
    pub panel_alpha: u8,
    pub row_overrides: [Option<OptionsRowOverride>; OPTIONS_ROW_COUNT],
}

impl Default for OptionsLayout {
    fn default() -> Self {
        OptionsLayout { rect: None, panel_alpha: 235, row_overrides: [None; OPTIONS_ROW_COUNT] }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OptionsRowOverride {
    pub y: Option<f32>,
    pub label_x: Option<f32>,
    pub value_right_x: Option<f32>,
    pub label_color: Option<Color>,
    pub value_color: Option<Color>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(default)]
pub struct OptionsRowOverrideConfig {
    pub index: usize,
    pub y: Option<f32>,
    pub label_x: Option<f32>,
    pub value_right_x: Option<f32>,
    pub label_color: Option<(u8, u8, u8)>,
    pub value_color: Option<(u8, u8, u8)>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct OptionsLayoutConfig {
    pub rect: Option<(f32, f32, f32, f32)>,
    pub panel_alpha: Option<u8>,
    pub row_overrides: Option<Vec<OptionsRowOverrideConfig>>,
}

impl OptionsLayoutConfig {
    fn resolve(&self, default: OptionsLayout) -> OptionsLayout {
        let rect = self.rect.and_then(|(x, y, w, h)| {
            (x.is_finite()
                && y.is_finite()
                && w.is_finite()
                && h.is_finite()
                && x >= 0.0
                && y >= 0.0
                && w > 0.0
                && h > 0.0
                && x + w <= SELECT_CANVAS_WIDTH
                && y + h <= SELECT_CANVAS_HEIGHT)
                .then(|| Rect::new(x, y, w, h))
        });
        let panel = rect.unwrap_or(Rect::new(0.0, 0.0, SELECT_CANVAS_WIDTH, SELECT_CANVAS_HEIGHT));
        let mut row_overrides = default.row_overrides;
        for config in self.row_overrides.as_deref().unwrap_or_default() {
            let Some(slot) = row_overrides.get_mut(config.index) else {
                continue;
            };
            let position = |value: Option<f32>, maximum: f32| value.filter(|value| value.is_finite() && *value >= 0.0 && *value <= maximum);
            *slot = Some(OptionsRowOverride {
                y: position(config.y, panel.h),
                label_x: position(config.label_x, panel.w),
                value_right_x: position(config.value_right_x, panel.w),
                label_color: config.label_color.map(|(r, g, b)| Color::rgb(r, g, b)),
                value_color: config.value_color.map(|(r, g, b)| Color::rgb(r, g, b)),
            });
        }
        OptionsLayout { rect, panel_alpha: self.panel_alpha.unwrap_or(default.panel_alpha), row_overrides }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SelectLayout {
    pub list_rect: Rect,
    pub detail_rect: Rect,
    pub row_height: f32,
    pub row_gap: f32,
    pub cover_rect: Rect,
}

impl Default for SelectLayout {
    fn default() -> Self {
        SelectLayout {
            list_rect: Rect::new(32.0, 60.0, 584.0, 600.0),
            detail_rect: Rect::new(632.0, 60.0, 616.0, 600.0),
            row_height: 36.0,
            row_gap: 4.0,
            cover_rect: Rect::new(648.0, 78.0, 160.0, 160.0),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(default)]
pub struct SelectLayoutConfig {
    pub list_rect: Option<(f32, f32, f32, f32)>,
    pub detail_rect: Option<(f32, f32, f32, f32)>,
    pub row_height: Option<f32>,
    pub row_gap: Option<f32>,
    pub cover_rect: Option<(f32, f32, f32, f32)>,
}

impl SelectLayoutConfig {
    fn resolve(&self, default: SelectLayout) -> SelectLayout {
        let rect = |value: Option<(f32, f32, f32, f32)>, fallback: Rect| match value {
            Some((x, y, w, h))
                if x.is_finite()
                    && y.is_finite()
                    && w.is_finite()
                    && h.is_finite()
                    && x >= 0.0
                    && y >= 0.0
                    && w > 0.0
                    && h > 0.0
                    && x + w <= SELECT_CANVAS_WIDTH
                    && y + h <= SELECT_CANVAS_HEIGHT =>
            {
                Rect::new(x, y, w, h)
            }
            None => fallback,
            Some(_) => fallback,
        };
        let list_rect = rect(self.list_rect, default.list_rect);
        SelectLayout {
            list_rect,
            detail_rect: rect(self.detail_rect, default.detail_rect),
            row_height: self.row_height.filter(|value| value.is_finite() && *value > 0.0 && *value <= list_rect.h).unwrap_or(default.row_height),
            row_gap: self.row_gap.filter(|value| value.is_finite() && *value >= 0.0 && *value < list_rect.h).unwrap_or(default.row_gap),
            cover_rect: rect(self.cover_rect, default.cover_rect),
        }
    }
}

/// The resolved UI palette. Fields are grouped by role so a theme reads top-to-bottom.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub select_layout: SelectLayout,
    pub options_layout: OptionsLayout,
    pub select_panel_alpha: u8,
    /// Window background.
    pub bg: Color,
    /// Top header bar.
    pub topbar: Color,
    /// Panel fill (detail panel, list rows base).
    pub panel: Color,
    /// Raised panel / button fill.
    pub panel_hi: Color,
    /// Hairline dividers / outlines.
    pub divider: Color,

    /// Primary text.
    pub text: Color,
    /// Secondary / dimmed text.
    pub text_dim: Color,
    /// Tertiary / muted hints.
    pub text_muted: Color,
    /// Accent text (counts, sort label, links).
    pub accent: Color,
    /// Focus ring / selection rails.
    pub focus: Color,

    /// Focused row title.
    pub title_focus: Color,
    /// Unfocused row title.
    pub title_dim: Color,
    /// Even row background.
    pub row_even: Color,
    /// Odd row background.
    pub row_odd: Color,
    /// Folder row background.
    pub row_folder: Color,
    /// Focused row background.
    pub row_focus: Color,
    /// Clear-lamp default (no record yet).
    pub lamp_default: Color,

    /// Button / selected-tab fill.
    pub button: Color,
    /// Active/toggled button fill.
    pub button_active: Color,

    /// Positive bar (progress, score graph current).
    pub good: Color,
    /// Warning / pacemaker behind.
    pub warn: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            select_layout: SelectLayout::default(),
            options_layout: OptionsLayout::default(),
            select_panel_alpha: 255,
            bg: Color::rgb(8, 8, 14),
            topbar: Color::rgb(18, 18, 30),
            panel: Color::rgb(14, 16, 26),
            panel_hi: Color::rgb(22, 24, 36),
            divider: Color::rgb(40, 42, 58),
            text: Color::rgb(235, 235, 235),
            text_dim: Color::rgb(170, 170, 185),
            text_muted: Color::rgb(120, 124, 140),
            accent: Color::rgb(120, 205, 235),
            focus: Color::rgb(120, 200, 240),
            title_focus: Color::rgb(244, 236, 156),
            title_dim: Color::rgb(190, 186, 150),
            row_even: Color::rgb(22, 24, 34),
            row_odd: Color::rgb(27, 29, 42),
            row_folder: Color::rgb(40, 44, 30),
            row_focus: Color::rgb(50, 56, 82),
            lamp_default: Color::rgb(44, 44, 54),
            button: Color::rgb(70, 80, 120),
            button_active: Color::rgb(40, 56, 80),
            good: Color::rgb(90, 200, 230),
            warn: Color::rgb(230, 180, 60),
        }
    }
}

/// Serde RON form of a theme: every colour is an optional `(r, g, b)` triple, so a file may set only
/// the colours it wants and inherit the rest from [`Theme::default`]. See `docs/theme.md`.
#[derive(Deserialize, Default, Debug)]
#[serde(default)]
pub struct ThemeConfig {
    pub select_layout: Option<SelectLayoutConfig>,
    pub options_layout: Option<OptionsLayoutConfig>,
    pub select_panel_alpha: Option<u8>,
    pub bg: Option<(u8, u8, u8)>,
    pub topbar: Option<(u8, u8, u8)>,
    pub panel: Option<(u8, u8, u8)>,
    pub panel_hi: Option<(u8, u8, u8)>,
    pub divider: Option<(u8, u8, u8)>,
    pub text: Option<(u8, u8, u8)>,
    pub text_dim: Option<(u8, u8, u8)>,
    pub text_muted: Option<(u8, u8, u8)>,
    pub accent: Option<(u8, u8, u8)>,
    pub focus: Option<(u8, u8, u8)>,
    pub title_focus: Option<(u8, u8, u8)>,
    pub title_dim: Option<(u8, u8, u8)>,
    pub row_even: Option<(u8, u8, u8)>,
    pub row_odd: Option<(u8, u8, u8)>,
    pub row_folder: Option<(u8, u8, u8)>,
    pub row_focus: Option<(u8, u8, u8)>,
    pub lamp_default: Option<(u8, u8, u8)>,
    pub button: Option<(u8, u8, u8)>,
    pub button_active: Option<(u8, u8, u8)>,
    pub good: Option<(u8, u8, u8)>,
    pub warn: Option<(u8, u8, u8)>,
}

impl ThemeConfig {
    /// Parse a RON theme string; any error yields an all-defaults config (so a broken theme can't
    /// brick the UI).
    pub fn parse(ron_src: &str) -> ThemeConfig {
        ron::from_str(ron_src).unwrap_or_default()
    }

    /// Merge this config over [`Theme::default`] — unset fields keep their default colour.
    pub fn resolve(&self) -> Theme {
        let d = Theme::default();
        let c = |o: Option<(u8, u8, u8)>, def: Color| o.map(|(r, g, b)| Color::rgb(r, g, b)).unwrap_or(def);
        Theme {
            select_layout: self.select_layout.unwrap_or_default().resolve(d.select_layout),
            options_layout: self.options_layout.as_ref().map_or(d.options_layout, |layout| layout.resolve(d.options_layout)),
            select_panel_alpha: self.select_panel_alpha.unwrap_or(d.select_panel_alpha),
            bg: c(self.bg, d.bg),
            topbar: c(self.topbar, d.topbar),
            panel: c(self.panel, d.panel),
            panel_hi: c(self.panel_hi, d.panel_hi),
            divider: c(self.divider, d.divider),
            text: c(self.text, d.text),
            text_dim: c(self.text_dim, d.text_dim),
            text_muted: c(self.text_muted, d.text_muted),
            accent: c(self.accent, d.accent),
            focus: c(self.focus, d.focus),
            title_focus: c(self.title_focus, d.title_focus),
            title_dim: c(self.title_dim, d.title_dim),
            row_even: c(self.row_even, d.row_even),
            row_odd: c(self.row_odd, d.row_odd),
            row_folder: c(self.row_folder, d.row_folder),
            row_focus: c(self.row_focus, d.row_focus),
            lamp_default: c(self.lamp_default, d.lamp_default),
            button: c(self.button, d.button),
            button_active: c(self.button_active, d.button_active),
            good: c(self.good, d.good),
            warn: c(self.warn, d.warn),
        }
    }
}

thread_local! {
    static THEME: RefCell<Theme> = RefCell::new(Theme::default());
}

/// Install the active UI theme (call once at startup, before rendering).
pub fn set_theme(t: Theme) {
    THEME.with(|c| *c.borrow_mut() = t);
}

/// The active UI theme. Cheap to copy (a handful of `Color`s); read once per render.
pub fn theme() -> Theme {
    THEME.with(|c| *c.borrow())
}

pub fn select_layout() -> SelectLayout {
    theme().select_layout
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_config_resolves_to_defaults() {
        let t = ThemeConfig::default().resolve();
        assert_eq!(t, Theme::default());
    }

    #[test]
    fn partial_config_overrides_only_set_fields() {
        let cfg = ThemeConfig::parse("(bg: Some((1, 2, 3)), accent: Some((9, 9, 9)))");
        let t = cfg.resolve();
        assert_eq!(t.bg, Color::rgb(1, 2, 3), "bg overridden");
        assert_eq!(t.accent, Color::rgb(9, 9, 9), "accent overridden");
        assert_eq!(t.panel, Theme::default().panel, "unset field keeps default");
        assert_eq!(t.text, Theme::default().text, "unset field keeps default");
    }

    #[test]
    fn partial_select_layout_keeps_the_legacy_geometry() {
        let cfg = ThemeConfig::parse("(select_layout: Some((list_rect: Some((664.0, 60.0, 584.0, 600.0)))) )");
        let layout = cfg.resolve().select_layout;
        assert_eq!(layout.list_rect, Rect::new(664.0, 60.0, 584.0, 600.0));
        assert_eq!(layout.detail_rect, SelectLayout::default().detail_rect);
        assert_eq!(layout.row_height, SelectLayout::default().row_height);
    }

    #[test]
    fn an_options_layout_and_translucent_select_panels_are_file_configurable() {
        let cfg =
            ThemeConfig::parse("(options_layout: Some((rect: Some((240.0, 122.0, 800.0, 476.0)), panel_alpha: Some(255))), select_panel_alpha: Some(210))");
        let theme = cfg.resolve();
        assert_eq!(theme.options_layout.rect, Some(Rect::new(240.0, 122.0, 800.0, 476.0)));
        assert_eq!(theme.options_layout.panel_alpha, 255);
        assert_eq!(theme.select_panel_alpha, 210);
    }

    #[test]
    fn an_options_row_can_be_moved_and_recolored_without_moving_its_neighbors() {
        let cfg = ThemeConfig::parse(
            "(options_layout: Some((rect: Some((240.0, 122.0, 800.0, 476.0)), row_overrides: Some([(index: 2, y: Some(98.0), label_x: Some(28.0), value_right_x: Some(760.0), label_color: Some((9, 8, 7)), value_color: Some((6, 5, 4)))]))))",
        );
        let layout = cfg.resolve().options_layout;
        assert!(layout.row_overrides[0].is_none());
        assert_eq!(
            layout.row_overrides[2].expect("the third row"),
            OptionsRowOverride {
                y: Some(98.0),
                label_x: Some(28.0),
                value_right_x: Some(760.0),
                label_color: Some(Color::rgb(9, 8, 7)),
                value_color: Some(Color::rgb(6, 5, 4)),
            }
        );
        assert!(layout.row_overrides[3].is_none());
    }

    #[test]
    fn invalid_select_geometry_uses_safe_defaults() {
        let cfg = ThemeConfig::parse("(select_layout: Some((list_rect: Some((0.0, 60.0, 1281.0, 600.0)), row_height: Some(0.0), row_gap: Some(-1.0))))");
        assert_eq!(cfg.resolve().select_layout, SelectLayout::default());
        let zero_pitch = ThemeConfig::parse("(select_layout: Some((row_height: Some(0.0), row_gap: Some(0.0))))").resolve().select_layout;
        assert!(zero_pitch.row_height + zero_pitch.row_gap > 0.0);
    }

    #[test]
    fn malformed_ron_falls_back_to_defaults() {
        assert_eq!(ThemeConfig::parse("not a theme at all").resolve(), Theme::default());
        assert_eq!(ThemeConfig::parse("").resolve(), Theme::default());
    }

    #[test]
    fn set_and_get_theme_round_trips() {
        let t = Theme { bg: Color::rgb(42, 0, 0), ..Theme::default() };
        set_theme(t);
        assert_eq!(theme().bg, Color::rgb(42, 0, 0));
        set_theme(Theme::default());
        assert_eq!(theme(), Theme::default());
    }
}
