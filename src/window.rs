use std::error::Error;

use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::error::OsError;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

pub(crate) struct WindowSpecs {
    pub(crate) title: String,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) resizable: bool,
}

impl Default for WindowSpecs {
    fn default() -> Self {
        Self {
            title: "reng".into(),
            width: 960,
            height: 540,
            resizable: true,
        }
    }
}

pub(crate) struct WindowHandler {
    specs: WindowSpecs,
    window: Option<Window>,
    startup_error: Option<OsError>,
}

impl WindowHandler {
    pub(crate) fn new(specs: WindowSpecs) -> Self {
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
            startup_error: None,
        }
    }

    pub(crate) fn run(mut self) -> Result<(), Box<dyn Error>> {
        let event_loop = tracing::info_span!("initialize_event_loop")
            .in_scope(EventLoop::new)
            .inspect_err(|error| tracing::error!(%error, "Event loop creation failed"))?;
        event_loop.set_control_flow(ControlFlow::Wait);
        tracing::info!("Event loop starting");
        event_loop
            .run_app(&mut self)
            .inspect_err(|error| tracing::error!(%error, "Event loop failed"))?;
        tracing::info!("Event loop stopped");

        if let Some(error) = self.startup_error {
            return Err(error.into());
        }

        Ok(())
    }
}

impl ApplicationHandler for WindowHandler {
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
            .with_inner_size(LogicalSize::new(self.specs.width, self.specs.height))
            .with_resizable(self.specs.resizable);

        match event_loop.create_window(attributes) {
            Ok(window) => {
                tracing::info!(window_id = ?window.id(), "Window created");
                self.window = Some(window);
            }
            Err(error) => {
                tracing::error!(%error, "Window creation failed");
                self.startup_error = Some(error);
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
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
            }
            WindowEvent::CloseRequested => {
                let _span = tracing::info_span!("close_window", ?window_id).entered();
                tracing::info!("Window close requested");
                event_loop.exit();
            }
            _ => {}
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        tracing::info!("Event loop exiting");
    }
}
