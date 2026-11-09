use std::{f64::consts::TAU, time::Duration};

/// How far, in clip space, the gradient midpoint drifts from the triangle's centroid.
const GRADIENT_DRIFT: f32 = 0.08;
/// Time for the gradient midpoint to complete one loop.
pub(super) const GRADIENT_PERIOD: Duration = Duration::from_secs(4);

/// Data consumed by the renderer. Color components are linear in 0..=1.
pub struct Scene {
    pub background: wgpu::Color,
    /// Three vertices containing clip-space XYZ positions followed by RGB colors.
    pub triangle: [[f32; 6]; 3],
    /// Clip-space XY offset of the gradient midpoint from the triangle's centroid.
    pub gradient_offset: [f32; 2],
}

impl Scene {
    /// Advances animation to `total` time since the first frame.
    pub fn update(&mut self, total: Duration) {
        let angle = TAU * total.as_secs_f64() / GRADIENT_PERIOD.as_secs_f64();
        // Circles through the centroid, so the first frame shows the unshifted gradient.
        self.gradient_offset = [
            GRADIENT_DRIFT * angle.sin() as f32,
            GRADIENT_DRIFT * (1.0 - angle.cos() as f32),
        ];
    }

    /// Triangle-list vertices: the triangle split into three triangles that meet at the
    /// gradient midpoint, which carries the average of the corner colors.
    pub fn vertices(&self) -> [[f32; 6]; 9] {
        let mut midpoint = [0.0; 6];
        for corner in &self.triangle {
            for (sum, value) in midpoint.iter_mut().zip(corner) {
                *sum += value / 3.0;
            }
        }
        midpoint[0] += self.gradient_offset[0];
        midpoint[1] += self.gradient_offset[1];

        let [a, b, c] = self.triangle;
        [a, b, midpoint, b, c, midpoint, c, a, midpoint]
    }
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
            triangle: [
                [0.0, 0.6, 0.0, 1.0, 0.0, 0.0],
                [-0.6, -0.6, 0.0, 0.0, 1.0, 0.0],
                [0.6, -0.6, 0.0, 0.0, 0.0, 1.0],
            ],
            gradient_offset: [0.0; 2],
        }
    }
}
