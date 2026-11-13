use std::{f64::consts::TAU, time::Duration};

use glam::{Mat4, Vec3, camera::rh::proj::directx};

use super::OrbitCamera;

/// Vertical field of view of the camera in radians.
const FOV_Y: f32 = std::f32::consts::FRAC_PI_4;
/// Radians per second that `Scene::animate` turns the first mesh.
const SPIN_SPEED: f64 = 1.0;

/// Data consumed by the renderer. Color components are linear in 0..=1.
pub struct Scene {
    pub background: wgpu::Color,
    /// Where the scene is seen from. Window input moves it.
    pub camera: OrbitCamera,
    /// Shapes drawn with a depth test. Each frame, `Scene::animate` replaces the first one's
    /// `transform` with a rotation about +Y.
    pub meshes: Vec<Mesh>,
    /// Text lines drawn in the top-right corner. Supports digits, `.`, spaces, and the letters in `src/renderer/text.rs`.
    pub debug_labels: Vec<String>,
}

/// A shape drawn as a list of triangles.
// ponytail: non-indexed; add an index buffer when meshes share many vertices.
pub struct Mesh {
    /// Three vertices per triangle, each a model-space XYZ position followed by a linear RGB
    /// color. Wind front faces counter-clockwise seen from outside; both sides are drawn until
    /// back-face culling is enabled.
    pub triangles: Vec<[[f32; 6]; 3]>,
    /// Model-to-world transform.
    pub transform: Mat4,
}

impl Scene {
    /// World-to-clip transform for a surface `aspect` (width / height), in WebGPU clip space:
    /// Y up and depth 0..=1.
    pub(crate) fn view_projection(&self, aspect: f32) -> Mat4 {
        // ponytail: fixed field of view and depth range; make them fields when the camera needs others.
        directx::perspective(FOV_Y, aspect, 0.1, 100.0) * self.camera.view()
    }

    /// Turns the first mesh about +Y to its pose `elapsed` after start.
    // ponytail: only meshes[0] moves; give meshes their own motion when several animate.
    pub(crate) fn animate(&mut self, elapsed: Duration) {
        // Wrap in f64 before narrowing: an f32 angle loses sub-frame precision after about a day.
        let angle = (elapsed.as_secs_f64() * SPIN_SPEED).rem_euclid(TAU) as f32;
        if let Some(mesh) = self.meshes.first_mut() {
            mesh.transform = Mat4::from_rotation_y(angle);
        }
    }
}

/// Unit cube centered on the origin, two counter-clockwise triangles per face.
/// The +X/+Y/+Z faces are red/green/blue; -X/-Y/-Z are cyan/magenta/yellow.
fn cube() -> Vec<[[f32; 6]; 3]> {
    let mut triangles = Vec::with_capacity(12);
    for normal in [
        Vec3::X,
        Vec3::NEG_X,
        Vec3::Y,
        Vec3::NEG_Y,
        Vec3::Z,
        Vec3::NEG_Z,
    ] {
        // u is perpendicular to the normal and v = normal × u, so u × v points outward.
        let u = Vec3::new(normal.z, normal.x, normal.y) * 0.5;
        let v = normal.cross(u);
        let center = normal * 0.5;
        let corners = [
            center - u - v,
            center + u - v,
            center + u + v,
            center - u + v,
        ];
        let color = if normal.element_sum() > 0.0 {
            normal
        } else {
            Vec3::ONE + normal
        };
        for triangle in [[0, 1, 2], [0, 2, 3]] {
            triangles.push(triangle.map(|i| {
                let p = corners[i];
                [p.x, p.y, p.z, color.x, color.y, color.z]
            }));
        }
    }
    triangles
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            background: wgpu::Color {
                r: 0.12,
                g: 0.28,
                b: 0.36,
                a: 1.0,
            },
            camera: OrbitCamera::default(),
            meshes: vec![Mesh {
                triangles: cube(),
                transform: Mat4::IDENTITY,
            }],
            debug_labels: Vec::new(),
        }
    }
}
