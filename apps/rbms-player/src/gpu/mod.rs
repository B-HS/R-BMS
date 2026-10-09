//! Native instanced-quad GPU backend (wgpu). Extracted from `main.rs` so the renderer's wgpu
//! plumbing lives apart from the app/UI state machine. `Gpu` implements `rbms_render::Renderer`,
//! so every UI composer (`render_playfield`/`render_hud`/…) targets it directly.
//!
//! The space a frame is drawn in is the target's own pixels: [`Renderer::size`] answers the size of
//! the viewport the frame lands in, which is the whole window when it is stretched and the largest
//! rectangle of the screen's shape when it is fitted. A skin screen draws straight onto that. The
//! built-in screens are laid out for a fixed size instead and reach this backend through
//! `rbms_render::ScaledRenderer`, which is what `crate::stage::Canvas` wraps it in.
//!
//! Colours go to the target as the bytes they were given as. The surface is configured, or viewed,
//! in a format without an sRGB transfer, so a texel of 128 is a pixel of 128 and blending works on
//! those bytes -- the arithmetic `rbms_render::CpuCanvas` does, and what the reference
//! implementation's default framebuffer does.

mod background;
mod batch;
#[cfg(test)]
mod pixel_tests;

#[cfg(test)]
pub(crate) use background::background_upload_needed;

use std::collections::HashMap;
use std::sync::Arc;

use batch::{Batch, BatchKind, ColoredInstance, DrawList, TexturedInstance, pad_rows_to_alignment, scissor_rect};
use rbms_render::{BlendFactor, BlendMode, Color, QuadParams, Rect, Renderer, TextureFilter, TextureId};
use winit::window::Window;

use crate::notify::{Level, notify};

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

const TEXTURED_SHADER: &str = r#"
@group(0) @binding(0) var<uniform> screen: vec4<f32>;
@group(1) @binding(0) var t: texture_2d<f32>;
@group(1) @binding(1) var s: sampler;
struct VsOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) tint: vec4<f32> };
@vertex
fn vs(@builtin(vertex_index) vi: u32,
      @location(0) rect: vec4<f32>,
      @location(1) uv: vec4<f32>,
      @location(2) tint: vec4<f32>,
      @location(3) rot: vec4<f32>) -> VsOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0));
    let c = corners[vi];
    let d = c * rect.zw - rot.zw;
    let turned = vec2<f32>(d.x * rot.x - d.y * rot.y, d.x * rot.y + d.y * rot.x);
    let px = rect.xy + rot.zw + turned;
    let ndc = vec2<f32>(px.x / screen.x * 2.0 - 1.0, 1.0 - px.y / screen.y * 2.0);
    var o: VsOut;
    o.pos = vec4<f32>(ndc, 0.0, 1.0);
    o.uv = mix(uv.xy, uv.zw, c);
    o.tint = tint;
    return o;
}
@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> { return textureSample(t, s, in.uv) * in.tint; }
"#;

/// The blend modes a textured quad can ask for, in the order their pipelines are built and indexed.
const BLEND_MODES: [BlendMode; 4] = [BlendMode::Alpha, BlendMode::Add, BlendMode::Multiply, BlendMode::InvertDst];

fn blend_index(mode: BlendMode) -> usize {
    BLEND_MODES.iter().position(|m| *m == mode).unwrap_or(0)
}

/// One term of the shared blend table as wgpu names it, so both backends blend by the same rule.
fn wgpu_factor(factor: BlendFactor) -> wgpu::BlendFactor {
    match factor {
        BlendFactor::Zero => wgpu::BlendFactor::Zero,
        BlendFactor::One => wgpu::BlendFactor::One,
        BlendFactor::SrcAlpha => wgpu::BlendFactor::SrcAlpha,
        BlendFactor::OneMinusSrcAlpha => wgpu::BlendFactor::OneMinusSrcAlpha,
        BlendFactor::SrcColor => wgpu::BlendFactor::Src,
        BlendFactor::OneMinusDstColor => wgpu::BlendFactor::OneMinusDst,
    }
}

fn blend_state(mode: BlendMode) -> wgpu::BlendState {
    let f = mode.factors();
    let component = |src, dst| wgpu::BlendComponent { src_factor: wgpu_factor(src), dst_factor: wgpu_factor(dst), operation: wgpu::BlendOperation::Add };
    wgpu::BlendState { color: component(f.src_color, f.dst_color), alpha: component(f.src_alpha, f.dst_alpha) }
}

/// One registered texture and the bind groups that read it, one per sampler.
struct GpuTexture {
    width: u32,
    height: u32,
    texture: wgpu::Texture,
    nearest: wgpu::BindGroup,
    linear: wgpu::BindGroup,
}

const INSTANCE_ATTRS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4];

const TEXTURED_ATTRS: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4, 3 => Float32x4];

/// Instances the buffers are sized for before they have to grow.
const INITIAL_INSTANCE_CAPACITY: usize = 8192;

/// Bytes the uniform holding the size of the space a frame is drawn in takes: one `vec4<f32>`.
const SCREEN_UNIFORM_BYTES: u64 = 16;

/// Formats whose bytes reach the target exactly as a fragment wrote them, in order of preference.
const PASSTHROUGH_FORMATS: [wgpu::TextureFormat; 2] = [wgpu::TextureFormat::Bgra8Unorm, wgpu::TextureFormat::Rgba8Unorm];

/// The format an offscreen target is rendered in and read back as.
#[cfg(test)]
const OFFSCREEN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// Environment variable that makes a test which needs a graphics adapter fail, rather than pass
/// unchecked, on a machine that cannot bring one up.
#[cfg(test)]
pub(crate) const REQUIRE_GPU_ENV: &str = "RBMS_REQUIRE_GPU";

