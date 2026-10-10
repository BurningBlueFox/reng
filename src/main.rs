mod window;

use std::error::Error;

use window::{WindowHandler, WindowSpecs};

fn main() -> Result<(), Box<dyn Error>> {
    WindowHandler::new(WindowSpecs::default()).run()
}
