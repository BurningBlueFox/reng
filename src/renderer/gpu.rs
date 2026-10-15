use std::{
    error::Error,
    sync::{Arc, Mutex},
};

use tracing::Instrument;
use winit::{dpi::PhysicalSize, window::Window};

use crate::scene::Scene;

type RenderResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

pub(crate) struct Renderer {
    instance: wgpu::Instance,
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    size: PhysicalSize<u32>,
    device_error: Arc<Mutex<Option<String>>>,
}

impl Renderer {
    pub(crate) async fn new(window: Arc<Window>) -> RenderResult<Self> {
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
            let pipeline = triangle_pipeline(&device, config.format.add_srgb_suffix());
            let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("triangle vertices"),
                size: size_of::<[[f32; 6]; 3]>() as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
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
                vertex_buffer,
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
        tracing::debug!(
            width = size.width,
            height = size.height,
            "Surface configured"
        );
    }

    pub(crate) fn render(&mut self, scene: &Scene) -> RenderResult<()> {
        let _span = tracing::info_span!("render_frame").entered();
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
                self.pipeline =
                    triangle_pipeline(&self.device, self.config.format.add_srgb_suffix());
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
        let vertex_bytes: Vec<u8> = scene
            .triangle
            .iter()
            .flatten()
            .flat_map(|value| value.to_ne_bytes())
            .collect();
        self.queue
            .write_buffer(&self.vertex_buffer, 0, &vertex_bytes);
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
                ..Default::default()
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.draw(0..3, 0..1);
        }
        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        self.queue.present(frame);
        tracing::info!("Frame presented");
        if suboptimal {
            self.resize(self.window.inner_size());
            self.window.request_redraw();
        }
        Ok(())
    }
}

fn triangle_pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("triangle shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("triangle.wgsl").into()),
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("triangle pipeline"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: size_of::<[f32; 6]>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
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
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
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
