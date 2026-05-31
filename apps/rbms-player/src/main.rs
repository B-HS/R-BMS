use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use rbms_audio::AudioEngine;
use rbms_chart::shuffle::NoteOption;
use rbms_chart::to_model;
use rbms_ir::{API_VERSION, ChartId, JudgeBreakdown, NullScoreServer, PlayOptions, PlayerId, RandomOption, ScoreServer, ScoreSubmission};
use rbms_judge::{ClearType, GaugeKind};
use rbms_model::Mode;
use rbms_play::{PlayEvent, Player};
use rbms_render::{
    Color, CoverState, DensityView, DetailView, HudView, RANK_BANDS, Rect, RecordRowView, RecordsView, Renderer, ResultView, SelectDetail, SelectHot, SelectModal, SelectRow,
    SelectView as SelectScene, Skin, SkinConfig, StatCell, cover_rect, dj_rank, draw_text, draw_text_centered, draw_text_right, ex_delta_label, render_hud, render_lane_cover, render_playfield,
    render_key_bomb, render_result, render_select, text_width,
};

use sha2::{Digest, Sha256};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

mod keyconfig;
mod replay;
mod scores;
mod settings;
mod tables;
use keyconfig::{ControlAction, KeyConfig, key_from_name, key_name};
use replay::{Replay, ReplayEvent};
use scores::{ScoreBook, ScoreRecord};
use settings::PlaySettings;
use tables::{TableList, TableSource};

const CW: u32 = 1280;
const CH: u32 = 720;
const MODE: Mode = Mode::BEAT_7K;

/// `#PREVIEW` hover-preview tuning: the reserved sample id, focus-settle debounce (frames), and
/// playback gain.
const PREVIEW_ID: u32 = 0;
const PREVIEW_DEBOUNCE_FRAMES: u64 = 20;
const PREVIEW_GAIN: f32 = 0.85;

struct PlayerConfig {
    keys_override: Option<Vec<(KeyCode, usize)>>,
    scratch_left: bool,
    scratch_auto: bool,
    lift: f32,
    cover: f32,
    hispeed: f64,
    gauge: GaugeKind,
    random: NoteOption,
    constant_speed: bool,
    offset_ms: i32,
    auto_offset: bool,
    judge_rate: i32,
    total_override: f64,
    bga: bool,
    skin_name: String,
    skin_path: Option<String>,
    server_url: Option<String>,
    player_id: String,
    table_url: Option<String>,
    keyconfig_path: Option<String>,
    replay_path: Option<String>,
    auto_replay: bool,
    debug: bool,
    font_path: Option<String>,
    score_graph: bool,
    replay_analysis: bool,
    preview: bool,
    songs_folder: Option<String>,
}

impl Default for PlayerConfig {
    fn default() -> Self {
        PlayerConfig {
            keys_override: None,
            scratch_left: false,
            scratch_auto: false,
            lift: 0.0,
            cover: 0.0,
            hispeed: 2.0,
            gauge: GaugeKind::Normal,
            random: NoteOption::Off,
            constant_speed: false,
            offset_ms: 0,
            auto_offset: false,
            judge_rate: 100,
            total_override: 0.0,
            bga: true,
            skin_name: "NORMAL".into(),
            skin_path: None,
            server_url: None,
            player_id: "guest".into(),
            table_url: None,
            keyconfig_path: None,
            replay_path: None,
            auto_replay: true,
            debug: false,
            font_path: None,
            score_graph: true,
            replay_analysis: true,
            preview: true,
            songs_folder: None,
        }
    }
}

fn gauge_from_name(s: &str) -> GaugeKind {
    match s.to_ascii_lowercase().as_str() {
        "assist" | "assisteasy" => GaugeKind::AssistEasy,
        "easy" => GaugeKind::Easy,
        "hard" => GaugeKind::Hard,
        "exhard" => GaugeKind::ExHard,
        "hazard" => GaugeKind::Hazard,
        _ => GaugeKind::Normal,
    }
}

fn ir_clear(c: ClearType) -> rbms_ir::ClearLamp {
    use rbms_ir::ClearLamp as L;
    match c {
        ClearType::NoPlay => L::NoPlay,
        ClearType::Failed => L::Failed,
        ClearType::AssistEasy => L::AssistEasy,
        ClearType::Easy => L::Easy,
        ClearType::Normal => L::Normal,
        ClearType::Hard => L::Hard,
        ClearType::ExHard => L::ExHard,
        ClearType::FullCombo => L::FullCombo,
        ClearType::Perfect => L::Perfect,
        ClearType::Max => L::Max,
    }
}

fn ir_gauge(g: GaugeKind) -> rbms_ir::GaugeType {
    use rbms_ir::GaugeType as G;
    match g {
        GaugeKind::AssistEasy => G::AssistEasy,
        GaugeKind::Easy => G::Easy,
        GaugeKind::Normal => G::Normal,
        GaugeKind::Hard => G::Hard,
        GaugeKind::ExHard => G::ExHard,
        GaugeKind::Hazard => G::Hazard,
    }
}

fn ir_random(n: NoteOption) -> RandomOption {
    match n {
        NoteOption::Off => RandomOption::Off,
        NoteOption::Mirror => RandomOption::Mirror,
        NoteOption::Random => RandomOption::Random,
        NoteOption::SRandom => RandomOption::SRandom,
        NoteOption::RRandom => RandomOption::RRandom,
        NoteOption::Rotate => RandomOption::Spiral,
        NoteOption::HRandom => RandomOption::HRandom,
        NoteOption::AllScratch => RandomOption::AllScratch,
    }
}

/// Canonical gauge token for settings storage (matches `gauge_from_name`'s vocabulary, so it
/// round-trips — unlike the display name `gauge_name` which has spaces/hyphens).
fn gauge_token(g: GaugeKind) -> &'static str {
    match g {
        GaugeKind::AssistEasy => "assist",
        GaugeKind::Easy => "easy",
        GaugeKind::Normal => "normal",
        GaugeKind::Hard => "hard",
        GaugeKind::ExHard => "exhard",
        GaugeKind::Hazard => "hazard",
    }
}

fn apply_settings(cfg: &mut PlayerConfig, s: &PlaySettings) {
    cfg.hispeed = s.hispeed.clamp(0.5, 10.0);
    cfg.gauge = gauge_from_name(&s.gauge);
    cfg.lift = s.lift.clamp(0.0, 0.9);
    cfg.cover = s.cover.clamp(0.0, 0.9);
    cfg.scratch_left = s.scratch_left;
    cfg.scratch_auto = s.scratch_auto;
    cfg.random = NoteOption::from_str(&s.random);
    cfg.constant_speed = s.constant_speed;
    cfg.offset_ms = s.offset_ms.clamp(-200, 200);
    cfg.auto_offset = s.auto_offset;
    cfg.judge_rate = s.judge_rate.clamp(50, 200);
    cfg.total_override = s.total_override.max(0.0);
    cfg.bga = s.bga;
    cfg.auto_replay = s.auto_replay;
    cfg.debug = s.debug;
    cfg.font_path = s.font_path.clone();
    cfg.score_graph = s.score_graph;
    cfg.replay_analysis = s.replay_analysis;
    cfg.preview = s.preview;
    cfg.songs_folder = s.songs_folder.clone();
    if !s.skin.trim().is_empty() {
        cfg.skin_name = s.skin.to_ascii_uppercase();
    }
}

/// SHA-256 of the running client binary, submitted as `client_build_sha256` for build-integrity /
/// ranked eligibility. Computed once at startup. `None` if the executable can't be read — the
/// server decides how to treat an unknown build (e.g. ranked=false).
fn compute_build_hash() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let bytes = std::fs::read(exe).ok()?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Some(format!("{:x}", hasher.finalize()))
}

/// Client platform tag (`OS-ARCH`, e.g. `macos-aarch64`) paired with the build hash.
fn client_platform() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

/// beatoraja-style default gauge TOTAL for a chart that omits `#TOTAL`, scaled by note count
/// (denser charts gain more per note). Charts almost always set `#TOTAL`; this is the fallback.
fn default_total(notes: usize) -> f64 {
    let n = notes.max(1) as f64;
    (7.605 * n / (0.01 * n + 6.5)).max(260.0)
}

/// Auto-calibration: the new judge offset (ms) given the run's mean timing error. The offset
/// is held constant during a run (so judging is consistent and replays reproduce); the mean
/// error then recentres timing for the NEXT run, converging in ~one play. `mean_us` > 0 = the
/// player was early (FAST), so the offset increases to compensate.
fn calibrated_offset(current_ms: i32, mean_us: i64) -> i32 {
    (current_ms + (mean_us as f64 / 1000.0).round() as i32).clamp(-200, 200)
}

const SKIN_NORMAL: &str = include_str!("../../../assets/skins/normal.ron");
const SKIN_WIDE: &str = include_str!("../../../assets/skins/wide.ron");

/// One of the bundled skins by name ("WIDE" or NORMAL). Used when no external `--skin` is given.
fn bundled_skin(name: &str) -> SkinConfig {
    let s = if name.eq_ignore_ascii_case("WIDE") { SKIN_WIDE } else { SKIN_NORMAL };
    ron::from_str(s).unwrap_or_default()
}

const SHADER: &str = r#"
@group(0) @binding(0) var<uniform> screen: vec4<f32>;
struct VsOut { @builtin(position) pos: vec4<f32>, @location(0) color: vec4<f32> };
@vertex
fn vs(@builtin(vertex_index) vi: u32, @location(0) rect: vec4<f32>, @location(1) color: vec4<f32>) -> VsOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0));
    let c = corners[vi];
    let px = rect.xy + c * rect.zw;
    let ndc = vec2<f32>(px.x / screen.x * 2.0 - 1.0, 1.0 - px.y / screen.y * 2.0);
    var o: VsOut;
    o.pos = vec4<f32>(ndc, 0.0, 1.0);
    o.color = color;
    return o;
}
@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> { return in.color; }
"#;

const BGA_DIM: u32 = 256;
const BGA_SHADER: &str = r#"
struct U { screen: vec4<f32>, rect: vec4<f32> };
@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var t: texture_2d<f32>;
@group(0) @binding(2) var s: sampler;
struct VsOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex
fn vs(@builtin(vertex_index) vi: u32) -> VsOut {
    var c = array<vec2<f32>, 6>(vec2<f32>(0.,0.), vec2<f32>(1.,0.), vec2<f32>(0.,1.), vec2<f32>(0.,1.), vec2<f32>(1.,0.), vec2<f32>(1.,1.));
    let q = c[vi];
    let px = u.rect.xy + q * u.rect.zw;
    let ndc = vec2<f32>(px.x / u.screen.x * 2. - 1., 1. - px.y / u.screen.y * 2.);
    var o: VsOut;
    o.pos = vec4<f32>(ndc, 0., 1.);
    o.uv = q;
    return o;
}
@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> { return textureSample(t, s, in.uv); }
"#;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Instance {
    rect: [f32; 4],
    color: [f32; 4],
}

const INSTANCE_ATTRS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4];

/// Native instanced-quad renderer. Every `fill_rect` becomes one GPU instance; the whole
/// note field is drawn in a single instanced draw call (no CPU rasterisation / texture
/// upload). Implements `rbms_render::Renderer`, so the playfield/result composers target
/// it directly.
struct Gpu {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    instances: wgpu::Buffer,
    instance_cap: usize,
    quads: Vec<Instance>,
    clear_color: Color,
    bga_pipeline: wgpu::RenderPipeline,
    bga_uniform: wgpu::Buffer,
    bga_tex: wgpu::Texture,
    bga_bind_group: wgpu::BindGroup,
    bga_active: bool,
}

impl Gpu {
    fn new(window: Arc<Window>) -> Gpu {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            flags: wgpu::InstanceFlags::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        });
        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("no graphics adapter");
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).expect("no device");

        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats.iter().copied().find(|f| f.is_srgb()).unwrap_or(caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("screen"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&uniform, 0, bytemuck::cast_slice(&[CW as f32, CH as f32, 0.0, 0.0]));

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: uniform.as_entire_binding() }],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("quad"), source: wgpu::ShaderSource::Wgsl(SHADER.into()) });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[Some(&bgl)], immediate_size: 0 });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Instance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &INSTANCE_ATTRS,
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                targets: &[Some(wgpu::ColorTargetState { format, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let instance_cap = 8192;
        let instances = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instances"),
            size: (instance_cap * std::mem::size_of::<Instance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bga_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bga"),
            size: wgpu::Extent3d { width: BGA_DIM, height: BGA_DIM, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let bga_view = bga_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let bga_sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
        let bga_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bga_u"),
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bga_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::VERTEX, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            ],
        });
        let bga_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bga_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: bga_uniform.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&bga_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&bga_sampler) },
            ],
        });
        let bga_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("bga"), source: wgpu::ShaderSource::Wgsl(BGA_SHADER.into()) });
        let bga_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[Some(&bga_bgl)], immediate_size: 0 });
        let bga_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&bga_pl),
            vertex: wgpu::VertexState { module: &bga_shader, entry_point: Some("vs"), buffers: &[], compilation_options: Default::default() },
            fragment: Some(wgpu::FragmentState { module: &bga_shader, entry_point: Some("fs"), targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })], compilation_options: Default::default() }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        Gpu {
            window,
            surface,
            device,
            queue,
            config,
            pipeline,
            bind_group,
            instances,
            instance_cap,
            quads: Vec::new(),
            clear_color: Color::BLACK,
            bga_pipeline,
            bga_uniform,
            bga_tex,
            bga_bind_group,
            bga_active: false,
        }
    }

    fn set_bga(&mut self, rgba: &[u8], rect: Rect) {
        if rgba.len() != (BGA_DIM * BGA_DIM * 4) as usize {
            return;
        }
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &self.bga_tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            rgba,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4 * BGA_DIM), rows_per_image: Some(BGA_DIM) },
            wgpu::Extent3d { width: BGA_DIM, height: BGA_DIM, depth_or_array_layers: 1 },
        );
        self.queue.write_buffer(&self.bga_uniform, 0, bytemuck::cast_slice(&[CW as f32, CH as f32, 0.0, 0.0, rect.x, rect.y, rect.w, rect.h]));
        self.bga_active = true;
    }

    fn clear_bga(&mut self) {
        self.bga_active = false;
    }

    /// Number of quad instances queued so far this frame (debug overlay metric).
    fn quad_count(&self) -> usize {
        self.quads.len()
    }

    fn render(&mut self) {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            _ => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
        };

        if self.quads.len() > self.instance_cap {
            self.instance_cap = self.quads.len().next_power_of_two();
            self.instances = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("instances"),
                size: (self.instance_cap * std::mem::size_of::<Instance>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if !self.quads.is_empty() {
            self.queue.write_buffer(&self.instances, 0, bytemuck::cast_slice(&self.quads));
        }

        let c = self.clear_color;
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut enc = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: c.r as f64 / 255.0, g: c.g as f64 / 255.0, b: c.b as f64 / 255.0, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if self.bga_active {
                rp.set_pipeline(&self.bga_pipeline);
                rp.set_bind_group(0, &self.bga_bind_group, &[]);
                rp.draw(0..6, 0..1);
            }
            if !self.quads.is_empty() {
                rp.set_pipeline(&self.pipeline);
                rp.set_bind_group(0, &self.bind_group, &[]);
                rp.set_vertex_buffer(0, self.instances.slice(..));
                rp.draw(0..6, 0..self.quads.len() as u32);
            }
        }
        self.queue.submit([enc.finish()]);
        self.window.pre_present_notify();
        frame.present();
    }

    fn resize(&mut self, w: u32, h: u32) {
        if w > 0 && h > 0 {
            self.config.width = w;
            self.config.height = h;
            self.surface.configure(&self.device, &self.config);
        }
    }
}

impl Renderer for Gpu {
    fn size(&self) -> (u32, u32) {
        (CW, CH)
    }

    fn clear(&mut self, color: Color) {
        self.quads.clear();
        self.clear_color = color;
    }

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.quads.push(Instance {
            rect: [rect.x, rect.y, rect.w, rect.h],
            color: [color.r as f32 / 255.0, color.g as f32 / 255.0, color.b as f32 / 255.0, color.a as f32 / 255.0],
        });
    }
}

