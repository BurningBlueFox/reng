use wasm_bindgen::prelude::*;

use crate::{
    telemetry,
    window::{WindowHandler, WindowSpecs},
};

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    telemetry::init().map_err(|error| JsValue::from_str(&error.to_string()))?;
    tracing::info!("Application starting");
    WindowHandler::new(WindowSpecs::default())
        .run()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}
