use std::{
    error::Error,
    sync::{Arc, Mutex},
};

use glam::{Mat3, Mat4, Vec3, Vec4};
use tracing::Instrument;
use wgpu::util::DeviceExt;
use winit::{dpi::PhysicalSize, window::Window};

use super::{text::label_vertices, texture};
use crate::scene::{DirectionalLight, Mesh, Scene};

pub(super) type RenderResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// One vertex as `triangle.wgsl` reads it: clip-space XYZW, linear RGB, then texture UV.
pub(super) type Vertex = [f32; 9];

pub(crate) struct Renderer {
    instance: wgpu::Instance,
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    texture_layout: wgpu::BindGroupLayout,
    /// One bind group per `Scene::textures` entry, in order.
    textures: Vec<wgpu::BindGroup>,
    /// 1×1 white texture for untextured meshes and labels.
    white: wgpu::BindGroup,
    depth: wgpu::TextureView,
    size: PhysicalSize<u32>,
    device_error: Arc<Mutex<Option<String>>>,
}

impl Renderer {
    /// Starts rendering to `window` and uploads `textures`, the scene's PNG images.
    pub(crate) async fn new(window: Arc<Window>, textures: &[&[u8]]) -> RenderResult<Self> {
        async {
            let instance = wgpu::Instance::default();
            let surface = instance.create_surface(window.clone())?;
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    compatible_surface: Some(&surface),
                    ..Default::default()
                })
                .await?;
            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor {
                    label: Some("reng device"),
                    required_limits: wgpu::Limits::downlevel_defaults()
                        .using_resolution(adapter.limits()),
                    ..Default::default()
                })
                .await?;
            let device_error = Arc::new(Mutex::new(None));
            let callback_error = device_error.clone();
            let callback_window = window.clone();
            // shortcut: device loss is fatal; rebuild GPU resources when recovery is needed.
            device.set_device_lost_callback(move |reason, message| {
                if reason != wgpu::DeviceLostReason::Destroyed {
                    tracing::error!(?reason, %message, "GPU device lost");
                    if let Ok(mut error) = callback_error.lock() {
                        *error = Some(format!("GPU device lost: {message}"));
                    }
                    callback_window.request_redraw();
                }
            });
            let callback_error = device_error.clone();
            let callback_window = window.clone();
            device.on_uncaptured_error(Arc::new(move |error| {
                tracing::error!(%error, "GPU validation or allocation failed");
                if let Ok(mut failure) = callback_error.lock() {
                    *failure = Some(format!("GPU error: {error}"));
                }
                callback_window.request_redraw();
            }));
            let size = window.inner_size();
            let config = surface_config(&surface, &adapter, size)?;
            let texture_layout = texture::bind_group_layout(&device);
            let pipeline =
                triangle_pipeline(&device, config.format.add_srgb_suffix(), &texture_layout);
            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("linear sampler"),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            });
            let white = texture::bind_group(
                &device,
                &queue,
                &texture_layout,
                &sampler,
                (1, 1, &[255; 4]),
            )?;
            let textures = textures
                .iter()
                .enumerate()
                .map(|(index, png)| {
                    let (width, height, rgba) = texture::decode_png(png).map_err(|error| {
                        format!(
                            "Scene texture {index} is not a readable PNG ({error}); \
                             if it is a Git LFS pointer, run `git lfs pull`"
                        )
                    })?;
                    texture::bind_group(
                        &device,
                        &queue,
                        &texture_layout,
                        &sampler,
                        (width, height, &rgba),
                    )
                })
                .collect::<RenderResult<_>>()?;
            let depth = depth_view(&device, &config);
            let info = adapter.get_info();
            tracing::info!(adapter = %info.name, backend = ?info.backend, "Renderer initialized");
            let mut renderer = Self {
                instance,
                window,
                surface,
                adapter,
                device,
                queue,
                config,
                pipeline,
                texture_layout,
                textures,
                white,
                depth,
                size,
                device_error,
            };
            renderer.resize(size);
            Ok(renderer)
        }
        .instrument(tracing::info_span!("initialize_renderer"))
        .await
    }

    pub(crate) fn resize(&mut self, size: PhysicalSize<u32>) {
        self.size = size;
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
        // Free now: the web backend otherwise holds every resized depth texture until garbage collection.
        self.depth.texture().destroy();
        self.depth = depth_view(&self.device, &self.config);
        tracing::debug!(
            width = size.width,
            height = size.height,
            "Surface configured"
        );
    }

    pub(crate) fn render(&mut self, scene: &Scene) -> RenderResult<()> {
        let _span = tracing::debug_span!("render_frame").entered();
        if let Some(error) = self
            .device_error
            .lock()
            .map_err(|_| "GPU error state poisoned")?
            .take()
        {
            return Err(error.into());
        }
        if self.size.width == 0 || self.size.height == 0 {
            return Ok(());
        }
        let (frame, suboptimal) = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => (frame, false),
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => (frame, true),
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.resize(self.window.inner_size());
                self.window.request_redraw();
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                tracing::warn!("Surface lost; recreating it");
                self.surface = self.instance.create_surface(self.window.clone())?;
                self.config = surface_config(&self.surface, &self.adapter, self.size)?;
                self.pipeline = triangle_pipeline(
                    &self.device,
                    self.config.format.add_srgb_suffix(),
                    &self.texture_layout,
                );
                self.resize(self.window.inner_size());
                self.window.request_redraw();
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Timeout => {
                tracing::warn!("Surface acquisition timed out; retrying");
                self.window.request_redraw();
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Occluded => return Ok(()),
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err("GPU surface validation failed".into());
            }
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.config.format.add_srgb_suffix()),
            ..Default::default()
        });
        let pixel = (2.0 * self.window.scale_factor() as f32).round();
        let (width, height) = (self.size.width as f32, self.size.height as f32);
        let labels = label_vertices(&scene.debug_labels, width, height, pixel);
        // Nonzero: zero-size surfaces returned above.
        let view_projection = scene.view_projection(width / height);
        // ponytail: transforms every vertex on the CPU and uploads a new buffer each frame; move
        // matrices to a uniform buffer with static per-mesh vertex buffers when meshes grow large.
        let mut vertices: Vec<Vertex> = Vec::new();
        // Consecutive vertex ranges and the texture each one samples.
        let mut draws = Vec::with_capacity(scene.meshes.len() + 1);
        for mesh in &scene.meshes {
            let texture = match mesh.texture {
                Some(index) => self
                    .textures
                    .get(index)
                    .ok_or_else(|| format!("Mesh uses missing scene texture {index}"))?,
                None => &self.white,
            };
            let start = vertices.len() as u32;
            vertices.extend(mesh_vertices(mesh, view_projection, scene.light));
            draws.push((start..vertices.len() as u32, texture));
        }
        let start = vertices.len() as u32;
        vertices.extend(labels);
        draws.push((start..vertices.len() as u32, &self.white));
        let vertex_bytes: Vec<u8> = vertices
            .iter()
            .flatten()
            .flat_map(|value| value.to_ne_bytes())
            .collect();
        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("scene vertices"),
                contents: &vertex_bytes,
                usage: wgpu::BufferUsages::VERTEX,
            });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("reng frame"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("draw scene"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(scene.background),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            // wgpu panics on an empty vertex slice; an empty scene still clears.
            if !vertices.is_empty() {
                pass.set_pipeline(&self.pipeline);
                pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                for (range, texture) in draws {
                    if !range.is_empty() {
                        pass.set_bind_group(0, texture, &[]);
                        pass.draw(range, 0..1);
                    }
                }
            }
        }
        self.queue.submit([encoder.finish()]);
        // Free once the GPU is done: the web backend otherwise holds every frame's vertex buffer
        // until garbage collection, hundreds of megabytes at this scene's size.
        vertex_buffer.destroy();
        self.window.pre_present_notify();
        self.queue.present(frame);
        tracing::debug!("Frame presented");
        if suboptimal {
            self.resize(self.window.inner_size());
        }
        // Draw continuously while frames are presented; occlusion and zero size pause the loop.
        self.window.request_redraw();
        Ok(())
    }
}

