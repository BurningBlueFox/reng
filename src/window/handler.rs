use std::{error::Error, sync::Arc};

use winit::application::ApplicationHandler;
#[cfg(not(target_arch = "wasm32"))]
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
#[cfg(target_arch = "wasm32")]
use winit::event_loop::EventLoopProxy;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
#[cfg(target_arch = "wasm32")]
use winit::platform::web::{EventLoopExtWebSys, WindowAttributesExtWebSys};
use winit::window::{Window, WindowId};

use super::WindowSpecs;
use crate::{renderer::Renderer, scene::Scene};

type RendererReady = Result<Renderer, Box<dyn Error + Send + Sync>>;

pub struct WindowHandler {
    specs: WindowSpecs,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    scene: Scene,
    #[cfg(target_arch = "wasm32")]
    proxy: Option<EventLoopProxy<RendererReady>>,
    startup_error: Option<Box<dyn Error + Send + Sync>>,
}

impl WindowHandler {
    pub fn new(specs: WindowSpecs, scene: Scene) -> Self {
        tracing::info!(
            title = %specs.title,
            width = specs.width,
            height = specs.height,
            resizable = specs.resizable,
            "Window configured"
        );
        Self {
            specs,
            window: None,
            renderer: None,
            scene,
            #[cfg(target_arch = "wasm32")]
            proxy: None,
            startup_error: None,
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: Box<dyn Error + Send + Sync>) {
        tracing::error!(%error, "Application failed");
        #[cfg(target_arch = "wasm32")]
        crate::web::notify("reng-error", &error.to_string());
        self.startup_error = Some(error);
        event_loop.exit();
    }

    pub fn run(self) -> Result<(), Box<dyn Error>> {
        let event_loop = tracing::info_span!("initialize_event_loop")
            .in_scope(|| EventLoop::<RendererReady>::with_user_event().build())
            .inspect_err(|error| tracing::error!(%error, "Event loop creation failed"))?;
        event_loop.set_control_flow(ControlFlow::Wait);
        tracing::info!("Event loop starting");
        #[cfg(target_arch = "wasm32")]
        {
            let mut app = self;
            app.proxy = Some(event_loop.create_proxy());
            event_loop.spawn_app(app);
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut app = self;
            event_loop
                .run_app(&mut app)
                .inspect_err(|error| tracing::error!(%error, "Event loop failed"))?;
            tracing::info!("Event loop stopped");

            if let Some(error) = app.startup_error {
                return Err(error);
            }
        }

        Ok(())
    }
}

impl ApplicationHandler<RendererReady> for WindowHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let _span = tracing::info_span!(
            "create_window",
            title = %self.specs.title,
            width = self.specs.width,
            height = self.specs.height,
            resizable = self.specs.resizable
        )
        .entered();
        let attributes = Window::default_attributes()
            .with_title(self.specs.title.as_str())
            .with_resizable(self.specs.resizable);
        #[cfg(not(target_arch = "wasm32"))]
        let attributes =
            attributes.with_inner_size(LogicalSize::new(self.specs.width, self.specs.height));
        #[cfg(target_arch = "wasm32")]
        let attributes = attributes.with_append(true);

        match event_loop.create_window(attributes) {
            Ok(window) => {
                tracing::info!(window_id = ?window.id(), "Window created");
                let window = Arc::new(window);
                self.window = Some(window.clone());
                #[cfg(not(target_arch = "wasm32"))]
                self.user_event(event_loop, pollster::block_on(Renderer::new(window)));
                #[cfg(target_arch = "wasm32")]
                if let Some(proxy) = self.proxy.clone() {
                    wasm_bindgen_futures::spawn_local(async move {
                        let result = Renderer::new(window).await;
                        let _ = proxy.send_event(result);
                    });
                }
            }
            Err(error) => {
                tracing::error!(%error, "Window creation failed");
                self.fail(event_loop, error.into());
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = &self.window else {
            return;
        };
        if window.id() != window_id {
            return;
        }
        match event {
            WindowEvent::Resized(size) => {
                let _span = tracing::info_span!(
                    "window_resized",
                    ?window_id,
                    width = size.width,
                    height = size.height
                )
                .entered();
                tracing::info!("Window resized");
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size);
                }
                window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                if let Some(renderer) = &mut self.renderer
                    && let Err(error) = renderer.render(&self.scene)
                {
                    self.fail(event_loop, error);
                }
            }
            WindowEvent::Occluded(false) => window.request_redraw(),
            WindowEvent::CloseRequested => {
                let _span = tracing::info_span!("close_window", ?window_id).entered();
                tracing::info!("Window close requested");
                event_loop.exit();
            }
            _ => {}
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, result: RendererReady) {
        match result {
            Ok(mut renderer) => {
                if let Some(window) = &self.window {
                    renderer.resize(window.inner_size());
                    self.renderer = Some(renderer);
                    window.request_redraw();
                    #[cfg(target_arch = "wasm32")]
                    crate::web::notify("reng-ready", "");
                }
            }
            Err(error) => self.fail(event_loop, error),
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        tracing::info!("Event loop exiting");
    }
}
