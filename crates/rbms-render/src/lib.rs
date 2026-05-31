pub mod cpu;
pub mod font;
pub mod hud;
pub mod playfield;
pub mod result;
pub mod select;
pub mod skin;

pub use cpu::CpuCanvas;
pub use font::{draw_text, draw_text_centered, draw_text_right, fit_text, load_font, reset_ui_family, set_ui_family, text_width};
pub use hud::{HudView, render_hud};
pub use playfield::{render_key_bomb, render_lane_cover, render_playfield};
pub use result::{RANK_BANDS, ResultView, dj_rank, draw_rank_bar, ex_delta_label, render_result};
pub use select::{
    CoverState, DensityView, DetailView, RecordRowView, RecordsView, SelectDetail, SelectHot, SelectModal, SelectRow, SelectView, StatCell, cover_rect, render_select,
};
pub use skin::{Skin, SkinConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b, a: 255 }
    }
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    pub const WHITE: Color = Color::rgb(235, 235, 235);
    pub const BLUE: Color = Color::rgb(70, 130, 230);
    pub const RED: Color = Color::rgb(230, 70, 70);
    pub const GREEN: Color = Color::rgb(70, 220, 120);
    pub const YELLOW: Color = Color::rgb(230, 210, 70);
    pub const ORANGE: Color = Color::rgb(230, 150, 60);
    pub const GRAY: Color = Color::rgb(90, 90, 100);
    pub const LANE_BG: Color = Color::rgb(18, 18, 24);
    pub const JUDGE_LINE: Color = Color::rgb(200, 40, 40);
}

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Rect { x, y, w, h }
    }
}

/// Backend-agnostic 2D draw surface. The wgpu backend and the CPU reference backend
/// both implement this, so the playfield composer is GPU-independent.
pub trait Renderer {
    fn size(&self) -> (u32, u32);
    fn clear(&mut self, color: Color);
    fn fill_rect(&mut self, rect: Rect, color: Color);
}