fn resolve_file(dir: &Path, name: &str, exts: &[&str]) -> Option<(PathBuf, String)> {
    let name = name.replace('\\', "/");
    let direct = dir.join(&name);
    if direct.is_file() {
        let ext = direct.extension().and_then(|e| e.to_str()).unwrap_or("").to_string();
        return Some((direct, ext));
    }
    let stem = Path::new(&name).with_extension("");
    for ext in exts {
        let p = dir.join(format!("{}.{ext}", stem.display()));
        if p.is_file() {
            return Some((p, ext.to_string()));
        }
    }
    None
}

fn resolve_keysound(dir: &Path, name: &str) -> Option<(PathBuf, String)> {
    resolve_file(dir, name, &["ogg", "wav", "flac", "mp3"])
}

fn decode_bga_256(dir: &Path, name: &str) -> Option<Vec<u8>> {
    let (path, _) = resolve_file(dir, name, &["png", "bmp", "jpg", "jpeg"])?;
    let bytes = std::fs::read(&path).ok()?;
    let img = image::load_from_memory(&bytes).ok()?;
    Some(img.resize_exact(BGA_DIM, BGA_DIM, image::imageops::FilterType::Triangle).to_rgba8().into_raw())
}

struct SongEntry {
    path: PathBuf,
    title: String,
    subtitle: String,
    artist: String,
    genre: String,
    maker: String,
    level: String,
    difficulty: i32,
    init_bpm: f64,
    rank: i32,
    total: f64,
    mode: Mode,
    md5: String,
    stagefile: String,
    banner: String,
    /// `#PREVIEW` audio path. Parsed and carried now; focus-preview playback is a follow-up
    /// (the audio engine is only alive during Play — see ROADMAP "UI 고도화").
    #[allow(dead_code)]
    preview: String,
}

/// Per-chart details that need full timing integration (`to_model`), computed lazily for the focused
/// song only (not every chart in the library) and cached in `App::focused_detail`.
struct ChartDetail {
    notes: usize,
    long_notes: usize,
    duration_us: i64,
    bpm_min: f64,
    bpm_max: f64,
    density: Vec<u32>,
    peak_density: f64,
    avg_density: f64,
    end_density: f64,
}

fn is_chart(p: &Path) -> bool {
    matches!(p.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref(), Some("bms" | "bme" | "bml" | "pms"))
}

fn scan_folder(root: &Path) -> Vec<SongEntry> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if is_chart(&p) {
                if let Ok(bytes) = std::fs::read(&p) {
                    let src = rbms_parser::parse(&bytes);
                    let mode = rbms_chart::detect_mode(&src, p.to_str().unwrap_or(""));
                    let h = &src.headers;
                    let title = if h.title.is_empty() { p.file_name().and_then(|n| n.to_str()).unwrap_or("?").to_string() } else { h.title.clone() };
                    out.push(SongEntry {
                        path: p,
                        title,
                        subtitle: h.subtitle.clone(),
                        artist: h.artist.clone(),
                        genre: h.genre.clone(),
                        maker: h.maker.clone(),
                        level: h.play_level.clone(),
                        difficulty: h.difficulty,
                        init_bpm: h.init_bpm,
                        rank: h.rank,
                        total: h.total.unwrap_or(0.0),
                        mode,
                        md5: src.md5.clone(),
                        stagefile: h.stagefile.clone(),
                        banner: h.banner.clone(),
                        preview: h.preview.clone(),
                    });
                }
            }
        }
    }
    out.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    out
}

/// Parse + time-integrate a single chart to derive its playable-note count, long-note count, length
/// and BPM range for the select detail panel. `None` if the file can't be read.
fn compute_chart_detail(path: &Path, mode: Mode) -> Option<ChartDetail> {
    let bytes = std::fs::read(path).ok()?;
    let src = rbms_parser::parse(&bytes);
    let total_value = src.headers.total.unwrap_or(0.0);
    let model = to_model(&src, mode);
    let long_notes = model
        .timelines
        .iter()
        .flat_map(|tl| tl.notes.iter().flatten())
        .filter(|n| matches!(n.kind, rbms_model::NoteKind::LongStart { .. }))
        .count();
    let duration_us = model.timelines.last().map(|t| t.time_us).unwrap_or(0);
    let mut bpm_min = f64::MAX;
    let mut bpm_max = f64::MIN;
    for tl in &model.timelines {
        if tl.bpm > 0.0 {
            bpm_min = bpm_min.min(tl.bpm);
            bpm_max = bpm_max.max(tl.bpm);
        }
    }
    if bpm_min > bpm_max {
        bpm_min = model.init_bpm;
        bpm_max = model.init_bpm;
    }
    let dens = rbms_chart::note_density(&model, total_value);
    Some(ChartDetail { notes: rbms_chart::count_playable_notes(&model), long_notes, duration_us, bpm_min, bpm_max, density: dens.bins, peak_density: dens.peak, avg_density: dens.avg, end_density: dens.end })
}

#[derive(PartialEq, Debug)]
enum Stage {
    Select,
    Settings,
    KeyConfig,
    Tables,
    Loading,
    Play,
    Result,
}

/// What a `Stage::Loading` frame is waiting to do once the LOADING screen has been shown for one
/// frame. `Song` loads a chart and enters Play; `Folder` (re)scans a newly-picked song folder and
/// re-matches the difficulty tables before returning to Select. Both block, so the LOADING frame
/// is presented first (folder descent within the already-scanned library is instant and skips this).
enum Loading {
    Song(usize),
    Folder(PathBuf),
}

/// Result of a background folder scan (run off-thread so the LOADING screen keeps animating instead
/// of freezing): the rescanned library and the per-table matched levels.
struct ScanOutcome {
    songs: Vec<SongEntry>,
    names: Vec<String>,
    levels: Vec<Vec<(String, Vec<usize>)>>,
}

/// One editable row in the key-config screen.
enum KcRow {
    ModeSelect,
    Control(ControlAction),
    Lane(usize),
}

fn kc_rows(edit_mode: Mode) -> Vec<KcRow> {
    let mut rows = vec![KcRow::ModeSelect];
    rows.extend(ControlAction::ALL.into_iter().map(KcRow::Control));
    rows.extend((0..edit_mode.key).map(KcRow::Lane));
    rows
}

/// Where the song-select browser currently is. Navigation is Root → (ALL SONGS | each table →
/// per-level folder) → charts, modelling beatoraja's table-as-custom-folder browsing. The
/// indices select a table and a level within `App::table_levels`.
#[derive(Clone, Copy, PartialEq)]
enum SelectView {
    Root,
    AllSongs,
    TableLevels(usize),
    TableLevel(usize, usize),
}

/// One row in the select list: either a folder to descend into, or a playable chart (index
/// into `App::songs`).
enum SelectItem {
    Song(usize),
    Folder { label: String, target: SelectView },
}

/// Cache key for the assembled [`SelectScene`]: rebuild only when one of these changes, so the scene
/// is not re-allocated every frame of the continuous redraw loop. `select_gen` bumps on any list
/// rebuild (catches same-length folder swaps); `scores` length catches a freshly saved record.
type SelectKey = (u64, usize, Option<usize>, usize, bool);

/// A clickable region recorded during rendering and hit-tested on a left-click. Immediate-mode:
/// `App::hot` is rebuilt every frame for the current stage, so the layout math lives in one place.
#[derive(Clone, Copy)]
enum Hot {
    SelectRow(usize),
    RecordRow(usize),
    SettingTab(usize),
    SettingRow(usize),
    ModalClose,
    ModalReplay,
}

/// Join a fetched difficulty table to the local library by md5, producing the owned charts
/// grouped by table level (in the table's level order, empty levels dropped).
fn compute_table_levels(songs: &[SongEntry], table: &rbms_table::DifficultyTable) -> Vec<(String, Vec<usize>)> {
    let mut by_md5: std::collections::HashMap<String, Vec<usize>> = std::collections::HashMap::new();
    for (i, s) in songs.iter().enumerate() {
        by_md5.entry(s.md5.to_ascii_lowercase()).or_default().push(i);
    }
    let mut out = Vec::new();
    for (level, entry_idxs) in table.by_level() {
        let mut idxs = Vec::new();
        for ei in entry_idxs {
            if let Some(v) = by_md5.get(&table.entries[ei].md5.to_ascii_lowercase()) {
                idxs.extend(v.iter().copied());
            }
        }
        idxs.sort_unstable();
        idxs.dedup();
        if !idxs.is_empty() {
            out.push((level, idxs));
        }
    }
    out
}

/// The font is ASCII-only, so non-ASCII table names (e.g. Japanese) can't render — fall back
/// to a generic ASCII label for those.
fn ascii_table_name(name: &str) -> String {
    let t = name.trim();
    if !t.is_empty() && t.is_ascii() { t.to_ascii_uppercase() } else { "DIFFICULTY TABLE".into() }
}

/// Stable per-URL filename for the on-disk table cache.
fn url_hash(s: &str) -> String {
    let mut h: u64 = 5381;
    for b in s.bytes() {
        h = h.wrapping_mul(33) ^ b as u64;
    }
    format!("{h:016x}")
}

/// Load a table from a `location`: an http(s) URL (fetched + cached) or a local `data.json` body file.
fn load_table_source(location: &str) -> Result<rbms_table::DifficultyTable, String> {
    if location.starts_with("http://") || location.starts_with("https://") {
        let cache = std::env::temp_dir().join(format!("rbms-table-{}.json", url_hash(location)));
        rbms_table::DifficultyTable::fetch_or_cache(location, &cache)
    } else {
        let bytes = std::fs::read(location).map_err(|e| e.to_string())?;
        rbms_table::DifficultyTable::from_body_bytes(&bytes, None)
    }
}

/// ASCII display name for a loaded table: the user's source name if usable, else the table's
/// own (header) name, else a generic label (the font is ASCII-only).
fn pick_table_name(src: &TableSource, table: &rbms_table::DifficultyTable) -> String {
    let s = src.name.trim();
    if !s.is_empty() && s.is_ascii() {
        return s.to_ascii_uppercase();
    }
    ascii_table_name(&table.name)
}

/// Display name for a source whose table failed to load (no header available).
fn fallback_source_name(src: &TableSource) -> String {
    let s = src.name.trim();
    if !s.is_empty() && s.is_ascii() { s.to_ascii_uppercase() } else { "TABLE".into() }
}

/// Load one table source and match it against the library. A failed load yields the source's
/// fallback name and an empty level set (kept so the parallel vecs stay aligned with sources).
fn load_and_match(src: &TableSource, songs: &[SongEntry]) -> (String, Vec<(String, Vec<usize>)>) {
    println!("loading table: {} ({})", src.name, src.location);
    match load_table_source(&src.location) {
        Ok(t) => {
            let lv = compute_table_levels(songs, &t);
            let owned: usize = lv.iter().map(|(_, v)| v.len()).sum();
            println!("  {} entries, matched {owned} local charts across {} levels", t.entries.len(), lv.len());
            (pick_table_name(src, &t), lv)
        }
        Err(e) => {
            eprintln!("  table load failed: {e}");
            (fallback_source_name(src), Vec::new())
        }
    }
}

/// Load every table source (one entry per source, aligned with `sources`). Returns parallel
/// `(display name, per-level owned-chart groups)` vecs.
fn fetch_and_match(sources: &[TableSource], songs: &[SongEntry]) -> (Vec<String>, Vec<Vec<(String, Vec<usize>)>>) {
    let mut names = Vec::new();
    let mut levels = Vec::new();
    for src in sources {
        let (name, lv) = load_and_match(src, songs);
        names.push(name);
        levels.push(lv);
    }
    (names, levels)
}

const GAUGE_CYCLE: [GaugeKind; 6] = [GaugeKind::AssistEasy, GaugeKind::Easy, GaugeKind::Normal, GaugeKind::Hard, GaugeKind::ExHard, GaugeKind::Hazard];
const SETTING_KEYCONFIG: usize = 11;
const SETTING_FONT: usize = 18;

/// Settings grouped into tabs by category. Each entry is `(tab name, [global setting indices])`
/// where the indices map to `setting_line`/`adjust_setting`.
const SETTING_TABS: &[(&str, &[usize])] = &[
    ("PLAY", &[0, 1, 2, 3, 16]),
    ("GAUGE", &[4, 13]),
    ("JUDGE", &[9, 12, 15]),
    ("DISPLAY", &[14, 18, 19, 20, 21, 5, 6, 10, 17]),
    ("INPUT", &[7, 8, 11]),
];

fn gauge_name(g: GaugeKind) -> &'static str {
    match g {
        GaugeKind::AssistEasy => "ASSIST EASY",
        GaugeKind::Easy => "EASY",
        GaugeKind::Normal => "NORMAL",
        GaugeKind::Hard => "HARD",
        GaugeKind::ExHard => "EX-HARD",
        GaugeKind::Hazard => "HAZARD",
    }
}

fn mode_color(mode: Mode) -> Color {
    match mode.key {
        6 => Color::GREEN,
        8 => Color::BLUE,
        9 => Color::rgb(230, 120, 200),
        16 => Color::ORANGE,
        _ => Color::GRAY,
    }
}

/// Format an epoch-millis timestamp as `YYYY-MM-DD HH:MM` (UTC) for the record list. Uses
/// Hinnant's civil-from-days algorithm so no calendar crate is needed.
fn fmt_datetime(ms: i64) -> String {
    let secs = ms.div_euclid(1000);
    let (days, tod) = (secs.div_euclid(86400), secs.rem_euclid(86400));
    let (hh, mm) = (tod / 3600, (tod % 3600) / 60);
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = era * 400 + yoe + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02} {hh:02}:{mm:02}")
}

/// BMS `#DIFFICULTY` slot name (1–5), the IIDX-style difficulty label.
fn difficulty_name(d: i32) -> &'static str {
    match d {
        1 => "BEGINNER",
        2 => "NORMAL",
        3 => "HYPER",
        4 => "ANOTHER",
        5 => "INSANE",
        _ => "—",
    }
}

/// `#DIFFICULTY` slot colour (BEGINNER→INSANE), the IIDX/LR2 difficulty palette used for the level badge.
fn difficulty_color(d: i32) -> Color {
    match d {
        1 => Color::rgb(80, 220, 120),
        2 => Color::rgb(90, 180, 240),
        3 => Color::rgb(240, 200, 70),
        4 => Color::rgb(240, 90, 90),
        5 => Color::rgb(200, 120, 230),
        _ => Color::GRAY,
    }
}

/// `#RANK` as a name + judge-width percent (beatoraja: 0 VERY HARD … 4 VERY EASY; `#RANK 2` = NORMAL = 75%).
fn rank_label(rank: i32) -> String {
    let name = match rank {
        0 => "VERY HARD",
        1 => "HARD",
        2 => "NORMAL",
        3 => "EASY",
        4 => "VERY EASY",
        _ => "?",
    };
    format!("{name} {}%", rbms_judge::rank_to_judgerank(rank))
}

/// A µs duration as `m:ss`.
fn fmt_duration(us: i64) -> String {
    let secs = (us / 1_000_000).max(0);
    format!("{}:{:02}", secs / 60, secs % 60)
}

fn mode_short(mode: Mode) -> &'static str {
    match mode.key {
        6 => "5K",
        8 => "7K",
        9 => "9K",
        12 => "10K",
        16 => "14K",
        _ => "?",
    }
}

/// beatoraja `ClearType` id (`ClearType.java`) for a lamp — persisted in score records so the
/// lamp round-trips. (LightAssistEasy=3 is unused; rbms has no separate light-assist lamp.)
fn clear_type_id(c: ClearType) -> u8 {
    match c {
        ClearType::NoPlay => 0,
        ClearType::Failed => 1,
        ClearType::AssistEasy => 2,
        ClearType::Easy => 4,
        ClearType::Normal => 5,
        ClearType::Hard => 6,
        ClearType::ExHard => 7,
        ClearType::FullCombo => 8,
        ClearType::Perfect => 9,
        ClearType::Max => 10,
    }
}

