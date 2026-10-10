use std::error::Error;

use tracing_subscriber::prelude::*;

pub fn init() -> Result<tracing_chrome::FlushGuard, Box<dyn Error>> {
    use std::fs::File;

    use tracing_chrome::ChromeLayerBuilder;
    use tracing_subscriber::{EnvFilter, filter::LevelFilter};

    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env()?;
    let (chrome_layer, guard) = ChromeLayerBuilder::new()
        .writer(File::create("trace.json")?)
        .include_args(true)
        .build();

    tracing_subscriber::registry()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_thread_names(true)
                .with_thread_ids(true)
                .with_writer(std::io::stdout),
        )
        .with(chrome_layer)
        .try_init()?;

    Ok(guard)
}
