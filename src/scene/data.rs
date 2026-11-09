/// Data consumed by the renderer. Color components are linear in 0..=1.
pub struct Scene {
    pub background: wgpu::Color,
    /// Three vertices containing clip-space XYZ positions followed by RGB colors.
    pub triangle: [[f32; 6]; 3],
    /// Text lines drawn in the top-right corner. Supports digits, `.`, spaces, and the letters in `src/renderer/text.rs`.
    pub debug_labels: Vec<String>,
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
            debug_labels: Vec::new(),
        }
    }
}
