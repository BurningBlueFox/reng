mod clock;
mod data;

pub use clock::FrameClock;
pub use data::{Mesh, Scene};

#[cfg(test)]
mod tests;
