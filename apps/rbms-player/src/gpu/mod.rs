//! Native instanced-quad GPU backend (wgpu). Extracted from `main.rs` so the renderer's wgpu
//! plumbing lives apart from the app/UI state machine. `Gpu` implements `rbms_render::Renderer`,
//! so every UI composer (`render_playfield`/`render_hud`/…) targets it directly.

mod background;
mod batch;

#[cfg(test)]
pub(crate) use background::background_upload_needed;

use std::collections::HashMap;
use std::sync::Arc;

use batch::{Batch, BatchKind, ColoredInstance, DrawList, TexturedInstance, pad_rows_to_alignment, scissor_rect};
use rbms_render::{BlendFactor, BlendMode, Color, QuadParams, Rect, Renderer, TextureFilter, TextureId};
use winit::window::Window;

use crate::{CH, CW};

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

/// Native instanced-quad renderer. Every `fill_rect` becomes one GPU instance; the whole
/// note field is drawn in a single instanced draw call (no CPU rasterisation / texture
/// upload). Implements `rbms_render::Renderer`, so the playfield/result composers target
/// it directly.
pub(crate) struct Gpu {
    pub(crate) window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
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
}

impl Gpu {
    pub(crate) fn new(window: Arc<Window>) -> Result<Gpu, GpuError> {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            flags: wgpu::InstanceFlags::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        });
        let surface = instance.create_surface(window.clone())?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .map_err(|_| GpuError::NoAdapter)?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;

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

        Ok(Gpu {
            window,
            surface,
            nearest_sampler: sampler(wgpu::FilterMode::Nearest),
            linear_sampler: sampler(wgpu::FilterMode::Linear),
            device,
            queue,
            config,
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
            letterbox: false,
            background: None,
            background_generation: None,
            background_texture: None,
        })
    }

    /// Number of quad instances queued so far this frame (debug overlay metric).
    pub(crate) fn quad_count(&self) -> usize {
        self.draw.quad_count()
    }

    /// Keep the logical screen's own shape inside the window, or stretch it to fill.
    ///
    /// Stretching is what the surface has always done and is still the default; a window that is
    /// not 16:9 then shows the field wider or taller than it was drawn, which moves where a note
    /// looks like it is. Fitting instead centres the screen and leaves the rest of the window in
    /// the colour the frame was cleared to.
    pub(crate) fn set_letterbox(&mut self, on: bool) {
        self.letterbox = on;
    }

    /// The part of the surface this frame is drawn into, as `(x, y, w, h)`.
    fn viewport(&self) -> (f32, f32, f32, f32) {
        surface_viewport(self.config.width, self.config.height, self.letterbox)
    }

    pub(crate) fn render(&mut self) {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            _ => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
        };

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
            let viewport = self.viewport();
            let (vx, vy, vw, vh) = viewport;
            rp.set_viewport(vx, vy, vw, vh, 0.0, 1.0);
            rp.set_bind_group(0, &self.bind_group, &[]);

            let surface = (self.config.width, self.config.height);
            for entry in &self.draw.batches {
                let Some((sx, sy, sw, sh)) = scissor_rect(entry.clip, viewport, (CW, CH), surface) else {
                    continue;
                };
                rp.set_scissor_rect(sx, sy, sw, sh);
                self.draw_batch(&mut rp, entry);
            }
        }
        self.queue.submit([enc.finish()]);
        self.window.pre_present_notify();
        frame.present();
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
            BatchKind::Textured { tex, blend, filter, .. } => {
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

    /// Where a physical window position lands in the fixed logical screen the UI is laid out in.
    ///
    /// The inverse of the fit the frame was drawn under, so a click lands on what is under the
    /// cursor either way. A position in a letterbox bar maps outside the logical screen, which is
    /// exactly what the hit test wants: there is nothing there to click.
    pub(crate) fn logical_from_physical(&self, x: f32, y: f32) -> (f32, f32) {
        logical_from_physical_in(x, y, self.config.width, self.config.height, self.letterbox)
    }

    pub(crate) fn resize(&mut self, w: u32, h: u32) {
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
        let kind = BatchKind::Textured { tex, blend: params.blend, filter: params.filter, rotated: params.is_rotated() };
        self.draw.push_textured(kind, TexturedInstance::new(&params));
    }

    fn push_clip(&mut self, rect: Rect) {
        self.draw.push_clip(rect);
    }

    fn pop_clip(&mut self) {
        self.draw.pop_clip();
    }
}

/// The part of a `surface_w` x `surface_h` surface the logical screen is drawn into, as
/// `(x, y, w, h)`.
///
/// Stretched, that is the whole surface. Fitted, it is the largest 16:9 rectangle the surface holds,
/// centred — so the remainder is one pair of bars, at the sides or above and below depending on
/// which way the window is the wrong shape.
///
/// The rectangle is held inside the surface rather than trusted to land there: scaling by a ratio
/// and multiplying back out can overshoot by a fraction of a pixel, and a viewport that starts a
/// hair outside its attachment is rejected outright.
fn surface_viewport(surface_w: u32, surface_h: u32, letterbox: bool) -> (f32, f32, f32, f32) {
    let (sw, sh) = (surface_w.max(1) as f32, surface_h.max(1) as f32);
    if !letterbox {
        return (0.0, 0.0, sw, sh);
    }
    let scale = (sw / CW as f32).min(sh / CH as f32);
    let (w, h) = ((CW as f32 * scale).min(sw), (CH as f32 * scale).min(sh));
    ((sw - w) * 0.5, (sh - h) * 0.5, w, h)
}