/// Where a finished frame goes.
enum Target {
    /// A window's surface, presented as each frame completes.
    Window { window: Arc<Window>, surface: wgpu::Surface<'static>, config: wgpu::SurfaceConfiguration },
    /// A texture nothing shows, read back by [`Gpu::capture`].
    #[cfg(test)]
    Offscreen { texture: wgpu::Texture },
}

impl Target {
    /// The target's size in its own pixels.
    fn size(&self) -> (u32, u32) {
        match self {
            Target::Window { config, .. } => (config.width, config.height),
            #[cfg(test)]
            Target::Offscreen { texture } => (texture.width(), texture.height()),
        }
    }

    /// Ask for another frame, where there is anything to ask.
    fn request_redraw(&self) {
        match self {
            Target::Window { window, .. } => window.request_redraw(),
            #[cfg(test)]
            Target::Offscreen { .. } => {}
        }
    }

    /// Tell the windowing system a frame is about to be presented, where there is one to tell.
    fn pre_present_notify(&self) {
        match self {
            Target::Window { window, .. } => window.pre_present_notify(),
            #[cfg(test)]
            Target::Offscreen { .. } => {}
        }
    }
}

/// The part of a target a frame is drawn into, in whole target pixels.
///
/// Whole pixels on purpose: the viewport's size is the space a frame is drawn in, so a viewport
/// that ended on a fraction of a pixel would have every skin coordinate land a fraction off the
/// pixel it names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Viewport {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl Viewport {
    fn size(self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// The viewport as `(x, y, w, h)`, which is how a render pass and the scissor arithmetic take
    /// it.
    fn as_rect(self) -> (f32, f32, f32, f32) {
        (self.x as f32, self.y as f32, self.width as f32, self.height as f32)
    }
}

/// The format a surface is configured with and the format frames are rendered to it through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SurfaceFormats {
    surface: wgpu::TextureFormat,
    render: wgpu::TextureFormat,
}

impl SurfaceFormats {
    fn same(format: wgpu::TextureFormat) -> SurfaceFormats {
        SurfaceFormats { surface: format, render: format }
    }
}

/// Pick the formats that put a fragment's bytes on screen untouched, out of what a surface offers.
///
/// A plain 8-bit format is taken as it is. Failing that, an sRGB one is configured and rendered
/// through a view of its plain counterpart, which `reinterpretable` says the backend allows. Only
/// when neither is possible does the surface keep a format with a colour transfer of its own: an
/// sRGB one if it has one, as it always chose, and failing that whatever it lists first. `None`
/// means the surface offers nothing, which is an adapter that cannot draw on it.
fn choose_surface_formats(offered: &[wgpu::TextureFormat], reinterpretable: bool) -> Option<SurfaceFormats> {
    if let Some(plain) = PASSTHROUGH_FORMATS.iter().copied().find(|plain| offered.contains(plain)) {
        return Some(SurfaceFormats::same(plain));
    }
    let viewed = offered.iter().copied().find(|format| format.is_srgb() && PASSTHROUGH_FORMATS.contains(&format.remove_srgb_suffix()));
    match viewed {
        Some(srgb) if reinterpretable => Some(SurfaceFormats { surface: srgb, render: srgb.remove_srgb_suffix() }),
        _ => offered.iter().copied().find(|format| format.is_srgb()).or(offered.first().copied()).map(SurfaceFormats::same),
    }
}

/// Whether a `width` x `height` image is within a backend's longest texture edge.
fn texture_fits(width: u32, height: u32, limit: u32) -> bool {
    width <= limit && height <= limit
}

/// Native instanced-quad renderer. Every `fill_rect` becomes one GPU instance; the whole
/// note field is drawn in a single instanced draw call (no CPU rasterisation / texture
/// upload). Implements `rbms_render::Renderer`, so the playfield/result composers target
/// it directly.
pub(crate) struct Gpu {
    target: Target,
    device: wgpu::Device,
    queue: wgpu::Queue,
    /// The format frames are rendered in. See [`choose_surface_formats`].
    format: wgpu::TextureFormat,
    /// The shape a fitted frame keeps, as a width and a height in any unit.
    shape: (u32, u32),
    /// Where this frame lands in the target. Its size is the space the frame is drawn in.
    viewport: Viewport,
    /// Holds that size for the vertex shaders, rewritten whenever it changes.
    screen_uniform: wgpu::Buffer,
    /// The longest edge of a texture the device accepts.
    max_texture_size: u32,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    instances: wgpu::Buffer,
    instance_cap: usize,
    /// One pipeline per blend mode, indexed by [`blend_index`]; blending is baked into a pipeline.
    textured_pipelines: Vec<wgpu::RenderPipeline>,
    textured_bgl: wgpu::BindGroupLayout,
    textured_instances: wgpu::Buffer,
    textured_cap: usize,
    nearest_sampler: wgpu::Sampler,
    linear_sampler: wgpu::Sampler,
    textures: Vec<Option<GpuTexture>>,
    texture_keys: HashMap<String, TextureId>,
    draw: DrawList,
    clear_color: Color,
    letterbox: bool,
    /// The background image and where it goes, redrawn by [`Renderer::clear`] so it lands under the
    /// frame's own quads without leaving the ordinary submission order.
    background: Option<(TextureId, Rect)>,
    /// Which decode the uploaded background pixels came from, so handing the same frame over again
    /// costs no copy and no upload.
    background_generation: Option<u64>,
    /// The handle those pixels went to.
    background_texture: Option<TextureId>,
}

