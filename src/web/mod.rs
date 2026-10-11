use wasm_bindgen::prelude::*;

use crate::{
    scene::Scene,
    telemetry,
    window::{WindowHandler, WindowSpecs},
};

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    telemetry::init().map_err(|error| JsValue::from_str(&error.to_string()))?;
    tracing::info!("Application starting");
    WindowHandler::new(WindowSpecs::default(), Scene::default())
        .run()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

pub(crate) fn notify(name: &str, message: &str) {
    let options = web_sys::CustomEventInit::new();
    options.set_detail(&JsValue::from_str(message));
    if let Some(window) = web_sys::window()
        && let Ok(event) = web_sys::CustomEvent::new_with_event_init_dict(name, &options)
    {
        let _ = window.dispatch_event(&event);
    }
}