fn clear_type_from_id(id: u8) -> ClearType {
    match id {
        1 => ClearType::Failed,
        2 | 3 => ClearType::AssistEasy,
        4 => ClearType::Easy,
        5 => ClearType::Normal,
        6 => ClearType::Hard,
        7 => ClearType::ExHard,
        8 => ClearType::FullCombo,
        9 => ClearType::Perfect,
        10 => ClearType::Max,
        _ => ClearType::NoPlay,
    }
}

/// Clear-lamp label + colour. Colours are beatoraja's official lamp palette
/// (`select/SkinDistributionGraph.LAMP`, ARGB → RGB).
fn clear_label_color(c: ClearType) -> (&'static str, Color) {
    match c {
        ClearType::NoPlay => ("NO PLAY", Color::rgb(64, 64, 64)),
        ClearType::Failed => ("FAILED", Color::rgb(0, 0, 128)),
        ClearType::AssistEasy => ("ASSIST EASY", Color::rgb(128, 0, 128)),
        ClearType::Easy => ("EASY", Color::rgb(64, 255, 64)),
        ClearType::Normal => ("CLEAR", Color::rgb(0, 192, 240)),
        ClearType::Hard => ("HARD", Color::rgb(255, 255, 255)),
        ClearType::ExHard => ("EX-HARD", Color::rgb(136, 255, 255)),
        ClearType::FullCombo => ("FULL COMBO", Color::rgb(255, 255, 136)),
        ClearType::Perfect => ("PERFECT", Color::rgb(136, 136, 255)),
        ClearType::Max => ("MAX", Color::rgb(0, 0, 255)),
    }
}

struct App {
    chart_path: String,
    autoplay: bool,
    config: PlayerConfig,
    mode: Mode,
    stage: Stage,
    songs: Vec<SongEntry>,
    active_keys: Vec<(KeyCode, usize)>,
    table_sources: Vec<TableSource>,
    table_names: Vec<String>,
    table_levels: Vec<Vec<(String, Vec<usize>)>>,
    tables_path: PathBuf,
    tables_sel: usize,
    text_input: Option<String>,
    select_view: SelectView,
    select_items: Vec<SelectItem>,
    sel: usize,
    set_tab: usize,
    set_sel: usize,
    pending: Option<Loading>,
    /// When set, a background folder scan is running; the LOADING screen polls it each frame.
    scan_rx: Option<std::sync::mpsc::Receiver<ScanOutcome>>,
    loading_drawn: bool,
    keyconfig: KeyConfig,
    keyconfig_path: PathBuf,
    settings_path: PathBuf,
    seed: u64,
    recording: Vec<ReplayEvent>,
    replay: Option<Replay>,
    replay_cursor: usize,
    cal_sum_us: i64,
    cal_count: u32,
    kc_edit_mode: Mode,
    kc_sel: usize,
    kc_capturing: bool,
    kc_warn: bool,
    result: Option<ResultView>,
    gpu: Option<Gpu>,
    audio: Option<AudioEngine>,
    player: Option<Player>,
    skin: Skin,
    skin_cfg: SkinConfig,
    bga_images: std::collections::HashMap<i32, Vec<u8>>,
    bga_events: Vec<(i64, i32)>,
    bga_cursor: usize,
    cur_bga: i32,
    server: Arc<dyn ScoreServer>,
    server_connected: Arc<AtomicBool>,
    clock: Instant,
    anchor_us: i64,
    scores: ScoreBook,
    scores_path: PathBuf,
    /// When the records modal is open, the index into the focused chart's record list (newest
    /// first) currently shown in detail.
    record_modal: Option<usize>,
    /// Lazily computed detail (notes/LN/length/BPM range) for the currently focused song, with the
    /// song index it was computed for — recomputed only when the focus moves to a different song.
    focused_detail: Option<ChartDetail>,
    focused_detail_si: Option<usize>,
    cover_rgba: Option<Vec<u8>>,
    cached_select: Option<SelectScene>,
    cached_select_key: Option<SelectKey>,
    select_gen: u64,
    preview_audio: Option<AudioEngine>,
    preview_si: Option<usize>,
    preview_target: Option<usize>,
    preview_target_frame: u64,
    preview_loop_us: i64,
    preview_next_us: i64,
    cursor: (f32, f32),
    hot: Vec<(Rect, Hot)>,
    last_frame: Instant,
    fps: f32,
    frame_count: u64,
    ram_mb: f32,
    /// SHA-256 of this binary, computed once at startup, submitted for build integrity (F8).
    build_sha256: Option<String>,
    // --- replay analysis mode (F4) ---
    /// Active when a replay is playing and REPLAY ANALYSIS is on: shows the analysis overlay and
    /// enables playback controls.
    analysis: bool,
    /// Once the user takes manual control (pause/seek/rate), playback runs off the virtual
    /// `analysis_us` clock and keysounds are muted (so scrubbing/slow-mo doesn't desync audio);
    /// until then it follows the real (audio) clock at 1× with sound.
    analysis_manual: bool,
    analysis_paused: bool,
    analysis_rate: f64,
    analysis_us: i64,
    /// Recent judged inputs `(lane, delta_us, judge)` for the per-note ms-off overlay (newest last).
    msoff: Vec<(usize, i64, u8)>,
}

impl App {
    fn new(input: String, autoplay: bool, mut config: PlayerConfig, settings_path: PathBuf) -> Self {
        let replay = config.replay_path.as_ref().and_then(|p| match Replay::load(Path::new(p)) {
            Ok(r) => {
                println!("replay: {} on {}", p, r.chart_path);
                Some(r)
            }
            Err(e) => {
                eprintln!("replay load failed: {e}");
                None
            }
        });
        let autoplay = if replay.is_some() { false } else { autoplay };

        let p = Path::new(&input);
        let is_dir = replay.is_none() && p.is_dir();
        // Remember a folder launch so the next bare launch reopens it (persisted below if changed).
        let folder_for_save = is_dir.then(|| input.clone());
        let prev_folder = config.songs_folder.clone();
        let (songs, stage, chart_path) = if let Some(rp) = &replay {
            (Vec::new(), Stage::Play, rp.chart_path.clone())
        } else if is_dir {
            let songs = scan_folder(p);
            println!("scanned {} charts under {input} — ↑/↓ select, Enter open, Esc back", songs.len());
            (songs, Stage::Select, String::new())
        } else if p.is_file() {
            (Vec::new(), Stage::Play, input)
        } else {
            // A bad/stale path (e.g. a remembered folder since deleted): recover to an empty select
            // screen (press O to pick a folder) instead of trying to play it and quitting.
            eprintln!("path not found: {input} — opening an empty select screen (press O to choose a folder)");
            (Vec::new(), Stage::Select, String::new())
        };
        if let Some(f) = &folder_for_save {
            config.songs_folder = Some(f.clone());
        }

        let tables_path = settings_path.parent().map(|d| d.join("tables.ron")).unwrap_or_else(|| PathBuf::from("tables.ron"));
        let scores_path = settings_path.parent().map(|d| d.join("scores.ron")).unwrap_or_else(|| PathBuf::from("scores.ron"));
        let scores = ScoreBook::load(&scores_path);
        let mut table_sources = TableList::load(&tables_path).tables;
        if let Some(url) = &config.table_url {
            if !table_sources.iter().any(|t| &t.location == url) {
                table_sources.push(TableSource { name: String::new(), location: url.clone() });
            }
        }
        let (table_names, table_levels) = if songs.is_empty() { (Vec::new(), Vec::new()) } else { fetch_and_match(&table_sources, &songs) };

        let server: Arc<dyn ScoreServer> = match &config.server_url {
            Some(url) => {
                println!("score server: {url}");
                Arc::new(rbms_ir::HttpScoreServer::new(url.clone(), None))
            }
            None => Arc::new(NullScoreServer),
        };
        let server_connected = Arc::new(AtomicBool::new(false));
        if config.server_url.is_some() {
            let server = server.clone();
            let connected = server_connected.clone();
            std::thread::spawn(move || {
                loop {
                    connected.store(server.health().is_ok(), Ordering::Relaxed);
                    std::thread::sleep(Duration::from_secs(5));
                }
            });
        }

        let keyconfig_path = config.keyconfig_path.clone().map(PathBuf::from).unwrap_or_else(|| {
            let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
            home.join(".config/rbms/keyconfig.ron")
        });
        let keyconfig = KeyConfig::load(&keyconfig_path);

        let mut app = App {
            chart_path,
            autoplay,
            config,
            mode: MODE,
            stage,
            songs,
            active_keys: keyconfig.lane_keys(MODE),
            table_sources,
            table_names,
            table_levels,
            tables_path,
            tables_sel: 0,
            text_input: None,
            select_view: SelectView::Root,
            select_items: Vec::new(),
            sel: 0,
            set_tab: 0,
            set_sel: 0,
            pending: None,
            scan_rx: None,
            loading_drawn: false,
            keyconfig,
            keyconfig_path,
            settings_path,
            seed: 1,
            recording: Vec::new(),
            replay,
            replay_cursor: 0,
            cal_sum_us: 0,
            cal_count: 0,
            kc_edit_mode: MODE,
            kc_sel: 0,
            kc_capturing: false,
            kc_warn: false,
            result: None,
            gpu: None,
            audio: None,
            player: None,
            skin: Skin::default_for(MODE, CW as f32, CH as f32),
            skin_cfg: SkinConfig::default(),
            bga_images: std::collections::HashMap::new(),
            bga_events: Vec::new(),
            bga_cursor: 0,
            cur_bga: -1,
            server,
            server_connected,
            clock: Instant::now(),
            anchor_us: 0,
            scores,
            scores_path,
            record_modal: None,
            focused_detail: None,
            focused_detail_si: None,
            cover_rgba: None,
            cached_select: None,
            cached_select_key: None,
            select_gen: 0,
            preview_audio: None,
            preview_si: None,
            preview_target: None,
            preview_target_frame: 0,
            preview_loop_us: 0,
            preview_next_us: 0,
            cursor: (0.0, 0.0),
            hot: Vec::new(),
            last_frame: Instant::now(),
            fps: 0.0,
            frame_count: 0,
            ram_mb: 0.0,
            build_sha256: compute_build_hash(),
            analysis: false,
            analysis_manual: false,
            analysis_paused: false,
            analysis_rate: 1.0,
            analysis_us: 0,
            msoff: Vec::new(),
        };
        app.rebuild_select_items();
        if folder_for_save.is_some() && folder_for_save != prev_folder {
            app.save_settings();
        }
        app
    }

    /// Recompute the visible select list for the current `select_view`.
    fn rebuild_select_items(&mut self) {
        let n = self.songs.len();
        self.select_items = match self.select_view {
            SelectView::Root => {
                let mut items = vec![SelectItem::Folder { label: format!("ALL SONGS ({n})"), target: SelectView::AllSongs }];
                for (ti, levels) in self.table_levels.iter().enumerate() {
                    if levels.is_empty() {
                        continue;
                    }
                    let total: usize = levels.iter().map(|(_, v)| v.len()).sum();
                    let name = self.table_names.get(ti).cloned().unwrap_or_else(|| "TABLE".into());
                    items.push(SelectItem::Folder { label: format!("{name} ({total})"), target: SelectView::TableLevels(ti) });
                }
                items
            }
            SelectView::AllSongs => (0..n).map(SelectItem::Song).collect(),
            SelectView::TableLevels(ti) => self
                .table_levels
                .get(ti)
                .map(|levels| {
                    levels
                        .iter()
                        .enumerate()
                        .map(|(li, (level, songs))| SelectItem::Folder { label: format!("LV {level} ({})", songs.len()), target: SelectView::TableLevel(ti, li) })
                        .collect()
                })
                .unwrap_or_default(),
            SelectView::TableLevel(ti, li) => self
                .table_levels
                .get(ti)
                .and_then(|levels| levels.get(li))
                .map(|(_, songs)| songs.iter().copied().map(SelectItem::Song).collect())
                .unwrap_or_default(),
        };
        if self.sel >= self.select_items.len() {
            self.sel = self.select_items.len().saturating_sub(1);
        }
        self.select_gen = self.select_gen.wrapping_add(1);
    }

    fn lane_for(&self, code: KeyCode) -> Option<usize> {
        self.active_keys.iter().find(|(k, _)| *k == code).map(|(_, l)| *l)
    }

    /// Which configured in-play control (if any) a key triggers.
    fn control_for(&self, code: KeyCode) -> Option<ControlAction> {
        ControlAction::ALL.into_iter().find(|a| self.keyconfig.control_key(*a) == Some(code))
    }

    /// Apply an in-play control: hi-speed, lane cover (sudden) and lift, clamped to sane ranges.
    fn apply_control(&mut self, action: ControlAction) {
        match action {
            ControlAction::HiSpeedUp => self.config.hispeed = (self.config.hispeed + 0.25).clamp(0.5, 10.0),
            ControlAction::HiSpeedDown => self.config.hispeed = (self.config.hispeed - 0.25).clamp(0.5, 10.0),
            ControlAction::CoverUp => self.config.cover = (self.config.cover + 0.05).clamp(0.0, 0.9),
            ControlAction::CoverDown => self.config.cover = (self.config.cover - 0.05).clamp(0.0, 0.9),
            ControlAction::LiftUp => {
                self.config.lift = (self.config.lift + 0.05).clamp(0.0, 0.9);
                self.rebuild_skin();
            }
            ControlAction::LiftDown => {
                self.config.lift = (self.config.lift - 0.05).clamp(0.0, 0.9);
                self.rebuild_skin();
            }
        }
    }

    fn current_settings(&self) -> PlaySettings {
        PlaySettings {
            hispeed: self.config.hispeed,
            gauge: gauge_token(self.config.gauge).to_string(),
            lift: self.config.lift,
            cover: self.config.cover,
            scratch_left: self.config.scratch_left,
            scratch_auto: self.config.scratch_auto,
            autoplay: self.autoplay,
            random: self.config.random.label().to_string(),
            constant_speed: self.config.constant_speed,
            offset_ms: self.config.offset_ms,
            auto_offset: self.config.auto_offset,
            judge_rate: self.config.judge_rate,
            total_override: self.config.total_override,
            bga: self.config.bga,
            skin: self.config.skin_name.clone(),
            auto_replay: self.config.auto_replay,
            debug: self.config.debug,
            font_path: self.config.font_path.clone(),
            score_graph: self.config.score_graph,
            replay_analysis: self.config.replay_analysis,
            preview: self.config.preview,
            songs_folder: self.config.songs_folder.clone(),
        }
    }

    fn save_settings(&self) {
        self.current_settings().save(&self.settings_path);
    }

    /// Open a native picker for a UI font (TTF/OTF/TTC), load it live as the preferred family,
    /// and persist the path so it is reapplied next launch.
    fn pick_font(&mut self) {
        if let Some(path) = rfd::FileDialog::new().add_filter("font", &["ttf", "otf", "ttc"]).set_title("Select UI font").pick_file() {
            match std::fs::read(&path) {
                Ok(bytes) => match rbms_render::load_font(bytes) {
                    Some(family) => {
                        rbms_render::set_ui_family(&family);
                        self.config.font_path = Some(path.to_string_lossy().to_string());
                        self.save_settings();
                        println!("font: {} ({family})", path.display());
                    }
                    None => eprintln!("font load failed (no usable face): {}", path.display()),
                },
                Err(e) => eprintln!("font read failed ({}): {e}", path.display()),
            }
        }
    }

    /// Revert the UI font to the bundled default.
    fn reset_font(&mut self) {
        rbms_render::reset_ui_family();
        self.config.font_path = None;
        self.save_settings();
    }

    fn offset_us(&self) -> i64 {
        self.config.offset_ms as i64 * 1000
    }