/// Why the GPU backend could not be brought up. Every variant means the player cannot draw, so the
/// caller reports it and exits rather than panicking on a machine that simply has no usable
/// adapter — a headless CI box or an old GPU, both of which do happen.
#[derive(Debug, thiserror::Error)]
pub(crate) enum GpuError {
    #[error("cannot draw on this window: {0}")]
    Surface(#[from] wgpu::CreateSurfaceError),
    #[error("no graphics adapter this build can use")]
    NoAdapter,
    #[error("the graphics adapter refused a device: {0}")]
    NoDevice(#[from] wgpu::RequestDeviceError),
    #[cfg(test)]
    #[error("this target is shown, not read back")]
    NotOffscreen,
    #[cfg(test)]
    #[error("the frame could not be read back: {0}")]
    Readback(String),
}

fn instance() -> wgpu::Instance {
    wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        flags: wgpu::InstanceFlags::default(),
        memory_budget_thresholds: Default::default(),
        backend_options: Default::default(),
        display: None,
    })
}

/// An adapter that can draw on `surface`, or any adapter at all when there is no surface.
fn request_adapter(instance: &wgpu::Instance, surface: Option<&wgpu::Surface<'_>>) -> Result<wgpu::Adapter, GpuError> {
    pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: surface,
        force_fallback_adapter: false,
    }))
    .map_err(|_| GpuError::NoAdapter)
}

/// A device with the adapter's own texture size limits rather than the portable defaults, so an
/// image is only ever refused for being larger than this machine can actually hold.
fn request_device(adapter: &wgpu::Adapter) -> Result<(wgpu::Device, wgpu::Queue), GpuError> {
    let required_limits = wgpu::Limits::default().using_resolution(adapter.limits());
    Ok(pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor { required_limits, ..Default::default() }))?)
}

