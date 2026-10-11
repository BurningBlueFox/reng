/// Data consumed by the renderer. Color components are linear RGBA in 0..=1.
pub struct Scene {
    pub background: wgpu::Color,
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
        }
    }
}
