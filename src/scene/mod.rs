mod camera;
mod clock;
mod data;
mod suzanne;

pub use camera::OrbitCamera;
pub use clock::FrameClock;
pub use data::{DirectionalLight, Mesh, Scene};

#[cfg(test)]
mod tests;