impl Gpu {
    /// Bring the backend up on a window. `shape` is the width and height whose proportions a fitted
    /// frame keeps, and `letterbox` is whether the first frame is fitted to them: the choice
    /// [`Gpu::set_letterbox`] makes for the frames after it, made here for the one before any of
    /// them, so a window that opens fitted is never shown stretched first.
    pub(crate) fn new(window: Arc<Window>, shape: (u32, u32), letterbox: bool) -> Result<Gpu, GpuError> {
        let size = window.inner_size();
        let instance = instance();
        let surface = instance.create_surface(window.clone())?;
        let adapter = request_adapter(&instance, Some(&surface))?;
        let (device, queue) = request_device(&adapter)?;

        let caps = surface.get_capabilities(&adapter);
        let reinterpretable = adapter.get_downlevel_capabilities().flags.contains(wgpu::DownlevelFlags::SURFACE_VIEW_FORMATS);
        let formats = choose_surface_formats(&caps.formats, reinterpretable).ok_or(GpuError::NoAdapter)?;
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: formats.surface,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: if formats.render == formats.surface { vec![] } else { vec![formats.render] },
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);
        Ok(Gpu::assemble(device, queue, formats.render, Target::Window { window, surface, config }, shape, letterbox))
    }

    /// Bring the backend up with no window: frames are drawn into a `width` x `height` texture and
    /// come back through [`Gpu::capture`].
    ///
    /// Fails, rather than panicking, on a machine with no adapter to draw with -- a headless CI box
    /// has none -- so a caller can skip what it meant to do.
    #[cfg(test)]
    fn offscreen(width: u32, height: u32, shape: (u32, u32)) -> Result<Gpu, GpuError> {
        let adapter = request_adapter(&instance(), None)?;
        let (device, queue) = request_device(&adapter)?;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("offscreen target"),
            size: wgpu::Extent3d { width: width.max(1), height: height.max(1), depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: OFFSCREEN_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        Ok(Gpu::assemble(device, queue, OFFSCREEN_FORMAT, Target::Offscreen { texture }, shape, false))
    }

    /// [`Gpu::offscreen`] for a test with nothing to check on a machine that has no adapter: `None`
    /// there, so the test can pass without drawing, unless [`REQUIRE_GPU_ENV`] says the comparison
    /// has to happen, in which case the missing backend is the failure.
    #[cfg(test)]
    pub(crate) fn offscreen_if_available(width: u32, height: u32, shape: (u32, u32)) -> Option<Gpu> {
        match Gpu::offscreen(width, height, shape) {
            Ok(gpu) => Some(gpu),
            Err(error) => {
                assert!(std::env::var_os(REQUIRE_GPU_ENV).is_none(), "{REQUIRE_GPU_ENV} is set, but the GPU backend did not come up: {error}");
                None
            }
        }
    }

    /// Build everything that does not depend on what kind of target the frames go to.
    fn assemble(device: wgpu::Device, queue: wgpu::Queue, format: wgpu::TextureFormat, target: Target, shape: (u32, u32), letterbox: bool) -> Gpu {
        let viewport = surface_viewport(target.size(), letterbox.then_some(shape));
        let screen_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("screen"),
            size: SCREEN_UNIFORM_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        write_screen_uniform(&queue, &screen_uniform, viewport.size());

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
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: screen_uniform.as_entire_binding() }],
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
                    array_stride: std::mem::size_of::<ColoredInstance>() as u64,
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

        let instance_cap = INITIAL_INSTANCE_CAPACITY;
        let instances = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instances"),
            size: (instance_cap * std::mem::size_of::<ColoredInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let textured_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("texture"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let textured_shader =
            device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("textured"), source: wgpu::ShaderSource::Wgsl(TEXTURED_SHADER.into()) });
        let textured_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("textured"),
            bind_group_layouts: &[Some(&bgl), Some(&textured_bgl)],
            immediate_size: 0,
        });
        let textured_pipelines = BLEND_MODES
            .iter()
            .map(|mode| {
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("textured"),
                    layout: Some(&textured_layout),
                    vertex: wgpu::VertexState {
                        module: &textured_shader,
                        entry_point: Some("vs"),
                        buffers: &[wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<TexturedInstance>() as u64,
                            step_mode: wgpu::VertexStepMode::Instance,
                            attributes: &TEXTURED_ATTRS,
                        }],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &textured_shader,
                        entry_point: Some("fs"),
                        targets: &[Some(wgpu::ColorTargetState { format, blend: Some(blend_state(*mode)), write_mask: wgpu::ColorWrites::ALL })],
                        compilation_options: Default::default(),
                    }),
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    multiview_mask: None,
                    cache: None,
                })
            })
            .collect();
        let textured_instances = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("textured instances"),
            size: (instance_cap * std::mem::size_of::<TexturedInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let sampler = |filter| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                mag_filter: filter,
                min_filter: filter,
                mipmap_filter: wgpu::MipmapFilterMode::Nearest,
                ..Default::default()
            })
        };

        Gpu {
            target,
            nearest_sampler: sampler(wgpu::FilterMode::Nearest),
            linear_sampler: sampler(wgpu::FilterMode::Linear),
            max_texture_size: device.limits().max_texture_dimension_2d,
            device,
            queue,
            format,
            shape,
            viewport,
            screen_uniform,
            pipeline,
            bind_group,
            instances,
            instance_cap,
            textured_pipelines,
            textured_bgl,
            textured_instances,
            textured_cap: instance_cap,
            textures: Vec::new(),
            texture_keys: HashMap::new(),
            draw: DrawList::default(),
            clear_color: Color::BLACK,
            letterbox,
            background: None,
            background_generation: None,
            background_texture: None,
        }
    }

    /// Number of quad instances queued so far this frame (debug overlay metric).
    pub(crate) fn quad_count(&self) -> usize {
        self.draw.quad_count()
    }

    /// Keep the screen's own shape inside the window, or stretch it to fill.
    ///
    /// Stretching is what the surface has always done and is still the default; a window that is
    /// not 16:9 then shows the field wider or taller than it was drawn, which moves where a note
    /// looks like it is. Fitting instead centres the screen and leaves the rest of the window in
    /// the colour the frame was cleared to.
    ///
    /// The choice takes hold when the frame in progress has been handed over, not in the middle of
    /// it: the fit decides the size of the space a frame is drawn in, and the half of a frame
    /// already queued was laid out for the size it started with.
    pub(crate) fn set_letterbox(&mut self, on: bool) {
        self.letterbox = on;
    }

    /// Ask the window for another frame. A target with no window has nobody to ask.
    pub(crate) fn request_redraw(&self) {
        self.target.request_redraw();
    }

    /// Work out where the next frame lands, and tell the shaders if that changed the size of the
    /// space it is drawn in. Called between frames only: see [`Gpu::set_letterbox`].
    fn settle(&mut self) {
        let viewport = surface_viewport(self.target.size(), self.letterbox.then_some(self.shape));
        if viewport.size() != self.viewport.size() {
            write_screen_uniform(&self.queue, &self.screen_uniform, viewport.size());
        }
        self.viewport = viewport;
    }

    /// Present the queued frame on the window. A target with no window is read with
    /// [`Gpu::capture`] instead, and this does nothing for it.
    pub(crate) fn render(&mut self) {
        let frame = match &self.target {
            Target::Window { surface, config, .. } => match surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
                _ => {
                    surface.configure(&self.device, config);
                    return;
                }
            },
            #[cfg(test)]
            Target::Offscreen { .. } => return,
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor { format: Some(self.format), ..Default::default() });
        let commands = self.encode(&view);
        self.queue.submit([commands.finish()]);
        self.target.pre_present_notify();
        frame.present();
        self.settle();
    }

    /// Draw the queued frame into the offscreen target and hand its pixels back as tightly packed
    /// RGBA8 rows, top row first.
    ///
    /// The bytes are what the target holds, with no colour transfer applied on the way in or out,
    /// so they compare directly against what `rbms_render::CpuCanvas` computes for the same draws.
    #[cfg(test)]
    pub(crate) fn capture(&mut self) -> Result<Vec<u8>, GpuError> {
        let texture = match &self.target {
            Target::Offscreen { texture } => texture.clone(),
            Target::Window { .. } => return Err(GpuError::NotOffscreen),
        };
        let (width, height) = (texture.width(), texture.height());
        let mut commands = self.encode(&texture.create_view(&wgpu::TextureViewDescriptor::default()));

        let stride = batch::padded_row_bytes(width);
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("capture"),
            size: u64::from(stride) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        commands.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(stride), rows_per_image: Some(height) },
            },
            texture.size(),
        );
        self.queue.submit([commands.finish()]);
        self.settle();

        let (sender, mapped) = std::sync::mpsc::channel();
        readback.slice(..).map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).ok();
        });
        self.device.poll(wgpu::PollType::wait_indefinitely()).map_err(|e| GpuError::Readback(e.to_string()))?;
        mapped.recv().map_err(|e| GpuError::Readback(e.to_string()))?.map_err(|e| GpuError::Readback(e.to_string()))?;

        let tight = (width * batch::BYTES_PER_PIXEL) as usize;
        let rows = readback.slice(..).get_mapped_range();
        let pixels = rows.chunks_exact(stride as usize).flat_map(|row| &row[..tight]).copied().collect();
        drop(rows);
        readback.unmap();
        Ok(pixels)
    }

    /// Record the queued frame as one render pass onto `view`.
    fn encode(&mut self, view: &wgpu::TextureView) -> wgpu::CommandEncoder {
        if self.draw.colored.len() > self.instance_cap {
            self.instance_cap = self.draw.colored.len().next_power_of_two();
            self.instances = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("instances"),
                size: (self.instance_cap * std::mem::size_of::<ColoredInstance>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if self.draw.textured.len() > self.textured_cap {
            self.textured_cap = self.draw.textured.len().next_power_of_two();
            self.textured_instances = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("textured instances"),
                size: (self.textured_cap * std::mem::size_of::<TexturedInstance>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if !self.draw.colored.is_empty() {
            self.queue.write_buffer(&self.instances, 0, bytemuck::cast_slice(&self.draw.colored));
        }
        if !self.draw.textured.is_empty() {
            self.queue.write_buffer(&self.textured_instances, 0, bytemuck::cast_slice(&self.draw.textured));
        }

        let c = self.clear_color;
        let channel = |value: u8| f64::from(value) / f64::from(u8::MAX);
        let mut enc = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: channel(c.r), g: channel(c.g), b: channel(c.b), a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            let viewport = self.viewport.as_rect();
            let (vx, vy, vw, vh) = viewport;
            rp.set_viewport(vx, vy, vw, vh, 0.0, 1.0);
            rp.set_bind_group(0, &self.bind_group, &[]);

            let surface = self.target.size();
            for entry in &self.draw.batches {
                let Some((sx, sy, sw, sh)) = scissor_rect(entry.clip, viewport, self.viewport.size(), surface) else {
                    continue;
                };
                rp.set_scissor_rect(sx, sy, sw, sh);
                self.draw_batch(&mut rp, entry);
            }
        }
        enc
    }

    /// Issue one batch as a single instanced draw, in the order it was queued.
    fn draw_batch(&self, rp: &mut wgpu::RenderPass<'_>, entry: &Batch) {
        if entry.count == 0 {
            return;
        }
        match entry.kind {
            BatchKind::Colored => {
                rp.set_pipeline(&self.pipeline);
                rp.set_vertex_buffer(0, self.instances.slice(..));
            }
            BatchKind::Textured { tex, blend, filter } => {
                let Some(Some(texture)) = self.textures.get(tex.0 as usize) else {
                    return;
                };
                rp.set_pipeline(&self.textured_pipelines[blend_index(blend)]);
                rp.set_bind_group(1, if filter == TextureFilter::Linear { &texture.linear } else { &texture.nearest }, &[]);
                rp.set_vertex_buffer(0, self.textured_instances.slice(..));
            }
        }
        rp.draw(0..6, entry.first..entry.first + entry.count);
    }

    /// Hand `rgba` to the GPU under an existing handle, padding rows to the copy alignment.
    fn upload(&self, texture: &wgpu::Texture, rgba: &[u8], width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        let (data, bytes_per_row) = pad_rows_to_alignment(rgba, width, height);
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            &data,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(bytes_per_row), rows_per_image: Some(height) },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );
    }

    /// Allocate a texture of `width` x `height` plus the bind groups that read it.
    fn create_texture(&self, width: u32, height: u32) -> GpuTexture {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("skin texture"),
            size: wgpu::Extent3d { width: width.max(1), height: height.max(1), depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let group = |sampler| {
            self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.textured_bgl,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) },
                ],
            })
        };
        GpuTexture { width, height, nearest: group(&self.nearest_sampler), linear: group(&self.linear_sampler), texture }
    }

    /// Where a physical window position lands in a `space`-sized coordinate space laid over the
    /// viewport the frame is drawn into.
    ///
    /// The inverse of the fit the frame was drawn under, so a click lands on what is under the
    /// cursor either way. A position in a letterbox bar maps outside the space, which is exactly
    /// what the hit test wants: there is nothing there to click. Asked with [`Renderer::size`] the
    /// answer is in the pixels a skin screen draws in; asked with the size the built-in screens are
    /// laid out for, it is in theirs.
    pub(crate) fn position_in_space(&self, x: f32, y: f32, space: (u32, u32)) -> (f32, f32) {
        position_in_viewport(x, y, self.viewport, space)
    }

    pub(crate) fn resize(&mut self, w: u32, h: u32) {
        if w == 0 || h == 0 {
            return;
        }
        match &mut self.target {
            Target::Window { surface, config, .. } => {
                config.width = w;
                config.height = h;
                surface.configure(&self.device, config);
            }
            #[cfg(test)]
            Target::Offscreen { .. } => return,
        }
        self.settle();
    }

    /// Stand an empty slot in for an image too large to upload, and say so once.
    ///
    /// The slot is the handle the caller gets back: it has no size and nothing draws from it. A
    /// key that is refused again keeps its slot and stays quiet, because a refused image offered on
    /// every frame would otherwise report itself sixty times a second.
    fn refuse_texture(&mut self, key: &str, width: u32, height: u32) -> TextureId {
        let known = self.texture_keys.get(key).copied().filter(|id| (id.0 as usize) < self.textures.len());
        let already_refused = known.is_some_and(|id| self.textures[id.0 as usize].is_none());
        if !already_refused {
            let limit = self.max_texture_size;
            notify(Level::Warn, format!("texture {key} is {width}x{height}, past the {limit} pixels this graphics adapter holds; it will not be drawn"));
        }
        match known {
            Some(id) => {
                self.textures[id.0 as usize] = None;
                id
            }
            None => {
                let id = TextureId(self.textures.len() as u32);
                self.textures.push(None);
                self.texture_keys.insert(key.to_string(), id);
                id
            }
        }
    }
}

