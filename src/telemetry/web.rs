use std::error::Error;

use tracing_subscriber::prelude::*;

pub fn init() -> Result<(), Box<dyn Error>> {
    use tracing_subscriber::filter::LevelFilter;
    use tracing_web::MakeWebConsoleWriter;

    tracing_subscriber::registry()
        .with(LevelFilter::INFO)
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .without_time()
                .with_writer(MakeWebConsoleWriter::new()),
        )
        .with(tracing_web::performance_layer())
        .try_init()?;
    Ok(())
}
