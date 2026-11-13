use super::gpu::Vertex;

/// 5×7 bitmap glyphs; each row's low five bits are pixels, most significant bit leftmost.
fn glyph(c: char) -> [u8; 7] {
    match c {
        '0' => [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        '3' => [
            0b11111, 0b00010, 0b00100, 0b00010, 0b00001, 0b10001, 0b01110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
        ],
        '6' => [
            0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
        ],
        '.' => [0, 0, 0, 0, 0, 0b01100, 0b01100],
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'D' => [
            0b11100, 0b10010, 0b10001, 0b10001, 0b10001, 0b10010, 0b11100,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
        'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        _ => [0; 7],
    }
}

/// Builds white clip-space XYZW triangles for `lines`, right-aligned in the top-right corner of a
/// `width`×`height` surface. They sit at depth 0, so they pass the depth test over every mesh.
/// `pixel` is the size of one glyph pixel in physical pixels.
// ponytail: one quad per glyph pixel; switch to a glyph atlas when labels grow beyond debug text.
pub(super) fn label_vertices(lines: &[String], width: f32, height: f32, pixel: f32) -> Vec<Vertex> {
    let margin = 4.0 * pixel;
    let mut vertices = Vec::new();
    for (row, line) in lines.iter().enumerate() {
        let columns = line.chars().count() as f32 * 6.0 - 1.0;
        let left = width - margin - columns * pixel;
        let top = margin + row as f32 * 9.0 * pixel;
        for (index, c) in line.chars().enumerate() {
            for (y, bits) in glyph(c).into_iter().enumerate() {
                for x in 0..5 {
                    if bits & (0b10000 >> x) == 0 {
                        continue;
                    }
                    let x0 = left + (index * 6 + x) as f32 * pixel;
                    let y0 = top + y as f32 * pixel;
                    let corner = |dx: f32, dy: f32| {
                        let px = x0 + dx * pixel;
                        let py = y0 + dy * pixel;
                        [
                            px / width * 2.0 - 1.0,
                            1.0 - py / height * 2.0,
                            0.0,
                            1.0,
                            1.0,
                            1.0,
                            1.0,
                        ]
                    };
                    vertices.extend([
                        corner(0.0, 0.0),
                        corner(1.0, 0.0),
                        corner(0.0, 1.0),
                        corner(1.0, 0.0),
                        corner(1.0, 1.0),
                        corner(0.0, 1.0),
                    ]);
                }
            }
        }
    }
    vertices
}
