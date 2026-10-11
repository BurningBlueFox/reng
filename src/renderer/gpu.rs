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
            let size = window.inner_size();
            let config = surface_config(&surface, &adapter, size)?;
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
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("reng frame"),
            });
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear background"),
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
