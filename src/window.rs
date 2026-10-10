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
        Self {
            specs,
            window: None,
            startup_error: None,
        }
    }

    pub(crate) fn run(mut self) -> Result<(), Box<dyn Error>> {
        let event_loop = EventLoop::new()?;
        event_loop.set_control_flow(ControlFlow::Wait);
        event_loop.run_app(&mut self)?;

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

        let attributes = Window::default_attributes()
            .with_title(self.specs.title.as_str())
            .with_inner_size(LogicalSize::new(self.specs.width, self.specs.height))
            .with_resizable(self.specs.resizable);

        match event_loop.create_window(attributes) {
            Ok(window) => self.window = Some(window),
            Err(error) => {
                self.startup_error = Some(error);
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if let WindowEvent::CloseRequested = event {
            event_loop.exit();
        }
    }
}
