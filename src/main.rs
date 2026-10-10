mod telemetry;
mod window;

use std::error::Error;

use window::{WindowHandler, WindowSpecs};

fn main() -> Result<(), Box<dyn Error>> {
    let _trace_guard = telemetry::init()?;
    tracing::info!("Application starting");

    let result = WindowHandler::new(WindowSpecs::default()).run();
    if let Err(error) = &result {
        tracing::error!(%error, "Application failed");
    }
    tracing::info!("Application stopped");
    result
}