/// The mapping from a physical window position to the logical screen, split out so it can be checked
/// without a window.
fn logical_from_physical_in(x: f32, y: f32, surface_w: u32, surface_h: u32, letterbox: bool) -> (f32, f32) {
    let (vx, vy, vw, vh) = surface_viewport(surface_w, surface_h, letterbox);
    ((x - vx) * CW as f32 / vw, (y - vy) * CH as f32 / vh)
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

    /// A 4:3 window: too tall for the screen's shape, so the bars are above and below it.
    const TALL: (u32, u32) = (1024, 768);

    /// An ultrawide window: too wide, so the bars are at the sides.
    const WIDE: (u32, u32) = (2560, 1080);

    fn stretched(x: f32, y: f32, w: u32, h: u32) -> (f32, f32) {
        logical_from_physical_in(x, y, w, h, false)
    }

    fn fitted(x: f32, y: f32, w: u32, h: u32) -> (f32, f32) {
        logical_from_physical_in(x, y, w, h, true)
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

    /// A window reported as zero-sized (minimised on some platforms) must not divide by zero.
    #[test]
    fn a_window_with_no_size_yet_maps_without_dividing_by_zero() {
        for letterbox in [false, true] {
            let (x, y) = logical_from_physical_in(10.0, 10.0, 0, 0, letterbox);
            assert!(x.is_finite() && y.is_finite(), "letterbox {letterbox} gave ({x}, {y})");
        }
    }

    #[test]
    fn stretching_fills_the_whole_window() {
        assert_eq!(surface_viewport(TALL.0, TALL.1, false), (0.0, 0.0, TALL.0 as f32, TALL.1 as f32));
        assert_eq!(surface_viewport(WIDE.0, WIDE.1, false), (0.0, 0.0, WIDE.0 as f32, WIDE.1 as f32));
    }

    #[test]
    fn a_window_of_the_screens_own_shape_is_filled_either_way() {
        let square_on = surface_viewport(CW * 3, CH * 3, true);
        assert_eq!(square_on, (0.0, 0.0, (CW * 3) as f32, (CH * 3) as f32), "a 16:9 window has no room left over");
        assert_eq!(square_on, surface_viewport(CW * 3, CH * 3, false), "so fitting and stretching agree");
    }

    #[test]
    fn a_window_that_is_too_tall_gets_bars_above_and_below() {
        let (x, y, w, h) = surface_viewport(TALL.0, TALL.1, true);
        assert_eq!((x, w), (0.0, TALL.0 as f32), "the full width is used");
        assert_eq!(h, 576.0, "1024 wide at 16:9");
        assert_eq!(y, (TALL.1 as f32 - h) * 0.5, "and the rest is split evenly");
        assert!(y > 0.0, "there is a bar to split");
    }

    #[test]
    fn a_window_that_is_too_wide_gets_bars_at_the_sides() {
        let (x, y, w, h) = surface_viewport(WIDE.0, WIDE.1, true);
        assert_eq!((y, h), (0.0, WIDE.1 as f32), "the full height is used");
        assert_eq!(w, 1920.0, "1080 tall at 16:9");
        assert_eq!(x, (WIDE.0 as f32 - w) * 0.5);
        assert!(x > 0.0);
    }

    #[test]
    fn the_fitted_screen_keeps_the_shape_it_was_drawn_at() {
        for (sw, sh) in [TALL, WIDE, (900, 900), (1280, 400), (17, 4000)] {
            let (x, y, w, h) = surface_viewport(sw, sh, true);
            assert!((w / h - CW as f32 / CH as f32).abs() < 1e-3, "{sw}x{sh} came out {w}x{h}");
            assert!(x >= 0.0 && y >= 0.0, "{sw}x{sh} placed the screen outside the window");
            assert!(x + w <= sw as f32 + 1e-3 && y + h <= sh as f32 + 1e-3, "{sw}x{sh} ran the screen off the window");
        }
    }

    #[test]
    fn a_click_inside_the_fitted_screen_lands_where_it_was_drawn() {
        for (sw, sh) in [TALL, WIDE] {
            let (x, y, w, h) = surface_viewport(sw, sh, true);
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
        let (_, y, _, _) = surface_viewport(TALL.0, TALL.1, true);
        let (_, above) = fitted(10.0, y * 0.5, TALL.0, TALL.1);
        assert!(above < 0.0, "a click above the screen came out at {above}");
        let (_, below) = fitted(10.0, TALL.1 as f32 - y * 0.5, TALL.0, TALL.1);
        assert!(below > CH as f32, "a click below the screen came out at {below}");

        let (x, ..) = surface_viewport(WIDE.0, WIDE.1, true);
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
}