    /// Feed all recorded inputs whose raw time has been reached, judging them at the recorded
    /// time plus the (replay's) offset — reproducing the original run. `mute` skips keysounds (used
    /// during analysis scrubbing/slow-mo, where the virtual clock would desync audio). Each judged
    /// input's timing delta is captured for the analysis ms-off overlay.
    fn feed_replay(&mut self, song: i64, anchor: i64, mute: bool) {
        let off = self.offset_us();
        loop {
            let ev = match self.replay.as_ref() {
                Some(rp) if self.replay_cursor < rp.events.len() => rp.events[self.replay_cursor],
                _ => break,
            };
            if ev.t > song {
                break;
            }
            self.replay_cursor += 1;
            let res = if ev.press {
                if let (false, Some(audio)) = (mute, self.audio.as_mut()) {
                    let play = |e: PlayEvent| audio.play(e.wav.max(0) as u32, 1.0, 0.0, 1.0, e.at_us + anchor);
                    self.player.as_mut().and_then(|p| p.press(ev.lane, ev.t + off, play))
                } else {
                    self.player.as_mut().and_then(|p| p.press(ev.lane, ev.t + off, |_| {}))
                }
            } else {
                self.player.as_mut().and_then(|p| p.release(ev.lane, ev.t + off))
            };
            if let Some(r) = res {
                self.msoff.push((r.lane, r.delta_us, r.judge as u8));
                if self.msoff.len() > 16 {
                    self.msoff.remove(0);
                }
            }
        }
    }

    /// Rebuild the replay's judge state at an arbitrary song time by re-simulating the recorded
    /// inputs from the start up to `target_us`. Used by analysis seek so judging stays exactly
    /// correct (no forward-sweep corruption) when jumping forwards or backwards.
    fn seek_replay(&mut self, target_us: i64) {
        if self.replay.is_none() {
            return;
        }
        let target = target_us.max(0);
        let Some(model) = self.player.as_ref().map(|p| p.model().clone()) else { return };
        let off = self.offset_us();
        let mut p = Player::new(model, false);
        p.set_gauge(self.config.gauge);
        p.set_judge_rate(self.config.judge_rate);
        if self.config.scratch_auto {
            let auto: Vec<bool> = (0..self.mode.key).map(|l| self.mode.is_scratch(l)).collect();
            p.set_auto_lanes(auto);
        }
        let mut cursor = 0;
        if let Some(rp) = &self.replay {
            for ev in &rp.events {
                if ev.t > target {
                    break;
                }
                if ev.press {
                    p.press(ev.lane, ev.t + off, |_| {});
                } else {
                    p.release(ev.lane, ev.t + off);
                }
                cursor += 1;
            }
        }
        p.update(target, |_| {});
        self.player = Some(p);
        self.replay_cursor = cursor;
        self.analysis_us = target;
        self.analysis_manual = true;
        self.msoff.clear();
    }

    /// Handle an analysis-mode playback key (only while a replay analysis is active): pause/resume,
    /// playback rate, and ±2 s seek. Returns whether the key was an analysis control.
    fn analysis_key(&mut self, code: KeyCode) -> bool {
        if !self.analysis {
            return false;
        }
        match code {
            KeyCode::Space => {
                self.analysis_manual = true;
                self.analysis_paused = !self.analysis_paused;
            }
            KeyCode::Equal => {
                self.analysis_manual = true;
                self.analysis_rate = (self.analysis_rate + 0.25).min(4.0);
            }
            KeyCode::Minus => {
                self.analysis_manual = true;
                self.analysis_rate = (self.analysis_rate - 0.25).max(0.25);
            }
            KeyCode::PageUp => self.seek_replay(self.analysis_us + 2_000_000),
            KeyCode::PageDown => self.seek_replay(self.analysis_us - 2_000_000),
            _ => return false,
        }
        true
    }

    /// Rebuild the resolved skin from the loaded base config plus the live scratch-side/lift.
    fn rebuild_skin(&mut self) {
        let mut cfg = self.skin_cfg.clone();
        cfg.scratch_left = self.config.scratch_left;
        cfg.lift = self.config.lift;
        self.skin = Skin::build(&cfg, self.mode, CW as f32, CH as f32);
    }

    fn enter_keyconfig(&mut self) {
        self.kc_sel = 0;
        self.kc_capturing = false;
        self.stage = Stage::KeyConfig;
    }

    fn cycle_edit_mode(&mut self, d: i32) {
        let all = Mode::ALL;
        let cur = all.iter().position(|m| m.key == self.kc_edit_mode.key).unwrap_or(0);
        self.kc_edit_mode = all[((cur as i32 + d).rem_euclid(all.len() as i32)) as usize];
        let len = kc_rows(self.kc_edit_mode).len();
        if self.kc_sel >= len {
            self.kc_sel = len - 1;
        }
    }

    /// Whether binding `code` to `row` (for `mode`) would collide with another action and so
    /// silently break it (controls are resolved before lanes in play, so a shared key would
    /// shadow the lane). Rebinding a row to its own current key is not a collision.
    fn binding_collides(&self, mode: Mode, row: &KcRow, code: KeyCode) -> bool {
        match row {
            KcRow::Lane(lane) => {
                self.control_for(code).is_some() || self.keyconfig.lane_keys(mode).iter().any(|(k, l)| *k == code && l != lane)
            }
            KcRow::Control(action) => {
                self.keyconfig.lane_keys(mode).iter().any(|(k, _)| *k == code)
                    || ControlAction::ALL.into_iter().any(|a| a != *action && self.keyconfig.control_key(a) == Some(code))
            }
            KcRow::ModeSelect => false,
        }
    }

    /// Key-config editor input. In capture mode the next key (except Esc) is bound to the
    /// focused row unless it collides with another action; otherwise navigate, switch
    /// edit-mode, start a rebind, or save+exit.
    fn keyconfig_input(&mut self, code: KeyCode) {
        let rows = kc_rows(self.kc_edit_mode);
        if self.kc_capturing {
            if code != KeyCode::Escape {
                if let Some(row) = rows.get(self.kc_sel) {
                    if self.binding_collides(self.kc_edit_mode, row, code) {
                        self.kc_warn = true;
                        self.kc_capturing = false;
                        return;
                    }
                    match row {
                        KcRow::Control(a) => self.keyconfig.set_control(*a, code),
                        KcRow::Lane(lane) => self.keyconfig.set_lane(self.kc_edit_mode, *lane, code),
                        KcRow::ModeSelect => {}
                    }
                }
            }
            self.kc_warn = false;
            self.kc_capturing = false;
            return;
        }
        self.kc_warn = false;
        match code {
            KeyCode::Escape => {
                self.keyconfig.save(&self.keyconfig_path);
                self.stage = Stage::Settings;
            }
            KeyCode::ArrowUp => self.kc_sel = self.kc_sel.saturating_sub(1),
            KeyCode::ArrowDown => self.kc_sel = (self.kc_sel + 1).min(rows.len().saturating_sub(1)),
            KeyCode::ArrowLeft if matches!(rows.get(self.kc_sel), Some(KcRow::ModeSelect)) => self.cycle_edit_mode(-1),
            KeyCode::ArrowRight if matches!(rows.get(self.kc_sel), Some(KcRow::ModeSelect)) => self.cycle_edit_mode(1),
            KeyCode::Enter | KeyCode::NumpadEnter => {
                if matches!(rows.get(self.kc_sel), Some(KcRow::Control(_)) | Some(KcRow::Lane(_))) {
                    self.kc_capturing = true;
                }
            }
            _ => {}
        }
    }

    /// Enter the focused select item: descend into a folder, or start a chart.
    fn select_enter(&mut self) {
        match self.select_items.get(self.sel) {
            Some(SelectItem::Song(i)) => {
                let i = *i;
                self.begin_loading(Loading::Song(i));
            }
            Some(SelectItem::Folder { target, .. }) => {
                self.select_view = *target;
                self.sel = 0;
                self.rebuild_select_items();
            }
            None => {}
        }
    }

    /// md5 of the currently focused chart, if a song (not a folder) is focused.
    fn focused_md5(&self) -> Option<String> {
        match self.select_items.get(self.sel) {
            Some(SelectItem::Song(si)) => self.songs.get(*si).map(|e| e.md5.clone()),
            _ => None,
        }
    }

    /// Index into `songs` of the focused select row, if it is a chart (not a folder).
    fn focused_song_index(&self) -> Option<usize> {
        match self.select_items.get(self.sel) {
            Some(SelectItem::Song(si)) => Some(*si),
            _ => None,
        }
    }

    /// Map a local score record into the renderer's record-row view (owned data, no borrow escapes).
    /// `trend` carries the EX delta versus the next-older play when the score graph is enabled.
    fn record_row_view(&self, r: &ScoreRecord, older: Option<&ScoreRecord>) -> RecordRowView {
        let (label, color) = clear_label_color(clear_type_from_id(r.clear));
        let trend = older.filter(|_| self.config.score_graph).map(|o| ex_delta_label(r.ex_score as i64 - o.ex_score as i64));
        RecordRowView { when: fmt_datetime(r.played_at), lamp: color, lamp_label: label, ex: r.ex_score, max_ex: r.max_ex, bp: r.counts[3] + r.counts[4] + r.counts[5], trend }
    }

    fn select_key(&self) -> SelectKey {
        (self.select_gen, self.sel, self.record_modal, self.scores.records.len(), self.config.score_graph)
    }

    /// Rebuild the cached select scene only when [`select_key`] changes. `frame()` runs a continuous
    /// redraw loop, so rebuilding every frame would re-walk + re-allocate the whole song list. Kept as a
    /// `&mut self` step (separate from borrowing the field) so the cached scene can be read disjointly
    /// from the GPU during render.
    fn refresh_select_cache(&mut self) {
        let key = self.select_key();
        if self.cached_select_key != Some(key) || self.cached_select.is_none() {
            let scene = self.build_select_view();
            self.cached_select = Some(scene);
            self.cached_select_key = Some(key);
        }
    }

    /// Assemble the backend-agnostic [`SelectScene`] from the current select state. Built before the
    /// GPU borrow so the render pass can stay a pure data → pixels call (the cover texture is uploaded
    /// separately into [`cover_rect`]).
    fn build_select_view(&self) -> SelectScene {
        let rows = self
            .select_items
            .iter()
            .map(|item| match item {
                SelectItem::Folder { label, .. } => SelectRow {
                    folder: true,
                    title: label.clone(),
                    mode_short: "",
                    mode_color: Color::GRAY,
                    level: String::new(),
                    difficulty_color: Color::GRAY,
                    lamp: Color::rgb(44, 44, 54),
                    folder_count: None,
                },
                SelectItem::Song(si) => {
                    let e = &self.songs[*si];
                    let lamp = self.scores.best_clear_for_md5(&e.md5).map(|c| clear_label_color(clear_type_from_id(c)).1).unwrap_or(Color::rgb(44, 44, 54));
                    SelectRow {
                        folder: false,
                        title: e.title.clone(),
                        mode_short: mode_short(e.mode),
                        mode_color: mode_color(e.mode),
                        level: e.level.clone(),
                        difficulty_color: difficulty_color(e.difficulty),
                        lamp,
                        folder_count: None,
                    }
                }
            })
            .collect();

        let header = match self.select_view {
            SelectView::Root => self.config.songs_folder.as_deref().and_then(|p| Path::new(p).file_name()).and_then(|n| n.to_str()).unwrap_or("ROOT").to_string(),
            SelectView::AllSongs => "ALL SONGS".into(),
            SelectView::TableLevels(ti) => self.table_names.get(ti).cloned().unwrap_or_default(),
            SelectView::TableLevel(ti, li) => {
                let table = self.table_names.get(ti).cloned().unwrap_or_default();
                let level = self.table_levels.get(ti).and_then(|ls| ls.get(li)).map(|(l, _)| format!("LV {l}")).unwrap_or_default();
                format!("{table} / {level}")
            }
        };

        let detail = match self.select_items.get(self.sel) {
            Some(SelectItem::Song(si)) => {
                let e = &self.songs[*si];
                let d = self.focused_detail.as_ref();
                let bpm = match d {
                    Some(d) if (d.bpm_max - d.bpm_min).abs() >= 1.0 => format!("{}\u{2013}{}", d.bpm_min.round() as i32, d.bpm_max.round() as i32),
                    Some(d) => format!("{}", d.bpm_min.round() as i32),
                    None => format!("{}", e.init_bpm.round() as i32),
                };
                let notes = d.map(|d| if d.long_notes > 0 { format!("{} ({}LN)", d.notes, d.long_notes) } else { d.notes.to_string() }).unwrap_or_else(|| "\u{2026}".into());
                let length = d.map(|d| fmt_duration(d.duration_us)).unwrap_or_else(|| "\u{2026}".into());
                let total = if e.total > 0.0 { format!("{}", e.total.round() as i32) } else { "AUTO".into() };
                let genre_maker = match (e.genre.trim(), e.maker.trim()) {
                    ("", "") => String::new(),
                    ("", m) => m.to_string(),
                    (g, "") => g.to_string(),
                    (g, m) => format!("{g}  \u{00B7}  {m}"),
                };
                let density = d.filter(|d| !d.density.is_empty()).map(|d| DensityView { bins: d.density.clone(), peak: d.peak_density, avg: d.avg_density, end: d.end_density });

                let recs = self.scores.for_md5(&e.md5);
                let plays = recs.len();
                let clears = recs.iter().filter(|r| r.clear >= 2).count();
                let best = recs.iter().max_by(|a, b| a.clear.cmp(&b.clear).then(a.ex_score.cmp(&b.ex_score)));
                let rank_bar = best.filter(|_| self.config.score_graph).map(|b| (b.ex_score, b.max_ex));
                let best = best.map(|b| self.record_row_view(b, None));
                let recent = recs.iter().enumerate().map(|(ri, r)| self.record_row_view(r, recs.get(ri + 1).copied())).collect();

                SelectDetail::Song(Box::new(DetailView {
                    accent: mode_color(e.mode),
                    title: e.title.clone(),
                    subtitle: e.subtitle.clone(),
                    artist: e.artist.clone(),
                    genre_maker,
                    mode_short: mode_short(e.mode),
                    mode_color: mode_color(e.mode),
                    level: e.level.clone(),
                    difficulty_color: difficulty_color(e.difficulty),
                    difficulty_name: difficulty_name(e.difficulty),
                    cover: if self.cover_rgba.is_some() { CoverState::Present } else { CoverState::None },
                    stats: vec![
                        StatCell { label: "BPM", value: bpm },
                        StatCell { label: "DENSITY", value: d.map(|d| format!("{}/s", d.avg_density.round() as i32)).unwrap_or_else(|| "\u{2026}".into()) },
                        StatCell { label: "NOTES", value: notes },
                        StatCell { label: "JUDGE", value: rank_label(e.rank) },
                        StatCell { label: "LENGTH", value: length },
                        StatCell { label: "TOTAL", value: total },
                    ],
                    density,
                    records: RecordsView { plays, clears, best, rank_bar, recent },
                }))
            }
            Some(SelectItem::Folder { label, .. }) => SelectDetail::Folder { label: label.clone(), count: 0 },
            None => SelectDetail::Empty,
        };

        let modal = self.record_modal.and_then(|ri| {
            let si = match self.select_items.get(self.sel) {
                Some(SelectItem::Song(si)) => *si,
                _ => return None,
            };
            let e = &self.songs[si];
            let recs = self.scores.for_md5(&e.md5);
            let r = recs.get(ri)?;
            let (clear_label, clear_color) = clear_label_color(clear_type_from_id(r.clear));
            let (rank, rank_color) = if self.config.score_graph {
                let (name, col) = RANK_BANDS[dj_rank(r.ex_score, r.max_ex)];
                (Some(name), col)
            } else {
                (None, Color::WHITE)
            };
            Some(SelectModal {
                title: e.title.clone(),
                clear_label,
                clear_color,
                when: fmt_datetime(r.played_at),
                sub: format!("{}   {}   GAUGE {}", r.mode, r.random, r.gauge),
                counts: r.counts,
                ex: r.ex_score,
                max_ex: r.max_ex,
                rank,
                rank_color,
                max_combo: r.max_combo,
                total_notes: r.total_notes,
                bp: r.counts[3] + r.counts[4] + r.counts[5],
                empty_poor: r.empty_poor,
                gauge_value: r.gauge_value.round() as i32,
                index: ri,
                total: recs.len(),
                has_replay: r.replay_file.is_some(),
            })
        });

        SelectScene {
            rows,
            sel: self.sel,
            header,
            guide: "UP DOWN  LEFT BACK  RIGHT/ENTER OPEN  TAB SETTINGS  O FOLDER  T TABLES  R RECORDS",
            detail,
            modal,
            score_graph: self.config.score_graph,
        }
    }