/// Clip-space vertices of `mesh` with each vertex color lit by `light` along the vertex normal.
// ponytail: lit per vertex on the CPU and interpolated across each triangle (Gouraud shading);
// light per fragment when point lights or specular highlights need detail inside triangles.
pub(super) fn mesh_vertices(
    mesh: &Mesh,
    view_projection: Mat4,
    light: DirectionalLight,
) -> impl Iterator<Item = Vertex> + '_ {
    // Keep clip-space w so the GPU clips at the near plane and interpolates perspective-correctly.
    let clip_from_model = view_projection * mesh.transform;
    // The inverse transpose keeps normals perpendicular to surfaces under non-uniform scale.
    let normal_from_model = Mat3::from_mat4(mesh.transform).inverse().transpose();
    mesh.triangles
        .as_flattened()
        .iter()
        .map(move |&[x, y, z, r, g, b, u, v, nx, ny, nz]| {
            let [cx, cy, cz, cw] = (clip_from_model * Vec4::new(x, y, z, 1.0)).to_array();
            let normal = (normal_from_model * Vec3::new(nx, ny, nz)).normalize_or_zero();
            let [r, g, b] = (Vec3::new(r, g, b) * light.shade(normal)).to_array();
            [cx, cy, cz, cw, r, g, b, u, v]
        })
}

fn triangle_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    texture_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    // An explicit layout keeps texture bind groups valid when the pipeline is rebuilt.
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("triangle layout"),
        bind_group_layouts: &[Some(texture_layout)],
        immediate_size: 0,
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("triangle shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("triangle.wgsl").into()),
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("triangle pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x3, 2 => Float32x2],
            })],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        // ponytail: no back-face culling; the depth test hides back faces. Cull `Face::Back` and wind
        // the label quads counter-clockwise when fill rate matters.
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

fn depth_view(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("depth"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

fn surface_config(
    surface: &wgpu::Surface<'_>,
    adapter: &wgpu::Adapter,
    size: PhysicalSize<u32>,
) -> RenderResult<wgpu::SurfaceConfiguration> {
    let mut config = surface
        .get_default_config(adapter, size.width.max(1), size.height.max(1))
        .ok_or("GPU adapter does not support this window surface")?;
    // WebGPU canvas formats are unorm; an sRGB view converts the linear scene color.
    config.view_formats = vec![config.format.add_srgb_suffix()];
    Ok(config)
}
