#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(not(target_arch = "wasm32"))]
pub use native::init;
#[cfg(target_arch = "wasm32")]
pub use web::init;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;
