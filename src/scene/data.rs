use std::{f64::consts::TAU, time::Duration};

use glam::{Mat4, Vec2, Vec3, camera::rh::proj::directx};

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
    /// PNG images that meshes sample by index. The renderer reads them once at startup.
    pub textures: Vec<&'static [u8]>,
    /// Lights the meshes; the debug labels are unlit.
    pub light: DirectionalLight,
    /// Shapes drawn with a depth test. Each frame, `Scene::animate` replaces the first one's
    /// `transform` with a rotation about +Y.
    pub meshes: Vec<Mesh>,
    /// Text lines drawn in the top-right corner. Supports digits, `.`, spaces, and the letters in `src/renderer/text.rs`.
    pub debug_labels: Vec<String>,
}

/// A shape drawn as a list of triangles.
// ponytail: non-indexed; add an index buffer when meshes share many vertices.
pub struct Mesh {
    /// Three vertices per triangle, each a model-space XYZ position, a linear RGB color, and a
    /// UV texture coordinate, with (0, 0) at the texture's top-left and (1, 1) at its
    /// bottom-right. Wind front faces counter-clockwise seen from outside; both sides are drawn
    /// until back-face culling is enabled.
    pub triangles: Vec<[[f32; 8]; 3]>,
    /// Model-to-world transform.
    pub transform: Mat4,
    /// Index into `Scene::textures`. Its colors, decoded from sRGB, multiply the vertex colors;
    /// `None` draws the vertex colors alone. Texture alpha is ignored.
    pub texture: Option<usize>,
}

/// Light whose parallel rays reach the whole scene from one direction, like sunlight.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DirectionalLight {
    /// World-space direction the rays travel, of any nonzero length.
    pub direction: Vec3,
    /// Linear RGB light received by a surface facing the rays head-on.
    pub color: Vec3,
    /// Linear RGB light every surface receives, so faces turned away are not black.
    pub ambient: Vec3,
}

impl DirectionalLight {
    /// Light reaching a surface whose unit `normal` points out of its front: the ambient light plus
    /// the direct light scaled by how squarely the surface faces the rays (Lambert diffuse).
    pub(crate) fn shade(&self, normal: Vec3) -> Vec3 {
        let facing = normal.dot(-self.direction.normalize_or_zero()).max(0.0);
        self.ambient + self.color * facing
    }
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
/// UVs map +X/+Y/+Z to the 1/2/3 cells of a 3×2 dice atlas (`dice.png`) and -X/-Y/-Z to the
/// 6/5/4 cells, so opposite faces add up to seven.
fn cube() -> Vec<[[f32; 8]; 3]> {
    let mut triangles = Vec::with_capacity(12);
    for (normal, pips) in [
        (Vec3::X, 1),
        (Vec3::NEG_X, 6),
        (Vec3::Y, 2),
        (Vec3::NEG_Y, 5),
        (Vec3::Z, 3),
        (Vec3::NEG_Z, 4),
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
        // The atlas cell's corners, in the same order: bottom-left, bottom-right, top-right, top-left.
        let cell = Vec2::new(((pips - 1) % 3) as f32, ((pips - 1) / 3) as f32);
        let uvs = [(0.0, 1.0), (1.0, 1.0), (1.0, 0.0), (0.0, 0.0)]
            .map(|(u, v)| (cell + Vec2::new(u, v)) / Vec2::new(3.0, 2.0));
        let color = if normal.element_sum() > 0.0 {
            normal
        } else {
            Vec3::ONE + normal
        };
        for triangle in [[0, 1, 2], [0, 2, 3]] {
            triangles.push(triangle.map(|i| {
                let (p, uv) = (corners[i], uvs[i]);
                [p.x, p.y, p.z, color.x, color.y, color.z, uv.x, uv.y]
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
            textures: vec![include_bytes!("dice.png")],
            // From above, in front, and to the right of the starting camera, so the cube's top is
            // brightest, then its front, then its right side.
            light: DirectionalLight {
                direction: Vec3::new(-0.4, -1.0, -0.7),
                color: Vec3::splat(0.8),
                ambient: Vec3::splat(0.2),
            },
            meshes: vec![Mesh {
                triangles: cube(),
                transform: Mat4::IDENTITY,
                texture: Some(0),
            }],
            debug_labels: Vec::new(),
        }
    }
}