    /// (Re)compute the focused chart's heavy detail (notes/LN/length/BPM range) only when the focus
    /// moves to a different song — so scrolling the list parses at most one chart per moved row.
    fn refresh_focused_detail(&mut self) {
        let si = self.focused_song_index();
        if si == self.focused_detail_si {
            return;
        }
        self.focused_detail_si = si;
        self.focused_detail = si.and_then(|i| self.songs.get(i)).and_then(|e| compute_chart_detail(&e.path, e.mode));
        // Decode the cover (#STAGEFILE, then #BANNER) for the detail panel; the texture is uploaded
        // into the single BGA slot during the Select render. One decode per moved row, like the detail.
        self.cover_rgba = si.and_then(|i| self.songs.get(i)).and_then(|e| {
            let dir = e.path.parent()?;
            [&e.stagefile, &e.banner].into_iter().filter(|n| !n.trim().is_empty()).find_map(|n| decode_bga_256(dir, n))
        });
    }

    /// Drive the `#PREVIEW` hover preview: once the focus settles on a song (debounced so fast
    /// scrolling doesn't decode every row), decode + loop its preview clip; gapless loop is kept by
    /// scheduling the next iteration ahead of the play clock. The preview audio engine is separate
    /// from the play engine (which only exists during Play) and lazily created on first need.
    fn update_preview(&mut self) {
        if !self.config.preview {
            if self.preview_audio.is_some() {
                self.stop_preview();
            }
            return;
        }
        let cur = self.focused_song_index();
        if cur != self.preview_target {
            self.preview_target = cur;
            self.preview_target_frame = self.frame_count;
            if self.preview_si.is_some() {
                if let Some(eng) = self.preview_audio.as_mut() {
                    eng.stop(PREVIEW_ID);
                }
                self.preview_si = None;
                self.preview_loop_us = 0;
            }
        }
        if self.preview_si != cur {
            if let Some(si) = cur {
                if self.frame_count.wrapping_sub(self.preview_target_frame) >= PREVIEW_DEBOUNCE_FRAMES {
                    self.start_preview(si);
                }
            }
            return;
        }
        // Loop by re-triggering at each clip boundary. (The mixer stops the same key on a new play, so
        // scheduling ahead would cut the current clip; re-triggering after it naturally ends is clean.)
        if self.preview_loop_us > 0 {
            if let Some(eng) = self.preview_audio.as_mut() {
                while eng.clock_us() >= self.preview_next_us {
                    eng.play(PREVIEW_ID, PREVIEW_GAIN, 0.0, 1.0, self.preview_next_us);
                    self.preview_next_us += self.preview_loop_us;
                }
            }
        }
    }

    /// Decode the focused song's `#PREVIEW` clip and start it looping. Marks the song as settled even
    /// when there is no preview file, so the debounce doesn't retry every frame.
    fn start_preview(&mut self, si: usize) {
        self.preview_si = Some(si);
        self.preview_loop_us = 0;
        let Some(e) = self.songs.get(si) else { return };
        if e.preview.trim().is_empty() {
            return;
        }
        let Some(dir) = e.path.parent() else { return };
        let Some((path, _)) = resolve_file(dir, &e.preview, &["wav", "ogg", "flac", "mp3"]) else { return };
        let Ok(bytes) = std::fs::read(&path) else { return };
        let ext = path.extension().and_then(|x| x.to_str()).map(str::to_owned);
        if self.preview_audio.is_none() {
            self.preview_audio = AudioEngine::new().ok();
        }
        let Some(eng) = self.preview_audio.as_mut() else { return };
        if eng.load(PREVIEW_ID, bytes, ext.as_deref()).is_err() {
            return;
        }
        let dur = eng.sample_duration_us(PREVIEW_ID).unwrap_or(0);
        let now = eng.clock_us();
        eng.play(PREVIEW_ID, PREVIEW_GAIN, 0.0, 1.0, now);
        self.preview_loop_us = dur;
        self.preview_next_us = now + dur.max(1);
    }

    /// Tear down the preview (drops the engine to release its cpal stream); recreated next time a
    /// preview is needed in select.
    fn stop_preview(&mut self) {
        self.preview_audio = None;
        self.preview_si = None;
        self.preview_target = None;
        self.preview_loop_us = 0;
    }

    /// Open the record-detail modal for the focused chart (newest record first), if it has any.
    fn open_record_modal(&mut self) {
        if let Some(md5) = self.focused_md5() {
            if !self.scores.for_md5(&md5).is_empty() {
                self.record_modal = Some(0);
            }
        }
    }

    /// Move the record-detail modal selection within the focused chart's records (clamped).
    fn record_modal_nav(&mut self, d: i32) {
        let Some(ri) = self.record_modal else { return };
        let n = self.focused_md5().map(|m| self.scores.for_md5(&m).len()).unwrap_or(0);
        if n == 0 {
            self.record_modal = None;
            return;
        }
        self.record_modal = Some((ri as i32 + d).clamp(0, n as i32 - 1) as usize);
    }

    /// Load and start the replay attached to the record currently shown in the modal.
    fn play_record_replay(&mut self) {
        let Some(ri) = self.record_modal else { return };
        let Some(md5) = self.focused_md5() else { return };
        let file = self.scores.for_md5(&md5).get(ri).and_then(|r| r.replay_file.clone());
        let Some(file) = file else { return };
        let dir = self.settings_path.parent().map(|d| d.join("replays")).unwrap_or_else(|| PathBuf::from("replays"));
        match Replay::load(&dir.join(&file)) {
            Ok(rp) => {
                self.record_modal = None;
                self.chart_path = rp.chart_path.clone();
                self.replay = Some(rp);
                self.replay_cursor = 0;
                self.result = None;
                self.loading_drawn = false;
                self.pending = None;
                self.stage = if self.load() { Stage::Play } else { Stage::Select };
            }
            Err(e) => {
                eprintln!("replay load failed: {e}");
                self.record_modal = None;
            }
        }
    }

    /// Hit-test the last frame's clickable regions against the cursor and dispatch a left-click.
    /// Regions are tested topmost-first (later pushes draw on top).
    fn handle_click(&mut self) {
        let (cx, cy) = self.cursor;
        let hit = self
            .hot
            .iter()
            .rev()
            .find(|(r, _)| cx >= r.x && cx <= r.x + r.w && cy >= r.y && cy <= r.y + r.h)
            .map(|(_, h)| *h);
        match hit {
            Some(Hot::SelectRow(idx)) => {
                if self.sel == idx {
                    self.select_enter();
                } else {
                    self.sel = idx;
                    self.record_modal = None;
                    self.print_selection();
                }
            }
            Some(Hot::RecordRow(ri)) => self.record_modal = Some(ri),
            Some(Hot::SettingTab(ti)) => {
                self.set_tab = ti.min(SETTING_TABS.len() - 1);
                self.set_sel = 0;
            }
            Some(Hot::SettingRow(i)) => {
                let items = SETTING_TABS[self.set_tab].1;
                if let Some(&g) = items.get(i) {
                    self.set_sel = i;
                    if g == SETTING_KEYCONFIG {
                        self.enter_keyconfig();
                    } else {
                        self.adjust_setting(g, 1);
                    }
                }
            }
            Some(Hot::ModalReplay) => self.play_record_replay(),
            Some(Hot::ModalClose) => self.record_modal = None,
            // A click that misses every region closes an open modal (click-outside-to-dismiss).
            None => self.record_modal = None,
        }
    }

    /// Open a native folder picker, then scan the chosen folder on a background thread while the
    /// LOADING screen animates — so the UI never freezes during the (recursive) scan + table fetch.
    fn open_folder_dialog(&mut self) {
        if let Some(dir) = rfd::FileDialog::new().set_title("Select song folder").pick_folder() {
            self.begin_folder_scan(dir);
        }
    }

    /// Spawn a background scan of `dir` (recursive chart scan + difficulty-table re-match) and switch
    /// to the LOADING screen; `frame` polls `scan_rx` and applies the result via `apply_scan`.
    fn begin_folder_scan(&mut self, dir: PathBuf) {
        println!("scanning {} ...", dir.display());
        let sources = self.table_sources.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let scan_dir = dir.clone();
        std::thread::spawn(move || {
            let songs = scan_folder(&scan_dir);
            let (names, levels) = fetch_and_match(&sources, &songs);
            let _ = tx.send(ScanOutcome { songs, names, levels });
        });
        self.scan_rx = Some(rx);
        self.pending = Some(Loading::Folder(dir));
        self.loading_drawn = false;
        self.stage = Stage::Loading;
    }

    /// Apply a finished background scan: swap in the new library/tables, persist the folder, and
    /// return to the (rebuilt) select screen.
    fn apply_scan(&mut self, out: ScanOutcome) {
        self.songs = out.songs;
        self.table_names = out.names;
        self.table_levels = out.levels;
        self.select_view = SelectView::Root;
        self.sel = 0;
        self.rebuild_select_items();
        if let Some(Loading::Folder(dir)) = &self.pending {
            let path = dir.to_string_lossy().to_string();
            if self.config.songs_folder.as_deref() != Some(path.as_str()) {
                self.config.songs_folder = Some(path);
                self.save_settings();
            }
        }
        self.scan_rx = None;
        self.pending = None;
        self.stage = Stage::Select;
        println!("scanned {} charts", self.songs.len());
        self.print_selection();
    }

    fn open_tables(&mut self) {
        self.tables_sel = 0;
        self.text_input = None;
        self.stage = Stage::Tables;
    }

    /// Rows in the table-manager: each source, then the two add actions.
    fn tables_row_count(&self) -> usize {
        self.table_sources.len() + 2
    }

    fn add_table_source(&mut self, src: TableSource) {
        let (name, levels) = load_and_match(&src, &self.songs);
        self.table_sources.push(src);
        self.table_names.push(name);
        self.table_levels.push(levels);
        TableList { tables: self.table_sources.clone() }.save(&self.tables_path);
    }

