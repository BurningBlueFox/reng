use std::f32::consts::TAU;

use glam::{Mat4, Vec3, camera::rh::view::look_at_mat4};

/// Where the camera starts relative to its target: above and to the right, so the +X, +Y and
/// +Z faces of a cube at the target show.
const START_OFFSET: Vec3 = Vec3::new(1.5, 1.5, 3.0);
/// Keeps the camera off the poles, where looking straight along Y has no defined "up".
const MAX_PITCH: f32 = 1.5;
/// Closest and farthest the camera may be from its target, in world units.
const DISTANCE_RANGE: (f32, f32) = (1.5, 50.0);

/// A camera that circles a target point and always looks at it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrbitCamera {
    /// The point the camera looks at and circles.
    pub target: Vec3,
    /// Turn about +Y in radians; 0 puts the camera on the target's +Z side.
    pub yaw: f32,
    /// Height above the target's horizontal plane in radians.
    pub pitch: f32,
    /// World units from the target.
    pub distance: f32,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        let distance = START_OFFSET.length();
        Self {
            target: Vec3::ZERO,
            yaw: START_OFFSET.x.atan2(START_OFFSET.z),
            pitch: (START_OFFSET.y / distance).asin(),
            distance,
        }
    }
}

impl OrbitCamera {
    /// The camera's position in world space.
    pub fn eye(&self) -> Vec3 {
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        self.target + self.distance * Vec3::new(cos_pitch * sin_yaw, sin_pitch, cos_pitch * cos_yaw)
    }

    /// World-to-camera transform: right-handed, Y up, looking down -Z.
    pub fn view(&self) -> Mat4 {
        look_at_mat4(self.eye(), self.target, Vec3::Y)
    }

    /// Circles the target by `yaw` and `pitch` radians. Pitch stops short of straight up or down.
    pub fn orbit(&mut self, yaw: f32, pitch: f32) {
        self.yaw = (self.yaw + yaw).rem_euclid(TAU);
        self.pitch = (self.pitch + pitch).clamp(-MAX_PITCH, MAX_PITCH);
    }

    /// Multiplies the distance to the target by `factor`; below 1 moves closer.
    pub fn zoom(&mut self, factor: f32) {
        self.distance = (self.distance * factor).clamp(DISTANCE_RANGE.0, DISTANCE_RANGE.1);
    }

    /// Slides the target, and the camera with it, across the horizontal plane: `forward` units
    /// the way the camera faces and `right` units to its right.
    pub fn pan(&mut self, forward: f32, right: f32) {
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let ahead = Vec3::new(-sin_yaw, 0.0, -cos_yaw);
        let side = Vec3::new(cos_yaw, 0.0, -sin_yaw);
        self.target += ahead * forward + side * right;
    }
}