/// Tell the vertex shaders how large the space a frame is drawn in is.
fn write_screen_uniform(queue: &wgpu::Queue, uniform: &wgpu::Buffer, size: (u32, u32)) {
    queue.write_buffer(uniform, 0, bytemuck::cast_slice(&[size.0 as f32, size.1 as f32, 0.0, 0.0]));
}

impl Renderer for Gpu {
    fn size(&self) -> (u32, u32) {
        self.viewport.size()
    }

    fn clear(&mut self, color: Color) {
        self.draw.clear();
        self.clear_color = color;
        if let Some((tex, params)) = self.background_quad() {
            self.draw_textured_quad(tex, params);
        }
    }

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.draw.push_colored(ColoredInstance::new(rect, color));
    }

    fn register_texture(&mut self, key: &str, rgba: &[u8], width: u32, height: u32) -> TextureId {
        if !texture_fits(width, height, self.max_texture_size) {
            return self.refuse_texture(key, width, height);
        }
        if let Some(&id) = self.texture_keys.get(key)
            && let Some(Some(existing)) = self.textures.get(id.0 as usize)
            && existing.width == width
            && existing.height == height
        {
            self.upload(&existing.texture, rgba, width, height);
            return id;
        }
        let created = self.create_texture(width, height);
        self.upload(&created.texture, rgba, width, height);
        match self.texture_keys.get(key).copied() {
            Some(id) if (id.0 as usize) < self.textures.len() => {
                self.textures[id.0 as usize] = Some(created);
                id
            }
            _ => {
                let id = TextureId(self.textures.len() as u32);
                self.textures.push(Some(created));
                self.texture_keys.insert(key.to_string(), id);
                id
            }
        }
    }

    fn release_texture(&mut self, tex: TextureId) {
        if let Some(slot) = self.textures.get_mut(tex.0 as usize) {
            *slot = None;
        }
        self.texture_keys.retain(|_, id| *id != tex);
        if self.background.is_some_and(|(id, _)| id == tex) {
            self.background = None;
        }
        if self.background_texture == Some(tex) {
            self.background_texture = None;
            self.background_generation = None;
        }
    }

    fn texture_size(&self, tex: TextureId) -> Option<(u32, u32)> {
        self.textures.get(tex.0 as usize).and_then(|t| t.as_ref()).map(|t| (t.width, t.height))
    }

    fn draw_textured_quad(&mut self, tex: TextureId, params: QuadParams) {
        let drawable = matches!(self.texture_size(tex), Some((width, height)) if width > 0 && height > 0);
        if !(drawable && params.dst.w > 0.0 && params.dst.h > 0.0) {
            return;
        }
        let kind = BatchKind::Textured { tex, blend: params.blend, filter: params.filter };
        self.draw.push_textured(kind, TexturedInstance::new(&params));
    }

    fn push_clip(&mut self, rect: Rect) {
        self.draw.push_clip(rect);
    }

    fn pop_clip(&mut self) {
        self.draw.pop_clip();
    }

    fn max_texture_size(&self) -> u32 {
        self.max_texture_size
    }
}