    fn add_table_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new().set_title("Select table json").add_filter("json", &["json"]).pick_file() {
            let location = path.to_string_lossy().to_string();
            let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("table").to_string();
            self.add_table_source(TableSource { name, location });
        }
    }

    fn remove_table_source(&mut self, idx: usize) {
        if idx < self.table_sources.len() {
            self.table_sources.remove(idx);
            if idx < self.table_names.len() {
                self.table_names.remove(idx);
            }
            if idx < self.table_levels.len() {
                self.table_levels.remove(idx);
            }
            TableList { tables: self.table_sources.clone() }.save(&self.tables_path);
            self.tables_sel = self.tables_sel.min((self.table_sources.len() + 2).saturating_sub(1));
        }
    }

    /// Table-manager input. In URL-text mode, type the URL (Enter adds, Esc cancels); otherwise
    /// navigate, add (URL/file), remove (D), or leave (Esc — rebuilds the browse list).
    fn tables_input(&mut self, event_loop: &ActiveEventLoop, code: KeyCode, typed: Option<&str>) {
        if self.text_input.is_some() {
            match code {
                KeyCode::Enter | KeyCode::NumpadEnter => {
                    let url = self.text_input.take().unwrap_or_default().trim().to_string();
                    if !url.is_empty() {
                        self.add_table_source(TableSource { name: String::new(), location: url });
                    }
                }
                KeyCode::Escape => self.text_input = None,
                KeyCode::Backspace => {
                    if let Some(b) = self.text_input.as_mut() {
                        b.pop();
                    }
                }
                _ => {
                    if let (Some(b), Some(t)) = (self.text_input.as_mut(), typed) {
                        b.extend(t.chars().filter(|c| !c.is_control()));
                    }
                }
            }
            return;
        }
        let n = self.tables_row_count();
        let add_url = self.table_sources.len();
        let add_file = self.table_sources.len() + 1;
        match code {
            KeyCode::Escape => {
                self.select_view = SelectView::Root;
                self.sel = 0;
                self.rebuild_select_items();
                self.stage = Stage::Select;
                let _ = event_loop;
            }
            KeyCode::ArrowUp => self.tables_sel = self.tables_sel.saturating_sub(1),
            KeyCode::ArrowDown => self.tables_sel = (self.tables_sel + 1).min(n.saturating_sub(1)),
            KeyCode::KeyD | KeyCode::Delete => {
                if self.tables_sel < self.table_sources.len() {
                    self.remove_table_source(self.tables_sel);
                }
            }
            KeyCode::Enter | KeyCode::NumpadEnter => {
                if self.tables_sel == add_url {
                    self.text_input = Some(String::new());
                } else if self.tables_sel == add_file {
                    self.add_table_file();
                }
            }
            _ => {}
        }
    }

    /// Go up one select level; at the root, quit.
    fn select_back(&mut self, event_loop: &ActiveEventLoop) {
        match self.select_view {
            SelectView::Root => event_loop.exit(),
            SelectView::AllSongs | SelectView::TableLevels(_) => {
                self.select_view = SelectView::Root;
                self.sel = 0;
                self.rebuild_select_items();
            }
            SelectView::TableLevel(ti, _) => {
                self.select_view = SelectView::TableLevels(ti);
                self.sel = 0;
                self.rebuild_select_items();
            }
        }
    }

    /// Queue a loading task and switch to the loading screen; the actual (blocking) work happens
    /// one frame later in `finish_loading`, so a LOADING frame is presented first.
    fn begin_loading(&mut self, task: Loading) {
        self.pending = Some(task);
        self.loading_drawn = false;
        self.stage = Stage::Loading;
    }

    /// Perform the queued chart load (one frame after the LOADING screen is shown) then enter Play —
    /// `load()` (re)bases the song clock at its end, so playback starts at 0 only after everything is
    /// loaded. Folder scans don't reach here (they run on a background thread, polled in `frame`).
    fn finish_loading(&mut self) {
        match self.pending.take() {
            Some(Loading::Song(idx)) => match self.songs.get(idx) {
                Some(entry) => {
                    self.chart_path = entry.path.to_string_lossy().to_string();
                    self.result = None;
                    self.stage = if self.load() { Stage::Play } else { Stage::Select };
                }
                None => self.stage = Stage::Select,
            },
            _ => self.stage = Stage::Select,
        }
    }

    fn to_select_or_exit(&mut self, event_loop: &ActiveEventLoop) {
        if self.songs.is_empty() {
            event_loop.exit();
        } else {
            self.audio = None;
            self.player = None;
            self.result = None;
            self.replay = None;
            self.replay_cursor = 0;
            self.record_modal = None;
            // Clear analysis state so a stale overlay/virtual-clock can't leak into the next play.
            self.analysis = false;
            self.analysis_manual = false;
            self.analysis_paused = false;
            self.analysis_rate = 1.0;
            self.analysis_us = 0;
            self.msoff.clear();
            self.stage = Stage::Select;
            self.rebuild_select_items();
            self.print_selection();
        }
    }

    fn setting_line(&self, i: usize) -> (&'static str, String) {
        let on = |b: bool| if b { "ON".to_string() } else { "OFF".to_string() };
        match i {
            0 => ("AUTOPLAY", on(self.autoplay)),
            1 => ("HI-SPEED", format!("{:.2}", self.config.hispeed)),
            2 => ("SPEED FIX", if self.config.constant_speed { "CONSTANT".to_string() } else { "FLOATING".to_string() }),
            3 => ("RANDOM", self.config.random.label().to_string()),
            4 => ("GAUGE", gauge_name(self.config.gauge).to_string()),
            5 => ("LIFT", format!("{}%", (self.config.lift * 100.0).round() as i32)),
            6 => ("LANE COVER", format!("{}%", (self.config.cover * 100.0).round() as i32)),
            7 => ("SCRATCH SIDE", if self.config.scratch_left { "LEFT".to_string() } else { "RIGHT".to_string() }),
            8 => ("SCRATCH AUTO", on(self.config.scratch_auto)),
            9 => ("JUDGE OFFSET", format!("{:+} MS", self.config.offset_ms)),
            10 => ("BGA", on(self.config.bga)),
            11 => ("KEY CONFIG", ">".to_string()),
            12 => ("JUDGE WIDTH", format!("{}%", self.config.judge_rate)),
            13 => ("TOTAL", if self.config.total_override > 0.0 { format!("{}", self.config.total_override.round() as i32) } else { "AUTO".to_string() }),
            14 => ("SKIN", if self.config.skin_path.is_some() { "CUSTOM".to_string() } else { self.config.skin_name.clone() }),
            15 => ("AUTO CAL", on(self.config.auto_offset)),
            16 => ("AUTO REPLAY", on(self.config.auto_replay)),
            17 => ("DEBUG MODE", on(self.config.debug)),
            18 => ("FONT", if self.config.font_path.is_some() { "CUSTOM".to_string() } else { "DEFAULT".to_string() }),
            19 => ("SCORE GRAPH", on(self.config.score_graph)),
            20 => ("REPLAY ANALYSIS", on(self.config.replay_analysis)),
            21 => ("PREVIEW", on(self.config.preview)),
            _ => ("", String::new()),
        }
    }

    fn adjust_setting(&mut self, global: usize, d: i32) {
        match global {
            0 => self.autoplay = !self.autoplay,
            1 => self.config.hispeed = (self.config.hispeed + d as f64 * 0.25).clamp(0.5, 10.0),
            2 => self.config.constant_speed = !self.config.constant_speed,
            3 => {
                let idx = NoteOption::ALL.iter().position(|o| *o == self.config.random).unwrap_or(0);
                self.config.random = NoteOption::ALL[((idx as i32 + d).rem_euclid(NoteOption::ALL.len() as i32)) as usize];
            }
            4 => {
                let idx = GAUGE_CYCLE.iter().position(|g| *g == self.config.gauge).unwrap_or(2);
                self.config.gauge = GAUGE_CYCLE[((idx as i32 + d).rem_euclid(GAUGE_CYCLE.len() as i32)) as usize];
            }
            5 => self.config.lift = (self.config.lift + d as f32 * 0.05).clamp(0.0, 0.9),
            6 => self.config.cover = (self.config.cover + d as f32 * 0.05).clamp(0.0, 0.9),
            7 => self.config.scratch_left = !self.config.scratch_left,
            8 => self.config.scratch_auto = !self.config.scratch_auto,
            9 => self.config.offset_ms = (self.config.offset_ms + d * 5).clamp(-200, 200),
            10 => self.config.bga = !self.config.bga,
            12 => self.config.judge_rate = (self.config.judge_rate + d * 5).clamp(50, 200),
            13 => self.config.total_override = (self.config.total_override + d as f64 * 10.0).max(0.0),
            15 => self.config.auto_offset = !self.config.auto_offset,
            16 => self.config.auto_replay = !self.config.auto_replay,
            17 => self.config.debug = !self.config.debug,
            19 => self.config.score_graph = !self.config.score_graph,
            20 => {
                self.config.replay_analysis = !self.config.replay_analysis;
                // Re-evaluate an in-progress replay so toggling the setting takes effect immediately.
                if self.stage == Stage::Play {
                    self.analysis = self.replay.is_some() && self.config.replay_analysis;
                }
            }
            21 => self.config.preview = !self.config.preview,
            18 => {
                if d < 0 {
                    self.reset_font();
                } else {
                    self.pick_font();
                }
            }
            14 => {
                self.config.skin_path = None;
                self.config.skin_name = if self.config.skin_name.eq_ignore_ascii_case("WIDE") { "NORMAL".into() } else { "WIDE".into() };
            }
            _ => {}
        }
    }

    /// Load the current `chart_path` and set up the player. Returns `false` (after logging)
    /// if the chart file can't be read, so callers can recover instead of panicking.
    fn load(&mut self) -> bool {
        // Drop the select preview engine before creating the Play engine, so the two cpal output
        // streams never coexist — covers every play-entry path (the direct replay launch bypasses the
        // Loading stage where frame() would otherwise tear it down).
        self.stop_preview();
        let bytes = match std::fs::read(&self.chart_path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("chart not found: {} ({e})", self.chart_path);
                return false;
            }
        };
        let src = rbms_parser::parse_with(&bytes, Default::default());
        let mode = rbms_chart::detect_mode(&src, &self.chart_path);
        let mut model = to_model(&src, mode);
        self.recording.clear();
        self.replay_cursor = 0;
        self.cal_sum_us = 0;
        self.cal_count = 0;
        let (random, seed) = match &self.replay {
            Some(rp) => {
                if !rp.md5.is_empty() && rp.md5 != src.md5 {
                    eprintln!("warning: chart md5 mismatch (replay {} vs file {}); replay may desync", rp.md5, src.md5);
                }
                self.config.offset_ms = rp.offset_ms;
                self.config.scratch_auto = rp.scratch_auto;
                self.config.gauge = gauge_from_name(&rp.gauge);
                let random = NoteOption::from_str(&rp.random);
                self.config.random = random;
                (random, rp.seed)
            }
            None => (
                self.config.random,
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1),
            ),
        };
        self.seed = seed;
        rbms_chart::shuffle::apply(&mut model, random, seed);
        if model.meta.total <= 0.0 {
            model.meta.total = default_total(rbms_chart::count_playable_notes(&model));
        }
        if self.config.total_override > 0.0 {
            model.meta.total = self.config.total_override;
        }
        self.mode = mode;
        self.active_keys = self.config.keys_override.clone().unwrap_or_else(|| self.keyconfig.lane_keys(mode));
        self.skin_cfg = match &self.config.skin_path {
            Some(p) => SkinConfig::load(p).unwrap_or_else(|e| {
                eprintln!("skin load failed ({e}), using bundled");
                bundled_skin(&self.config.skin_name)
            }),
            None => bundled_skin(&self.config.skin_name),
        };
        self.rebuild_skin();
        let dir = Path::new(&self.chart_path).parent().unwrap_or(Path::new(".")).to_path_buf();

        match AudioEngine::new() {
            Ok(mut audio) => {
                let mut loaded = 0;
                for (id, name) in model.wavmap.iter().enumerate() {
                    if name.is_empty() {
                        continue;
                    }
                    if let Some((path, ext)) = resolve_keysound(&dir, name) {
                        if let Ok(data) = std::fs::read(&path) {
                            if audio.load(id as u32, data, Some(&ext)).is_ok() {
                                loaded += 1;
                            }
                        }
                    }
                }
                println!("loaded {loaded} keysounds, device {} Hz", audio.out_rate());
                self.audio = Some(audio);
            }
            Err(e) => println!("audio unavailable ({e}) — visual only"),
        }

        let status = if self.autoplay {
            "AUTOPLAY".to_string()
        } else {
            let mut keys = self.active_keys.clone();
            keys.sort_by_key(|(_, lane)| *lane);
            let hint = keys
                .iter()
                .map(|(code, lane)| {
                    let name = key_name(*code);
                    if mode.is_scratch(*lane) { format!("{name}=SC") } else { name.to_string() }
                })
                .collect::<Vec<_>>()
                .join(" ");
            format!("interactive ({hint})")
        };
        println!("playing '{}' [{}] ({} notes) — {}", model.meta.title, mode.name, rbms_chart::count_playable_notes(&model), status);
        self.bga_images.clear();
        if self.config.bga && self.skin.bga.is_some() {
            for (id, name) in model.bgamap.iter().enumerate() {
                if name.is_empty() {
                    continue;
                }
                if let Some(rgba) = decode_bga_256(&dir, name) {
                    self.bga_images.insert(id as i32, rgba);
                }
            }
        }
        self.bga_events = model.timelines.iter().filter(|tl| tl.bga >= 0).map(|tl| (tl.time_us, tl.bga)).collect();
        self.bga_events.sort_by_key(|e| e.0);
        self.bga_cursor = 0;
        self.cur_bga = -1;
        println!("loaded {} BGA images", self.bga_images.len());

        self.clock = Instant::now();
        self.anchor_us = self.audio.as_ref().map(|a| a.clock_us()).unwrap_or(0);
        // During replay playback the recorded input stream drives judging, so the player must not
        // also autoplay (that would double-hit every note).
        let mut player = Player::new(model, self.autoplay && self.replay.is_none());
        player.set_gauge(self.config.gauge);
        player.set_judge_rate(self.config.judge_rate);
        if self.config.scratch_auto {
            let auto: Vec<bool> = (0..mode.key).map(|l| mode.is_scratch(l)).collect();
            player.set_auto_lanes(auto);
        }
        self.player = Some(player);
        // Replay analysis starts following the live clock with sound; it only switches to the
        // virtual clock once the user takes manual control.
        self.analysis = self.replay.is_some() && self.config.replay_analysis;
        self.analysis_manual = false;
        self.analysis_paused = false;
        self.analysis_rate = 1.0;
        self.analysis_us = 0;
        self.msoff.clear();
        true
    }

    fn song_us(&self) -> i64 {
        match &self.audio {
            Some(a) => a.clock_us() - self.anchor_us,
            None => self.clock.elapsed().as_micros() as i64,
        }
    }

    fn enter_result(&mut self) {
        let Some(player) = self.player.as_ref() else { return };
        let j = &player.judge;
        let lamp = j.clear_lamp();
        let (label, color) = clear_label_color(lamp);
        println!(
            "RESULT [{label}]  EX {}/{}  combo {}/{}  gauge {:.1}%  PG/GR/GD/BD/POOR/MISS {:?}  empty-poor {}",
            j.ex_score,
            j.total_notes() * 2,
            j.max_combo,
            j.total_notes(),
            j.gauge.value(),
            j.counts,
            j.empty_poor,
        );
        // Deltas compare against the records that existed BEFORE this play (this run's record is
        // pushed further down), so prev = newest stored play, best = max stored EX.
        let history = self.scores.for_md5(&player.model().md5);
        let prev_ex = history.first().map(|r| r.ex_score);
        let prev_best_ex = history.iter().map(|r| r.ex_score).max();
        self.result = Some(ResultView {
            title: player.model().meta.title.chars().take(48).collect(),
            counts: j.counts,
            ex_score: j.ex_score,
            max_score: j.total_notes() * 2,
            max_combo: j.max_combo,
            total_notes: j.total_notes(),
            fast: j.fast,
            slow: j.slow,
            gauge: j.gauge.value(),
            clear_label: label,
            clear_color: color,
            prev_best_ex,
            prev_ex,
            show_graph: self.config.score_graph,
        });

        let model = player.model();
        let c = j.counts;
        let played_at = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0);
        let sub = ScoreSubmission {
            api_version: API_VERSION,
            chart: ChartId { md5: model.md5.clone(), sha256: model.sha256.clone() },
            player: PlayerId { id: self.config.player_id.clone() },
            mode: self.mode.name.to_string(),
            clear: ir_clear(lamp),
            ex_score: j.ex_score,
            max_ex_score: j.total_notes() * 2,
            judge: JudgeBreakdown {
                pgreat: c[0],
                great: c[1],
                good: c[2],
                bad: c[3],
                poor: c[4],
                miss: c[5],
                fast: j.fast,
                slow: j.slow,
                combobreak: c[3] + c[4] + c[5],
                epg: j.early[0],
                lpg: j.late[0],
                egr: j.early[1],
                lgr: j.late[1],
                egd: j.early[2],
                lgd: j.late[2],
                ebd: j.early[3],
                lbd: j.late[3],
                epr: j.early[4],
                lpr: j.late[4],
                ems: j.early[5],
                lms: j.late[5],
                avgjudge: j.avg_judge_us(),
                empty_poor: j.empty_poor,
            },
            max_combo: j.max_combo,
            total_notes: j.total_notes(),
            minbp: c[3] + c[4] + c[5],
            gauge_value: j.gauge.value(),
            options: PlayOptions {
                gauge: ir_gauge(self.config.gauge),
                random: ir_random(self.config.random),
                random_p2: None,
                scratch_auto: self.config.scratch_auto,
                lntype: 1,
                input_device: "keyboard".into(),
                assist: vec![],
                option: 0,
                judge_rate: self.config.judge_rate,
                offset_ms: self.config.offset_ms,
                constant: self.config.constant_speed,
                hispeed: self.config.hispeed,
                lift: self.config.lift,
                lane_cover: self.config.cover,
                total_override: self.config.total_override,
                autoplay: self.autoplay,
                auto_offset: self.config.auto_offset,
                scratch_left: self.config.scratch_left,
                green_number: if self.config.constant_speed {
                    2000.0 / self.config.hispeed * (1.0 - self.config.cover as f64)
                } else {
                    rbms_chart::scroll::green_number(model.init_bpm, self.config.hispeed, 1.0, self.config.cover as f64)
                },
            },
            played_at,
            client: concat!("rbms/", env!("CARGO_PKG_VERSION")).into(),
            replay_id: None,
            seed: self.seed,
            judge_algorithm: "Combo".into(),
            rule: String::new(),
            skin: self.config.skin_name.clone(),
            client_build_sha256: self.build_sha256.clone(),
            client_platform: Some(client_platform()),
            extra: Default::default(),
        };
        let server = self.server.clone();
        std::thread::spawn(move || match server.submit_score(&sub) {
            Ok(r) => println!("score submitted: accepted={} rank={:?}", r.accepted, r.rank),
            Err(e) => eprintln!("score submit: {e}"),
        });

        let played_ms = played_at;
        let mut replay_file: Option<String> = None;
        if self.config.auto_replay && self.replay.is_none() && !self.autoplay && !self.recording.is_empty() {
            let stem: String = model.md5.chars().take(8).collect();
            let dir = self.settings_path.parent().map(|d| d.join("replays")).unwrap_or_else(|| PathBuf::from("replays"));
            let name = format!("{stem}-{played_ms}.ron");
            let rp = Replay {
                chart_path: self.chart_path.clone(),
                md5: model.md5.clone(),
                mode: self.mode.name.to_string(),
                random: self.config.random.label().to_string(),
                seed: self.seed,
                offset_ms: self.config.offset_ms,
                scratch_auto: self.config.scratch_auto,
                gauge: gauge_token(self.config.gauge).to_string(),
                events: self.recording.clone(),
            };
            rp.save(&dir.join(&name));
            replay_file = Some(name);
        }

        // Persist a local play record (independent of the score server) for every real
        // interactive play, so history/replays survive offline. autoplay/replay runs are excluded.
        if self.replay.is_none() && !self.autoplay {
            let record = ScoreRecord {
                md5: model.md5.clone(),
                title: model.meta.title.clone(),
                mode: self.mode.name.to_string(),
                clear: clear_type_id(lamp),
                ex_score: j.ex_score,
                max_ex: j.total_notes() * 2,
                counts: j.counts,
                empty_poor: j.empty_poor,
                max_combo: j.max_combo,
                total_notes: j.total_notes(),
                gauge: gauge_token(self.config.gauge).to_string(),
                gauge_value: j.gauge.value(),
                random: self.config.random.label().to_string(),
                played_at: played_ms,
                replay_file,
            };
            self.scores.push(record);
            self.scores.save(&self.scores_path);
        }

        if self.config.auto_offset && !self.autoplay && self.replay.is_none() && self.cal_count >= 20 {
            let mean_us = self.cal_sum_us / self.cal_count as i64;
            let new_offset = calibrated_offset(self.config.offset_ms, mean_us);
            println!("auto-cal: avg {:+} ms over {} hits → judge offset {} ms (was {})", mean_us / 1000, self.cal_count, new_offset, self.config.offset_ms);
            self.config.offset_ms = new_offset;
            self.save_settings();
        }

        self.stage = Stage::Result;
    }

    fn frame(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;
        if dt > 0.0 {
            let inst = 1.0 / dt;
            self.fps = if self.fps <= 0.0 { inst } else { self.fps * 0.9 + inst * 0.1 };
        }
        self.frame_count = self.frame_count.wrapping_add(1);
        if self.config.debug && self.frame_count % 15 == 0 {
            if let Some(u) = memory_stats::memory_stats() {
                self.ram_mb = u.physical_mem as f32 / (1024.0 * 1024.0);
            }
        }
        if self.stage == Stage::Loading {
            if self.scan_rx.is_some() {
                // A background folder scan is running; apply it the frame it finishes, otherwise keep
                // animating the LOADING screen.
                match self.scan_rx.as_ref().unwrap().try_recv() {
                    Ok(out) => self.apply_scan(out),
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        self.scan_rx = None;
                        self.pending = None;
                        self.stage = Stage::Select;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {}
                }
            } else if self.loading_drawn {
                self.finish_loading();
            }
        }
        if self.stage == Stage::Select {
            self.refresh_focused_detail();
            self.update_preview();
        } else if self.preview_audio.is_some() {
            self.stop_preview();
        }
        // In manual analysis the displayed song time comes from the virtual clock (pausable,
        // rate-scaled); otherwise it follows the real (audio) clock, and analysis mirrors it so a
        // first manual control resumes from the live position. Manual analysis mutes keysounds.
        let manual = self.stage == Stage::Play && self.analysis && self.analysis_manual;
        let song = if manual {
            if !self.analysis_paused {
                self.analysis_us = (self.analysis_us + (dt as f64 * 1_000_000.0 * self.analysis_rate) as i64).max(0);
            }
            self.analysis_us
        } else {
            let s = self.song_us();
            if self.stage == Stage::Play && self.analysis {
                self.analysis_us = s;
            }
            s
        };
        let anchor = self.anchor_us;

        if self.stage == Stage::Play {
            if self.replay.is_some() {
                self.feed_replay(song, anchor, manual);
            }
            if let (false, Some(audio)) = (manual, self.audio.as_mut()) {
                let play = |e: PlayEvent| audio.play(e.wav.max(0) as u32, 1.0, 0.0, 1.0, e.at_us + anchor);
                if let Some(player) = self.player.as_mut() {
                    player.update(song, play);
                }
            } else if let Some(player) = self.player.as_mut() {
                player.update(song, |_| {});
            }
            if let Some(player) = self.player.as_ref() {
                // Analysis stays on the field at the end (the player scrubs); only a non-analysis
                // run auto-advances to the result screen.
                if !self.analysis && player.judge.total_notes() > 0 && song > player.last_time_us() + 2_000_000 {
                    self.enter_result();
                }
            }
        }

        let (set_tab, set_sel) = (self.set_tab.min(SETTING_TABS.len() - 1), self.set_sel);
        let settings_lines: Vec<(&'static str, String)> =
            if self.stage == Stage::Settings { SETTING_TABS[set_tab].1.iter().map(|&i| self.setting_line(i)).collect() } else { Vec::new() };
        // Refresh the (cached) select scene before the GPU borrow, then read it as a disjoint immutable
        // field so it coexists with the mutable `self.gpu` borrow during render.
        if self.stage == Stage::Select {
            self.refresh_select_cache();
        }
        let select_scene = if self.stage == Stage::Select { self.cached_select.as_ref() } else { None };

        if let Some(gpu) = self.gpu.as_mut() {
            self.hot.clear();
            match self.stage {
                Stage::Loading => {
                    gpu.clear_bga();
                    gpu.clear(Color::rgb(8, 8, 14));
                    let scanning = self.scan_rx.is_some();
                    let (heading, sub): (&str, String) = match &self.pending {
                        Some(Loading::Song(i)) => ("LOADING", self.songs.get(*i).map(|e| e.title.chars().take(30).collect()).unwrap_or_default()),
                        Some(Loading::Folder(dir)) => ("SCANNING", dir.file_name().and_then(|n| n.to_str()).map(|s| s.chars().take(40).collect()).unwrap_or_default()),
                        None => ("LOADING", String::new()),
                    };
                    let cx = CW as f32 * 0.5;
                    let cy = CH as f32 * 0.5;
                    let dots = ".".repeat((self.frame_count / 12 % 4) as usize);
                    draw_text_centered(gpu, cx, cy - 36.0, 3.0, Color::WHITE, &format!("{heading}{dots}"));
                    if !sub.is_empty() {
                        draw_text_centered(gpu, cx, cy + 14.0, 1.6, Color::rgb(200, 200, 215), &sub);
                    }
                    // Indeterminate sweeping bar (ping-pong) so a long background scan reads as working,
                    // not frozen.
                    if scanning {
                        let (bw, bh, seg) = (360.0, 6.0, 96.0);
                        let bx = cx - bw * 0.5;
                        let by = cy + 56.0;
                        gpu.fill_rect(Rect::new(bx, by, bw, bh), Color::rgb(28, 28, 40));
                        let phase = (self.frame_count % 120) as f32 / 60.0;
                        let t = if phase <= 1.0 { phase } else { 2.0 - phase };
                        gpu.fill_rect(Rect::new(bx + t * (bw - seg), by, seg, bh), Color::rgb(90, 200, 230));
                    }
                    self.loading_drawn = true;
                }
                Stage::Select => {
                    let view = select_scene.unwrap();
                    // Upload the focused cover into the single BGA slot (drawn behind the panel quads);
                    // the renderer leaves the cover square unfilled so the texture shows through.
                    match (&self.cover_rgba, &view.detail) {
                        (Some(rgba), SelectDetail::Song(_)) => gpu.set_bga(rgba, cover_rect()),
                        _ => gpu.clear_bga(),
                    }
                    let hot = render_select(gpu, view);
                    self.hot.extend(hot.into_iter().map(|(rect, h)| {
                        let mapped = match h {
                            SelectHot::Row(i) => Hot::SelectRow(i),
                            SelectHot::Record(i) => Hot::RecordRow(i),
                            SelectHot::ModalReplay => Hot::ModalReplay,
                            SelectHot::ModalClose => Hot::ModalClose,
                        };
                        (rect, mapped)
                    }));
                }
                Stage::Play => {
                    while self.bga_cursor < self.bga_events.len() && self.bga_events[self.bga_cursor].0 <= song {
                        self.cur_bga = self.bga_events[self.bga_cursor].1;
                        self.bga_cursor += 1;
                    }
                    match (self.config.bga, self.skin.bga, self.bga_images.get(&self.cur_bga)) {
                        (true, Some(rect), Some(img)) => gpu.set_bga(img, rect),
                        _ => gpu.clear_bga(),
                    }
                    if let Some(player) = self.player.as_ref() {
                        render_playfield(gpu, &player.model().timelines, song, self.config.hispeed, &self.skin, player.beam_on(), player.beam_off(), self.config.constant_speed);
                        render_lane_cover(gpu, &self.skin, self.config.cover);
                        render_key_bomb(gpu, &self.skin, player.bomb(), song);
                        let j = &player.judge;
                        // Green number = note travel time (ms) for the scroll speed shown right now. In
                        // CONSTANT mode it is fixed (2000/hi-speed at the calibration BPM); in FLOATING
                        // it tracks the BPM of the timeline segment under the current play time.
                        let tls = &player.model().timelines;
                        let seg = tls.binary_search_by(|t| t.time_us.cmp(&song)).unwrap_or_else(|i| i.saturating_sub(1));
                        let (bpm, scroll) = tls.get(seg).map(|t| (t.bpm, t.scroll)).unwrap_or((player.model().init_bpm, 1.0));
                        let cover = self.config.cover as f64;
                        let green = if self.config.constant_speed {
                            2000.0 / self.config.hispeed * (1.0 - cover)
                        } else {
                            rbms_chart::scroll::green_number(bpm, self.config.hispeed, scroll, cover)
                        };
                        let best_ex = self.scores.best_ex_for_md5(&player.model().md5);
                        let hud = HudView {
                            combo: j.combo,
                            last_judge: j.last_judge.map(|x| x as u8),
                            last_fast: j.last_fast,
                            fast: j.fast,
                            slow: j.slow,
                            counts: j.counts,
                            ex_score: j.ex_score,
                            gauge: j.gauge.value(),
                            green_number: green,
                            max_ex: j.total_notes() * 2,
                            best_ex,
                        };
                        render_hud(gpu, &self.skin, &hud);

                        // Replay analysis overlay: a playback bar, the current rate/paused state and
                        // time, plus the recent per-note timing errors (ms early = cyan +, late = orange -).
                        if self.analysis {
                            let total = player.last_time_us().max(1);
                            let prog = (song as f32 / total as f32).clamp(0.0, 1.0);
                            let bx = 40.0;
                            let bw = CW as f32 - 80.0;
                            let by = CH as f32 - 14.0;
                            gpu.fill_rect(Rect::new(0.0, CH as f32 - 74.0, CW as f32, 74.0), Color { r: 0, g: 0, b: 0, a: 170 });
                            gpu.fill_rect(Rect::new(bx, by, bw, 6.0), Color::rgb(40, 40, 52));
                            gpu.fill_rect(Rect::new(bx, by, bw * prog, 6.0), Color::rgb(90, 200, 230));
                            gpu.fill_rect(Rect::new((bx + bw * prog - 1.5).max(bx), by - 3.0, 3.0, 12.0), Color::WHITE);
                            let state = if self.analysis_paused { "PAUSED".to_string() } else { format!("{:.2}x", self.analysis_rate) };
                            draw_text(gpu, bx, CH as f32 - 66.0, 1.4, Color::YELLOW, &format!("ANALYSIS  {state}   {:.1} / {:.1} S", song as f32 / 1e6, total as f32 / 1e6));
                            draw_text_right(gpu, bx + bw, CH as f32 - 66.0, 1.1, Color::GRAY, "SPACE PAUSE  -/+ SPEED  PGUP/PGDN SEEK  ESC EXIT");
                            let start = self.msoff.len().saturating_sub(14);
                            let mut mx = bx;
                            for &(_, delta_us, _) in &self.msoff[start..] {
                                let col = if delta_us > 0 { Color::rgb(90, 210, 230) } else if delta_us < 0 { Color::ORANGE } else { Color::WHITE };
                                let txt = format!("{:+}", delta_us / 1000);
                                draw_text(gpu, mx, CH as f32 - 44.0, 1.3, col, &txt);
                                mx += text_width(&txt, 1.3) + 12.0;
                            }
                        }
                    }
                }
                Stage::Settings => {
                    gpu.clear_bga();
                    gpu.clear(Color::rgb(8, 8, 14));
                    const PANEL_W: f32 = 720.0;
                    let x0 = (CW as f32 - PANEL_W) * 0.5;
                    draw_text(gpu, x0, 36.0, 3.0, Color::WHITE, "SETTINGS");
                    draw_text(gpu, x0, 78.0, 1.2, Color::GRAY, "TAB SWITCH   UP DOWN MOVE   LEFT RIGHT CHANGE   ENTER OPEN   ESC SAVE/BACK");
                    let mut tx = x0;
                    for (ti, (name, _)) in SETTING_TABS.iter().enumerate() {
                        let on = ti == set_tab;
                        let w = text_width(name, 1.6) + 28.0;
                        gpu.fill_rect(Rect::new(tx, 104.0, w, 34.0), if on { Color::rgb(70, 80, 120) } else { Color::LANE_BG });
                        draw_text(gpu, tx + 14.0, 112.0, 1.6, if on { Color::YELLOW } else { Color::rgb(150, 150, 165) }, name);
                        self.hot.push((Rect::new(tx, 104.0, w, 34.0), Hot::SettingTab(ti)));
                        tx += w + 8.0;
                    }
                    for (i, (label, val)) in settings_lines.iter().enumerate() {
                        let y = 160.0 + i as f32 * 50.0;
                        let sel = i == set_sel;
                        gpu.fill_rect(Rect::new(x0, y, PANEL_W, 42.0), if sel { Color::rgb(70, 80, 120) } else { Color::LANE_BG });
                        draw_text(gpu, x0 + 20.0, y + 13.0, 1.8, if sel { Color::WHITE } else { Color::rgb(170, 170, 185) }, label);
                        draw_text_right(gpu, x0 + PANEL_W - 20.0, y + 13.0, 2.0, if sel { Color::YELLOW } else { Color::WHITE }, val);
                        self.hot.push((Rect::new(x0, y, PANEL_W, 42.0), Hot::SettingRow(i)));
                    }
                }
                Stage::KeyConfig => {
                    gpu.clear_bga();
                    gpu.clear(Color::rgb(8, 8, 14));
                    const PANEL_W: f32 = 760.0;
                    let x0 = (CW as f32 - PANEL_W) * 0.5;
                    draw_text(gpu, x0, 40.0, 3.0, Color::WHITE, "KEY CONFIG");
                    let (hint, hint_col) = if self.kc_warn {
                        ("KEY ALREADY BOUND - TRY ANOTHER", Color::RED)
                    } else if self.kc_capturing {
                        ("PRESS A KEY...   ESC CANCEL", Color::YELLOW)
                    } else {
                        ("UP DOWN MOVE   ENTER REBIND   LEFT RIGHT EDIT-MODE   ESC SAVE/BACK", Color::GRAY)
                    };
                    draw_text(gpu, x0, 82.0, 1.3, hint_col, hint);
                    let dups = self.keyconfig.collisions(self.kc_edit_mode);
                    let rows = kc_rows(self.kc_edit_mode);
                    let row_h = 28.0;
                    let visible = 17usize;
                    let start = self.kc_sel.saturating_sub(visible / 2).min(rows.len().saturating_sub(visible.min(rows.len())));
                    for (i, ridx) in (start..(start + visible).min(rows.len())).enumerate() {
                        let y = 112.0 + i as f32 * (row_h + 2.0);
                        let on = ridx == self.kc_sel;
                        let (label, raw) = match &rows[ridx] {
                            KcRow::ModeSelect => ("EDIT MODE".to_string(), mode_short(self.kc_edit_mode).to_string()),
                            KcRow::Control(a) => (a.label().to_string(), self.keyconfig.control_token(*a).to_string()),
                            KcRow::Lane(lane) => {
                                let name = if self.kc_edit_mode.is_scratch(*lane) { format!("SCRATCH {}", lane + 1) } else { format!("LANE {}", lane + 1) };
                                (name, self.keyconfig.lane_token(self.kc_edit_mode, *lane))
                            }
                        };
                        let is_dup = !matches!(&rows[ridx], KcRow::ModeSelect) && key_from_name(&raw).is_some_and(|k| dups.contains(&k));
                        gpu.fill_rect(Rect::new(x0, y, PANEL_W, row_h), if on { Color::rgb(70, 80, 120) } else { Color::LANE_BG });
                        draw_text(gpu, x0 + 16.0, y + 8.0, 1.6, if on { Color::WHITE } else { Color::rgb(170, 170, 185) }, &label);
                        let value = if on && self.kc_capturing { "?".to_string() } else if raw.is_empty() { "-".to_string() } else { raw };
                        let vcol = if on && self.kc_capturing {
                            Color::YELLOW
                        } else if is_dup {
                            Color::RED
                        } else if on {
                            Color::rgb(120, 230, 230)
                        } else {
                            Color::WHITE
                        };
                        draw_text_right(gpu, x0 + PANEL_W - 16.0, y + 8.0, 1.6, vcol, &value);
                    }
                }
                Stage::Tables => {
                    gpu.clear_bga();
                    gpu.clear(Color::rgb(8, 8, 14));
                    const PANEL_W: f32 = 900.0;
                    let x0 = (CW as f32 - PANEL_W) * 0.5;
                    draw_text(gpu, x0, 40.0, 3.0, Color::WHITE, "DIFFICULTY TABLES");
                    if let Some(buf) = self.text_input.as_ref() {
                        draw_text(gpu, x0, 84.0, 1.4, Color::YELLOW, "TYPE TABLE URL  -  ENTER ADD  ESC CANCEL");
                        gpu.fill_rect(Rect::new(x0, 116.0, PANEL_W, 40.0), Color::LANE_BG);
                        let shown: String = buf.chars().rev().take(70).collect::<Vec<_>>().into_iter().rev().collect();
                        draw_text(gpu, x0 + 14.0, 128.0, 1.6, Color::WHITE, &format!("{shown}_"));
                    } else {
                        draw_text(gpu, x0, 84.0, 1.3, Color::GRAY, "UP DOWN MOVE   ENTER SELECT   D REMOVE   ESC BACK");
                        let add_url = self.table_sources.len();
                        let row_count = self.table_sources.len() + 2;
                        for i in 0..row_count {
                            let y = 124.0 + i as f32 * 44.0;
                            let on = i == self.tables_sel;
                            let label = if i < self.table_sources.len() {
                                let s = &self.table_sources[i];
                                let nm = if s.name.is_ascii() && !s.name.trim().is_empty() { s.name.clone() } else { String::new() };
                                let loc: String = s.location.chars().take(64).collect();
                                if nm.is_empty() { loc } else { format!("{nm}  -  {loc}") }
                            } else if i == add_url {
                                "+ ADD TABLE (URL)".to_string()
                            } else {
                                "+ ADD TABLE (FILE)".to_string()
                            };
                            gpu.fill_rect(Rect::new(x0, y, PANEL_W, 36.0), if on { Color::rgb(70, 80, 120) } else { Color::LANE_BG });
                            let col = if i >= self.table_sources.len() { Color::GREEN } else if on { Color::WHITE } else { Color::rgb(170, 170, 185) };
                            draw_text(gpu, x0 + 16.0, y + 10.0, 1.5, col, &label);
                        }
                    }
                }
                Stage::Result => {
                    gpu.clear_bga();
                    if let Some(view) = self.result.as_ref() {
                        render_result(gpu, view);
                    }
                }
            }
            if self.config.server_url.is_some() {
                let connected = self.server_connected.load(Ordering::Relaxed);
                gpu.fill_rect(Rect::new(CW as f32 - 22.0, 10.0, 10.0, 10.0), if connected { Color::GREEN } else { Color::RED });
            }
            if self.config.debug {
                let frame_ms = if self.fps > 0.0 { 1000.0 / self.fps } else { 0.0 };
                let mut lines = vec![
                    "DEBUG".to_string(),
                    format!("FPS {:.0}  ({:.1} MS)", self.fps, frame_ms),
                    format!("RAM {:.1} MB", self.ram_mb),
                    format!("QUADS {}  STAGE {:?}", gpu.quad_count(), self.stage),
                ];
                if self.stage == Stage::Play {
                    if let Some(p) = self.player.as_ref() {
                        let j = &p.judge;
                        lines.push(format!("TIME {:.2} / {:.2} S", song as f32 / 1e6, p.last_time_us() as f32 / 1e6));
                        lines.push(format!("NOTES {} / {}", j.total_judged(), j.total_notes()));
                        lines.push(format!("COMBO {}  MAX {}", j.combo, j.max_combo));
                        lines.push(format!("EX {}  GAUGE {:.1}%", j.ex_score, j.gauge.value()));
                        lines.push(format!("FAST {}  SLOW {}  EPOOR {}", j.fast, j.slow, j.empty_poor));
                    }
                    lines.push(format!("HISPEED {:.2}  OFFSET {:+}MS", self.config.hispeed, self.config.offset_ms));
                    let clk = self.audio.as_ref().map(|a| a.clock_us()).unwrap_or(0);
                    lines.push(format!("AUDIO {} US  ANCHOR {}", clk, anchor));
                } else {
                    lines.push(format!("SEL {} / {}", self.sel + 1, self.select_items.len()));
                    lines.push(format!("SCORES {}  SONGS {}", self.scores.records.len(), self.songs.len()));
                    lines.push(format!("CURSOR {:.0} {:.0}", self.cursor.0, self.cursor.1));
                }
                let lh = 16.0;
                let ph = lines.len() as f32 * lh + 12.0;
                gpu.fill_rect(Rect::new(6.0, 6.0, 320.0, ph), Color { r: 0, g: 0, b: 0, a: 180 });
                for (i, l) in lines.iter().enumerate() {
                    let col = if i == 0 { Color::YELLOW } else { Color::rgb(120, 240, 140) };
                    draw_text(gpu, 14.0, 12.0 + i as f32 * lh, 1.2, col, l);
                }
            }
            gpu.render();
        }
    }

    fn print_selection(&self) {
        let total = self.select_items.len();
        match self.select_items.get(self.sel) {
            Some(SelectItem::Song(i)) => {
                if let Some(e) = self.songs.get(*i) {
                    println!("[{}/{}] {} [{}]", self.sel + 1, total, e.title, e.mode.name);
                }
            }
            Some(SelectItem::Folder { label, .. }) => println!("[{}/{}] {label}/", self.sel + 1, total),
            None => {}
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gpu.is_some() {
            return;
        }
        let attrs = Window::default_attributes().with_title("rbms").with_inner_size(winit::dpi::LogicalSize::new(CW, CH));
        let window = Arc::new(event_loop.create_window(attrs).unwrap());
        self.gpu = Some(Gpu::new(window.clone()));
        if self.stage == Stage::Play && !self.load() {
            event_loop.exit();
            return;
        }
        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(gpu) = self.gpu.as_mut() {
                    gpu.resize(size.width, size.height);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                // Map the physical cursor onto the fixed 1280×720 logical space the UI is laid out
                // in (the surface stretches that space across the whole window).
                if let Some(gpu) = self.gpu.as_ref() {
                    let sz = gpu.window.inner_size();
                    self.cursor = (position.x as f32 * CW as f32 / sz.width.max(1) as f32, position.y as f32 * CH as f32 / sz.height.max(1) as f32);
                }
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => self.handle_click(),
            WindowEvent::KeyboardInput { event, .. } => {
                let PhysicalKey::Code(code) = event.physical_key else { return };
                let pressed = event.state == ElementState::Pressed && !event.repeat;
                match self.stage {
                    Stage::Select if self.record_modal.is_some() => {
                        if pressed {
                            match code {
                                KeyCode::Escape => self.record_modal = None,
                                KeyCode::ArrowUp => self.record_modal_nav(-1),
                                KeyCode::ArrowDown => self.record_modal_nav(1),
                                KeyCode::Enter | KeyCode::NumpadEnter => self.play_record_replay(),
                                _ => {}
                            }
                        }
                    }
                    Stage::Select => {
                        if pressed {
                            match code {
                                KeyCode::Escape | KeyCode::ArrowLeft => self.select_back(event_loop),
                                KeyCode::Tab => self.stage = Stage::Settings,
                                KeyCode::KeyO => self.open_folder_dialog(),
                                KeyCode::KeyT => self.open_tables(),
                                KeyCode::KeyR => self.open_record_modal(),
                                KeyCode::ArrowUp => {
                                    self.sel = self.sel.saturating_sub(1);
                                    self.print_selection();
                                }
                                KeyCode::ArrowDown => {
                                    if self.sel + 1 < self.select_items.len() {
                                        self.sel += 1;
                                    }
                                    self.print_selection();
                                }
                                KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::ArrowRight => self.select_enter(),
                                _ => {}
                            }
                        }
                    }
                    Stage::Settings => {
                        if pressed {
                            let items = SETTING_TABS[self.set_tab].1;
                            self.set_sel = self.set_sel.min(items.len().saturating_sub(1));
                            let focused = items.get(self.set_sel).copied().unwrap_or(0);
                            let on_keyconfig = focused == SETTING_KEYCONFIG;
                            let on_font = focused == SETTING_FONT;
                            match code {
                                KeyCode::Escape => {
                                    self.save_settings();
                                    self.stage = Stage::Select;
                                }
                                KeyCode::Tab => {
                                    self.set_tab = (self.set_tab + 1) % SETTING_TABS.len();
                                    self.set_sel = 0;
                                }
                                KeyCode::Enter | KeyCode::NumpadEnter => {
                                    if on_keyconfig {
                                        self.enter_keyconfig();
                                    } else if on_font {
                                        self.pick_font();
                                    } else {
                                        self.save_settings();
                                        self.stage = Stage::Select;
                                    }
                                }
                                KeyCode::ArrowUp => self.set_sel = self.set_sel.saturating_sub(1),
                                KeyCode::ArrowDown => self.set_sel = (self.set_sel + 1).min(items.len().saturating_sub(1)),
                                KeyCode::ArrowLeft => self.adjust_setting(focused, -1),
                                KeyCode::ArrowRight => {
                                    if on_keyconfig {
                                        self.enter_keyconfig();
                                    } else {
                                        self.adjust_setting(focused, 1);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    Stage::KeyConfig => {
                        if pressed {
                            self.keyconfig_input(code);
                        }
                    }
                    Stage::Tables => {
                        if pressed {
                            self.tables_input(event_loop, code, event.text.as_deref());
                        }
                    }
                    Stage::Result => {
                        if pressed && matches!(code, KeyCode::Escape | KeyCode::Enter | KeyCode::NumpadEnter) {
                            self.to_select_or_exit(event_loop);
                        }
                    }
                    Stage::Loading => {
                        if pressed && code == KeyCode::Escape {
                            self.pending = None;
                            self.scan_rx = None;
                            self.to_select_or_exit(event_loop);
                        }
                    }
                    Stage::Play => {
                        if code == KeyCode::Escape {
                            // If there is nothing left to hit (every note resolved), skip straight to
                            // the result screen instead of discarding the run; otherwise quit out.
                            let done = self.player.as_ref().is_some_and(|p| p.judge.total_notes() > 0 && p.judge.total_judged() >= p.judge.total_notes());
                            if done {
                                self.enter_result();
                            } else {
                                self.to_select_or_exit(event_loop);
                            }
                            return;
                        }
                        if pressed && self.analysis_key(code) {
                            return;
                        }
                        if pressed {
                            if let Some(action) = self.control_for(code) {
                                self.apply_control(action);
                                return;
                            }
                        }
                        if !self.autoplay && self.replay.is_none() {
                            if let Some(lane) = self.lane_for(code) {
                                let raw = self.song_us();
                                let judge_t = raw + self.offset_us();
                                let anchor = self.anchor_us;
                                match event.state {
                                    ElementState::Pressed if !event.repeat => {
                                        self.recording.push(ReplayEvent { t: raw, lane, press: true });
                                        let mut hit: Option<rbms_judge::JudgeResult> = None;
                                        if let (Some(player), Some(audio)) = (self.player.as_mut(), self.audio.as_mut()) {
                                            hit = player.press(lane, judge_t, |e: PlayEvent| audio.play(e.wav.max(0) as u32, 1.0, 0.0, 1.0, e.at_us + anchor));
                                        }
                                        // auto-calibration: accumulate the timing error of accurate hits
                                        // (PG/GR/GD, ±150ms) — offset is recentred for the next run at
                                        // enter_result, so within-run judging stays consistent.
                                        if self.config.auto_offset {
                                            if let Some(r) = hit {
                                                if (r.judge as usize) <= 2 && r.delta_us.abs() <= 150_000 {
                                                    self.cal_sum_us += r.delta_us;
                                                    self.cal_count += 1;
                                                }
                                            }
                                        }
                                    }
                                    ElementState::Released => {
                                        self.recording.push(ReplayEvent { t: raw, lane, press: false });
                                        if let Some(player) = self.player.as_mut() {
                                            player.release(lane, judge_t);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                self.frame();
                if let Some(gpu) = self.gpu.as_ref() {
                    gpu.window.request_redraw();
                }
            }
            _ => {}
        }
    }
}

fn main() {
    let settings_path = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(".")).join(".config/rbms/settings.ron");
    let saved = PlaySettings::load(&settings_path);
    let mut cfg = PlayerConfig::default();
    apply_settings(&mut cfg, &saved);
    let mut autoplay = saved.autoplay;

    let mut chart: Option<String> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--interactive" => autoplay = false,
            "--auto" => autoplay = true,
            "--sc-left" => cfg.scratch_left = true,
            "--sc-auto" => cfg.scratch_auto = true,
            "--lift" => {
                if let Some(v) = args.next().and_then(|v| v.parse::<f32>().ok()) {
                    cfg.lift = v.clamp(0.0, 0.9);
                }
            }
            "--hispeed" => {
                if let Some(v) = args.next().and_then(|v| v.parse::<f64>().ok()) {
                    cfg.hispeed = v.clamp(0.5, 10.0);
                }
            }
            "--gauge" => {
                if let Some(name) = args.next() {
                    cfg.gauge = gauge_from_name(&name);
                }
            }
            "--skin" => cfg.skin_path = args.next(),
            "--font" => cfg.font_path = args.next(),
            "--server" => cfg.server_url = args.next(),
            "--table" => cfg.table_url = args.next(),
            "--keyconfig" => cfg.keyconfig_path = args.next(),
            "--replay" => cfg.replay_path = args.next(),
            "--player" => {
                if let Some(id) = args.next() {
                    cfg.player_id = id;
                }
            }
            "--keys" => {
                if let Some(spec) = args.next() {
                    let parsed: Vec<(KeyCode, usize)> = spec.split(',').enumerate().filter_map(|(i, n)| key_from_name(n.trim()).map(|k| (k, i))).collect();
                    if !parsed.is_empty() {
                        cfg.keys_override = Some(parsed);
                    }
                }
            }
            other if chart.is_none() && !other.starts_with("--") => chart = Some(other.to_string()),
            _ => {}
        }
    }
    // Folder/chart precedence: explicit launch arg > the remembered song folder > $RBMS_SONGS.
    let remembered = saved.songs_folder.clone().filter(|s| !s.trim().is_empty()).or_else(|| std::env::var("RBMS_SONGS").ok().filter(|s| !s.trim().is_empty()));
    let chart = match chart {
        Some(c) => c,
        None if cfg.replay_path.is_some() => String::new(),
        None => match remembered {
            Some(folder) => folder,
            None => {
                eprintln!(
                    "usage: rbms-player <chart|folder> [--interactive|--auto] [--sc-left] [--sc-auto] [--lift F] [--hispeed F] [--gauge ...] [--keys ...] [--table URL] [--keyconfig path.ron] [--replay file.ron]"
                );
                std::process::exit(1);
            }
        },
    };

    // Apply a user-chosen UI font (settings or --font) before any text is drawn.
    if let Some(fp) = cfg.font_path.clone() {
        match std::fs::read(&fp) {
            Ok(bytes) => match rbms_render::load_font(bytes) {
                Some(family) => {
                    rbms_render::set_ui_family(&family);
                    println!("font: {fp} ({family})");
                }
                None => eprintln!("font load failed (no usable face): {fp}"),
            },
            Err(e) => eprintln!("font not found: {fp} ({e})"),
        }
    }

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::new(chart, autoplay, cfg, settings_path);
    event_loop.run_app(&mut app).unwrap();
}

#[cfg(test)]
mod tests {
    use super::{calibrated_offset, clear_type_from_id, clear_type_id, client_platform, compute_build_hash, fmt_datetime};
    use rbms_judge::ClearType;

    #[test]
    fn datetime_formats_utc() {
        assert_eq!(fmt_datetime(0), "1970-01-01 00:00");
        assert_eq!(fmt_datetime(86_400_000), "1970-01-02 00:00");
        assert_eq!(fmt_datetime(1_700_000_000_000), "2023-11-14 22:13");
    }

    #[test]
    fn clear_lamp_id_roundtrips() {
        for c in [ClearType::NoPlay, ClearType::Failed, ClearType::AssistEasy, ClearType::Easy, ClearType::Normal, ClearType::Hard, ClearType::ExHard, ClearType::FullCombo, ClearType::Perfect, ClearType::Max] {
            assert_eq!(clear_type_from_id(clear_type_id(c)), c, "{c:?} lamp id must round-trip");
        }
    }

    #[test]
    fn build_hash_is_64_hex_chars() {
        let h = compute_build_hash().expect("the test binary should be readable");
        assert_eq!(h.len(), 64, "SHA-256 hex is 64 chars");
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()), "hash is lowercase hex");
    }

    #[test]
    fn client_platform_is_os_arch() {
        let p = client_platform();
        assert!(p.contains('-'), "platform tag is OS-ARCH");
        assert_eq!(p, format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH));
    }

    #[test]
    fn calibrated_offset_recenters_from_mean() {
        assert_eq!(calibrated_offset(0, 60_000), 60, "60ms-early avg -> +60ms offset");
        assert_eq!(calibrated_offset(0, -40_000), -40);
        assert_eq!(calibrated_offset(60, 0), 60, "no error -> no change");
        assert_eq!(calibrated_offset(0, 0), 0);
        assert_eq!(calibrated_offset(190, 50_000), 200, "clamped to +200");
    }

    #[test]
    fn calibration_converges_in_one_run() {
        // a constant +60ms-early bias: the run's mean error is (bias - offset*1000);
        // recentring once should drive the next run's mean error to ~0.
        let bias_us: i64 = 60_000;
        let offset0 = 0;
        let mean_run1 = bias_us - offset0 as i64 * 1000;
        let offset1 = calibrated_offset(offset0, mean_run1);
        let mean_run2 = bias_us - offset1 as i64 * 1000;
        assert!(mean_run2.abs() <= 1_000, "after one calibration the residual error is ~0 (was {mean_run2}us)");
    }
}
