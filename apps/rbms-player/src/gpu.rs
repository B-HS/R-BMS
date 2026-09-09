//! Native instanced-quad GPU backend (wgpu). Extracted from `main.rs` so the renderer's wgpu
//! plumbing lives apart from the app/UI state machine. `Gpu` implements `rbms_render::Renderer`,
//! so every UI composer (`render_playfield`/`render_hud`/…) targets it directly.

use std::sync::Arc;

use rbms_render::{Color, Rect, Renderer};
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

pub(crate) const BGA_DIM: u32 = 256;
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
    quads: Vec<Instance>,
    clear_color: Color,
    bga_pipeline: wgpu::RenderPipeline,
    bga_uniform: wgpu::Buffer,
    bga_tex: wgpu::Texture,
    bga_bind_group: wgpu::BindGroup,
    bga_active: bool,
}

impl Gpu {
    pub(crate) fn new(window: Arc<Window>) -> Gpu {
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
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
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
            fragment: Some(wgpu::FragmentState {
                module: &bga_shader,
                entry_point: Some("fs"),
                targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })],
                compilation_options: Default::default(),
            }),
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

    pub(crate) fn set_bga(&mut self, rgba: &[u8], rect: Rect) {
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

    pub(crate) fn clear_bga(&mut self) {
        self.bga_active = false;
    }

    /// Number of quad instances queued so far this frame (debug overlay metric).
    pub(crate) fn quad_count(&self) -> usize {
        self.quads.len()
    }

    pub(crate) fn render(&mut self) {
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