/// The part of a `surface` a frame is drawn into.
///
/// Stretched (`fit` is `None`), that is the whole surface. Fitted, it is the largest rectangle of
/// the proportions `fit` names that the surface holds, centred -- so the remainder is one pair of
/// bars, at the sides or above and below depending on which way the window is the wrong shape.
///
/// The rectangle is rounded to whole pixels and held inside the surface rather than trusted to land
/// there: its size is the space the frame is drawn in, and a viewport that starts a hair outside
/// its attachment is rejected outright. A surface with no size yet is treated as one pixel.
fn surface_viewport(surface: (u32, u32), fit: Option<(u32, u32)>) -> Viewport {
    let (sw, sh) = (surface.0.max(1), surface.1.max(1));
    let Some(shape) = fit else {
        return Viewport { x: 0, y: 0, width: sw, height: sh };
    };
    let (shape_w, shape_h) = (shape.0.max(1) as f32, shape.1.max(1) as f32);
    let scale = (sw as f32 / shape_w).min(sh as f32 / shape_h);
    let width = ((shape_w * scale).round() as u32).clamp(1, sw);
    let height = ((shape_h * scale).round() as u32).clamp(1, sh);
    Viewport { x: (sw - width) / 2, y: (sh - height) / 2, width, height }
}

/// The mapping from a physical window position to a `space`-sized coordinate space laid over
/// `viewport`, split out so it can be checked without a window.
fn position_in_viewport(x: f32, y: f32, viewport: Viewport, space: (u32, u32)) -> (f32, f32) {
    let (vx, vy, vw, vh) = viewport.as_rect();
    ((x - vx) * space.0 as f32 / vw, (y - vy) * space.1 as f32 / vh)
}

