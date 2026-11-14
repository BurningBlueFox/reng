use std::{f32::consts::PI, f64::consts::TAU, time::Duration};

use glam::{Mat4, Quat, Vec2, Vec3, camera::rh::proj::directx};

use super::{OrbitCamera, suzanne::suzanne};

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
    /// Shapes drawn with a depth test: by default a textured die, then a sphere and Suzanne behind
    /// it. Each frame, `Scene::animate` replaces the first one's `transform` with a rotation
    /// about +Y.
    pub meshes: Vec<Mesh>,
    /// Text lines drawn in the top-right corner. Supports digits, `.`, spaces, and the letters in `src/renderer/text.rs`.
    pub debug_labels: Vec<String>,
}

/// A shape drawn as a list of triangles.
// ponytail: non-indexed; add an index buffer when meshes share many vertices.
pub struct Mesh {
    /// Three vertices per triangle, each a model-space XYZ position, a linear RGB color, a UV
    /// texture coordinate with (0, 0) at the texture's top-left and (1, 1) at its bottom-right,
    /// and a model-space XYZ normal of any nonzero length pointing out of the front. Flat faces
    /// repeat the face normal on each corner; smooth surfaces give each vertex the surface's own
    /// normal. Wind front faces counter-clockwise seen from outside; both sides are drawn until
    /// back-face culling is enabled.
    pub triangles: Vec<[[f32; 11]; 3]>,
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
fn cube() -> Vec<[[f32; 11]; 3]> {
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
                let (p, uv, n) = (corners[i], uvs[i], normal);
                [
                    p.x, p.y, p.z, color.x, color.y, color.z, uv.x, uv.y, n.x, n.y, n.z,
                ]
            }));
        }
    }
    triangles
}

/// Unit-radius sphere around the origin colored `color`: `rings` bands from the +Y pole to the -Y
/// pole, each split into `segments` around Y, with normals pointing straight out for smooth
/// shading. UVs wrap the whole texture around it once.
fn sphere(segments: u32, rings: u32, color: Vec3) -> Vec<[[f32; 11]; 3]> {
    let vertex = |ring: u32, segment: u32| {
        let (u, v) = (segment as f32 / segments as f32, ring as f32 / rings as f32);
        // Polar angle from +Y, then azimuth from +Z toward +X.
        let (sin_polar, cos_polar) = (v * PI).sin_cos();
        let (sin_azimuth, cos_azimuth) = (u * 2.0 * PI).sin_cos();
        let n = Vec3::new(sin_polar * sin_azimuth, cos_polar, sin_polar * cos_azimuth);
        [
            n.x, n.y, n.z, color.x, color.y, color.z, u, v, n.x, n.y, n.z,
        ]
    };
    let mut triangles = Vec::new();
    for ring in 0..rings {
        for segment in 0..segments {
            // Seen from outside: upper-left, lower-left, lower-right, upper-right.
            let [a, b, c, d] = [
                vertex(ring, segment),
                vertex(ring + 1, segment),
                vertex(ring + 1, segment + 1),
                vertex(ring, segment + 1),
            ];
            // At the poles a and d, or b and c, meet, leaving one triangle instead of two.
            if ring + 1 < rings {
                triangles.push([a, b, c]);
            }
            if ring > 0 {
                triangles.push([a, c, d]);
            }
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
            // From the +X, +Y, and +Z side, above and just behind the starting camera, so the cube's
            // top is brightest, then its +Z face, then its +X face.
            light: DirectionalLight {
                direction: Vec3::new(-0.4, -1.0, -0.7),
                color: Vec3::splat(0.8),
                ambient: Vec3::splat(0.2),
            },
            // The sphere and Suzanne sit above and behind the die, so all three stay in view at the
            // starting camera from wide landscape to portrait tablet screens.
            meshes: vec![
                Mesh {
                    triangles: cube(),
                    transform: Mat4::IDENTITY,
                    texture: Some(0),
                },
                Mesh {
                    triangles: sphere(48, 24, Vec3::splat(0.8)),
                    transform: Mat4::from_scale_rotation_translation(
                        Vec3::splat(0.55),
                        Quat::IDENTITY,
                        Vec3::new(-2.5, 0.5, -3.2),
                    ),
                    texture: None,
                },
                Mesh {
                    triangles: suzanne(Vec3::new(0.8, 0.4, 0.1)),
                    // Turned about +Y to face the starting camera at (1.5, 1.5, 3).
                    transform: Mat4::from_scale_rotation_translation(
                        Vec3::splat(0.5),
                        Quat::from_rotation_y((1.5f32 + 1.1).atan2(3.0 + 3.9)),
                        Vec3::new(-1.1, 0.5, -3.9),
                    ),
                    texture: None,
                },
            ],
            debug_labels: Vec::new(),
        }
    }
}