/// Point the window's fit at what the DISPLAY settings ask for. A headless canvas has no surface to
/// fit, so there is nothing to do for one.
pub(crate) fn apply_letterbox(canvas: &mut crate::stage::Canvas<'_>, on: bool) {
    match canvas {
        crate::stage::Canvas::Window(gpu) => gpu.set_letterbox(on),
        #[cfg(test)]
        crate::stage::Canvas::Headless(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CH, CW};

    /// The shape the built-in screens are laid out for, and so the one a fitted frame keeps.
    const SHAPE: (u32, u32) = (CW, CH);

    /// A 4:3 window: too tall for the screen's shape, so the bars are above and below it.
    const TALL: (u32, u32) = (1024, 768);

    /// An ultrawide window: too wide, so the bars are at the sides.
    const WIDE: (u32, u32) = (2560, 1080);

    /// How far a fitted viewport's edge may sit from the exact proportion, in pixels: rounding to
    /// whole pixels moves it by up to half of one.
    const ROUNDING_SLACK: f32 = 0.5;

    fn viewport(surface: (u32, u32), letterbox: bool) -> Viewport {
        surface_viewport(surface, letterbox.then_some(SHAPE))
    }

    fn stretched(x: f32, y: f32, w: u32, h: u32) -> (f32, f32) {
        position_in_viewport(x, y, viewport((w, h), false), SHAPE)
    }

    fn fitted(x: f32, y: f32, w: u32, h: u32) -> (f32, f32) {
        position_in_viewport(x, y, viewport((w, h), true), SHAPE)
    }

    #[test]
    fn a_window_at_the_logical_size_maps_a_position_onto_itself() {
        assert_eq!(stretched(0.0, 0.0, CW, CH), (0.0, 0.0));
        assert_eq!(stretched(640.0, 360.0, CW, CH), (640.0, 360.0));
        assert_eq!(stretched(CW as f32, CH as f32, CW, CH), (CW as f32, CH as f32));
    }

    #[test]
    fn a_resized_window_maps_its_corners_onto_the_logical_corners() {
        let (w, h) = (CW * 2, CH * 2);
        assert_eq!(stretched(0.0, 0.0, w, h), (0.0, 0.0));
        assert_eq!(stretched(w as f32, h as f32, w, h), (CW as f32, CH as f32));
        assert_eq!(stretched(w as f32 / 2.0, h as f32 / 2.0, w, h), (CW as f32 / 2.0, CH as f32 / 2.0));
    }

    /// The same physical position answers in whichever space it is asked for: the fixed one the
    /// built-in screens are laid out in, or the viewport's own pixels a skin screen draws in.
    #[test]
    fn a_position_is_given_in_the_space_it_is_asked_for() {
        let surface = (CW * 2, CH * 2);
        let frame = viewport(surface, false);
        assert_eq!(position_in_viewport(640.0, 360.0, frame, SHAPE), (320.0, 180.0), "half as far in the fixed space");
        assert_eq!(position_in_viewport(640.0, 360.0, frame, frame.size()), (640.0, 360.0), "and unmoved in the viewport's own pixels");

        let (wide, bars) = (viewport(WIDE, true), viewport(WIDE, true).x as f32);
        assert_eq!(position_in_viewport(bars + 100.0, 50.0, wide, wide.size()), (100.0, 50.0), "a fitted frame's own pixels start where its bars end");
    }

    /// A window reported as zero-sized (minimised on some platforms) must not divide by zero.
    #[test]
    fn a_window_with_no_size_yet_maps_without_dividing_by_zero() {
        for letterbox in [false, true] {
            let (x, y) = position_in_viewport(10.0, 10.0, viewport((0, 0), letterbox), SHAPE);
            assert!(x.is_finite() && y.is_finite(), "letterbox {letterbox} gave ({x}, {y})");
        }
    }

    #[test]
    fn stretching_fills_the_whole_window() {
        assert_eq!(viewport(TALL, false), Viewport { x: 0, y: 0, width: TALL.0, height: TALL.1 });
        assert_eq!(viewport(WIDE, false), Viewport { x: 0, y: 0, width: WIDE.0, height: WIDE.1 });
    }

    /// The size a frame is drawn in is the viewport's, so it follows the window rather than being
    /// the fixed size it once was.
    #[test]
    fn the_space_a_frame_is_drawn_in_is_the_viewport_in_pixels() {
        assert_eq!(viewport((1920, 1080), false).size(), (1920, 1080));
        assert_eq!(viewport(TALL, false).size(), TALL, "a stretched frame is drawn across the whole window");
        assert_eq!(viewport(TALL, true).size(), (1024, 576), "and a fitted one across what is left inside the bars");
        assert_eq!(viewport(WIDE, true).size(), (1920, 1080));
    }

    #[test]
    fn a_window_of_the_screens_own_shape_is_filled_either_way() {
        let square_on = viewport((CW * 3, CH * 3), true);
        assert_eq!(square_on, Viewport { x: 0, y: 0, width: CW * 3, height: CH * 3 }, "a 16:9 window has no room left over");
        assert_eq!(square_on, viewport((CW * 3, CH * 3), false), "so fitting and stretching agree");
    }

    #[test]
    fn a_window_that_is_too_tall_gets_bars_above_and_below() {
        let Viewport { x, y, width, height } = viewport(TALL, true);
        assert_eq!((x, width), (0, TALL.0), "the full width is used");
        assert_eq!(height, 576, "1024 wide at 16:9");
        assert_eq!(y, (TALL.1 - height) / 2, "and the rest is split evenly");
        assert!(y > 0, "there is a bar to split");
    }

    #[test]
    fn a_window_that_is_too_wide_gets_bars_at_the_sides() {
        let Viewport { x, y, width, height } = viewport(WIDE, true);
        assert_eq!((y, height), (0, WIDE.1), "the full height is used");
        assert_eq!(width, 1920, "1080 tall at 16:9");
        assert_eq!(x, (WIDE.0 - width) / 2);
        assert!(x > 0);
    }

    #[test]
    fn the_fitted_screen_keeps_the_shape_it_was_drawn_at() {
        for (sw, sh) in [TALL, WIDE, (900, 900), (1280, 400), (17, 4000)] {
            let Viewport { x, y, width, height } = viewport((sw, sh), true);
            let scale = (sw as f32 / CW as f32).min(sh as f32 / CH as f32);
            let (exact_w, exact_h) = (CW as f32 * scale, CH as f32 * scale);
            assert!((width as f32 - exact_w).abs() <= ROUNDING_SLACK, "{sw}x{sh} came out {width} wide for {exact_w}");
            assert!((height as f32 - exact_h).abs() <= ROUNDING_SLACK, "{sw}x{sh} came out {height} tall for {exact_h}");
            assert!(x + width <= sw && y + height <= sh, "{sw}x{sh} ran the screen off the window");
        }
    }

    /// A shape other than the default one is kept just the same: the fit follows the value it is
    /// given rather than a size built into the backend.
    #[test]
    fn the_fit_follows_the_shape_it_is_given() {
        let square = surface_viewport(WIDE, Some((1, 1)));
        assert_eq!(square, Viewport { x: (WIDE.0 - WIDE.1) / 2, y: 0, width: WIDE.1, height: WIDE.1 });
        let classic = surface_viewport(WIDE, Some((4, 3)));
        assert_eq!(classic.size(), (1440, 1080));
    }

    #[test]
    fn a_click_inside_the_fitted_screen_lands_where_it_was_drawn() {
        for (sw, sh) in [TALL, WIDE] {
            let Viewport { x, y, width, height } = viewport((sw, sh), true);
            let (x, y, w, h) = (x as f32, y as f32, width as f32, height as f32);
            assert_eq!(fitted(x, y, sw, sh), (0.0, 0.0), "{sw}x{sh} top left");
            let (bx, by) = fitted(x + w, y + h, sw, sh);
            assert!((bx - CW as f32).abs() < 1e-3 && (by - CH as f32).abs() < 1e-3, "{sw}x{sh} bottom right came out ({bx}, {by})");
            let (mx, my) = fitted(x + w * 0.5, y + h * 0.5, sw, sh);
            assert!((mx - CW as f32 * 0.5).abs() < 1e-3 && (my - CH as f32 * 0.5).abs() < 1e-3, "{sw}x{sh} middle came out ({mx}, {my})");
        }
    }

    /// A bar is not part of the screen, so a click in one has to miss everything the frame drew.
    #[test]
    fn a_click_in_a_bar_lands_outside_the_logical_screen() {
        let y = viewport(TALL, true).y as f32;
        let (_, above) = fitted(10.0, y * 0.5, TALL.0, TALL.1);
        assert!(above < 0.0, "a click above the screen came out at {above}");
        let (_, below) = fitted(10.0, TALL.1 as f32 - y * 0.5, TALL.0, TALL.1);
        assert!(below > CH as f32, "a click below the screen came out at {below}");

        let x = viewport(WIDE, true).x as f32;
        let (left, _) = fitted(x * 0.5, 10.0, WIDE.0, WIDE.1);
        assert!(left < 0.0, "a click left of the screen came out at {left}");
        let (right, _) = fitted(WIDE.0 as f32 - x * 0.5, 10.0, WIDE.0, WIDE.1);
        assert!(right > CW as f32, "a click right of the screen came out at {right}");
    }

    /// The whole point of fitting: the same window shows the screen undistorted rather than
    /// stretched, so a click in the middle of the drawn field is not the same physical point.
    #[test]
    fn fitting_and_stretching_disagree_on_a_window_of_the_wrong_shape() {
        assert_ne!(fitted(512.0, 100.0, TALL.0, TALL.1), stretched(512.0, 100.0, TALL.0, TALL.1));
    }

    /// The colour decision in one place: a surface that offers a format without an sRGB transfer
    /// is given it, so the bytes a frame writes are the bytes on screen.
    #[test]
    fn a_plain_format_is_preferred_over_the_srgb_one_a_surface_lists_first() {
        use wgpu::TextureFormat::{Bgra8Unorm, Bgra8UnormSrgb, Rgba8Unorm, Rgba8UnormSrgb, Rgba16Float};
        for reinterpretable in [false, true] {
            assert_eq!(choose_surface_formats(&[Bgra8UnormSrgb, Bgra8Unorm, Rgba16Float], reinterpretable), Some(SurfaceFormats::same(Bgra8Unorm)));
            assert_eq!(choose_surface_formats(&[Rgba8UnormSrgb, Rgba8Unorm], reinterpretable), Some(SurfaceFormats::same(Rgba8Unorm)));
        }
    }

    #[test]
    fn a_surface_with_only_an_srgb_format_is_rendered_through_its_plain_view() {
        use wgpu::TextureFormat::{Bgra8Unorm, Bgra8UnormSrgb, Rgba16Float};
        let viewed = choose_surface_formats(&[Rgba16Float, Bgra8UnormSrgb], true);
        assert_eq!(viewed, Some(SurfaceFormats { surface: Bgra8UnormSrgb, render: Bgra8Unorm }));
    }

    /// Where the backend cannot view a surface in another format there is nothing left to choose,
    /// and the surface gets the format it always got: an sRGB one, or its own first.
    #[test]
    fn a_surface_that_cannot_be_viewed_plainly_keeps_the_format_it_always_had() {
        use wgpu::TextureFormat::{Bgra8UnormSrgb, Rgba16Float};
        assert_eq!(choose_surface_formats(&[Rgba16Float, Bgra8UnormSrgb], false), Some(SurfaceFormats::same(Bgra8UnormSrgb)));
        assert_eq!(choose_surface_formats(&[Rgba16Float], true), Some(SurfaceFormats::same(Rgba16Float)));
        assert_eq!(choose_surface_formats(&[], true), None, "a surface that offers nothing cannot be drawn on");
    }

    #[test]
    fn an_image_is_refused_only_when_an_edge_is_past_the_limit() {
        const LIMIT: u32 = 8192;
        assert!(texture_fits(LIMIT, LIMIT, LIMIT), "an image exactly at the limit still fits");
        assert!(texture_fits(6400, 1200, LIMIT));
        assert!(!texture_fits(LIMIT + 1, 1, LIMIT), "one pixel too wide is too wide however short it is");
        assert!(!texture_fits(1, LIMIT + 1, LIMIT));
    }
}
