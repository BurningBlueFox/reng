use glam::{Mat4, Vec3};

use super::{gpu::mesh_vertices, text::label_vertices, texture::decode_png};
use crate::scene::{DirectionalLight, Mesh};

#[test]
fn right_aligns_label_pixels_in_top_right_corner() {
    // "." lights a 2×2 block in columns 1..3 and rows 5..7 of its 5×7 cell.
    let vertices = label_vertices(&[" .".into()], 100.0, 50.0, 1.0);
    assert_eq!(vertices.len(), 4 * 6);
    let xs = vertices.iter().map(|v| (v[0] + 1.0) / 2.0 * 100.0);
    let ys = vertices.iter().map(|v| (1.0 - v[1]) / 2.0 * 50.0);
    // Right edge of the 11-pixel line sits at 100 - 4 margin; "." spans pixels 7..9 of it.
    assert_eq!(xs.clone().fold(f32::MAX, f32::min), 85.0 + 7.0);
    assert_eq!(xs.fold(f32::MIN, f32::max), 85.0 + 9.0);
    assert_eq!(ys.clone().fold(f32::MAX, f32::min), 4.0 + 5.0);
    assert_eq!(ys.fold(f32::MIN, f32::max), 4.0 + 7.0);
    // Depth 0, w = 1, white, UV (0, 0).
    assert!(
        vertices
            .iter()
            .all(|v| v[2..] == [0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0])
    );
}

fn encode_png(color: png::ColorType, width: u32, data: &[u8]) -> Vec<u8> {
    let mut png = Vec::new();
    let height = data.len() as u32 / width / color.samples() as u32;
    let mut encoder = png::Encoder::new(&mut png, width, height);
    encoder.set_color(color);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(data).unwrap();
    writer.finish().unwrap();
    png
}

#[test]
fn decodes_png_color_types_to_rgba() {
    let rgb = encode_png(png::ColorType::Rgb, 2, &[255, 0, 0, 0, 0, 255]);
    assert_eq!(
        decode_png(&rgb).unwrap(),
        (2, 1, vec![255, 0, 0, 255, 0, 0, 255, 255])
    );
    let gray = encode_png(png::ColorType::Grayscale, 1, &[10, 200]);
    assert_eq!(
        decode_png(&gray).unwrap(),
        (1, 2, vec![10, 10, 10, 255, 200, 200, 200, 255])
    );
    let gray_alpha = encode_png(png::ColorType::GrayscaleAlpha, 1, &[7, 128]);
    assert_eq!(decode_png(&gray_alpha).unwrap(), (1, 1, vec![7, 7, 7, 128]));
}

#[test]
fn rejects_git_lfs_pointer_instead_of_png() {
    let pointer = b"version https://git-lfs.github.com/spec/v1\noid sha256:0\nsize 1\n";
    assert!(decode_png(pointer).is_err());
}

#[test]
fn lights_each_vertex_by_its_world_space_normal() {
    // A triangle colored (1, 0.5, 0.25) whose vertex normals all point along (1, 0, 1), unnormalized.
    let mesh = |transform| Mesh {
        triangles: vec![
            [[0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 1.0]]
                .map(|[x, y, z]| [x, y, z, 1.0, 0.5, 0.25, 0.0, 0.0, 2.0, 0.0, 2.0]),
        ],
        transform,
        texture: None,
    };
    let light = DirectionalLight {
        direction: Vec3::new(0.0, 0.0, -3.0),
        color: Vec3::new(0.4, 0.8, 0.8),
        ambient: Vec3::splat(0.1),
    };
    let lit = |transform| {
        let vertices: Vec<_> = mesh_vertices(&mesh(transform), Mat4::IDENTITY, light).collect();
        assert_eq!(vertices.len(), 3);
        assert!(vertices.iter().all(|v| v[4..7] == vertices[0][4..7]));
        Vec3::from_slice(&vertices[0][4..7])
    };
    // Facing the rays at 45°: ambient plus cos 45° of the direct light, times the vertex color.
    let facing = Vec3::splat(0.1) + Vec3::new(0.4, 0.8, 0.8) * std::f32::consts::FRAC_1_SQRT_2;
    let expected = facing * Vec3::new(1.0, 0.5, 0.25);
    assert!(lit(Mat4::IDENTITY).abs_diff_eq(expected, 1e-6));
    // A quarter turn faces it (1, 0, -1), away from the rays: ambient only. Unlike a half turn, this
    // rotation is not symmetric, so it also catches a normal matrix that skips the transpose.
    let away = lit(Mat4::from_rotation_y(std::f32::consts::FRAC_PI_2));
    assert!(
        away.abs_diff_eq(Vec3::new(0.1, 0.05, 0.025), 1e-6),
        "{away}"
    );
    // Stretching X tilts the surface toward the rays: its normal becomes (1/2, 0, 1), not (2, 0, 1).
    let stretched = lit(Mat4::from_scale(Vec3::new(2.0, 1.0, 1.0)));
    let facing = Vec3::splat(0.1) + Vec3::new(0.4, 0.8, 0.8) * (1.0 / 1.25f32.sqrt());
    let expected = facing * Vec3::new(1.0, 0.5, 0.25);
    assert!(stretched.abs_diff_eq(expected, 1e-6), "{stretched}");

    // Smooth shading: each vertex is lit along its own normal, here toward and away from the rays.
    let mut smooth = mesh(Mat4::IDENTITY);
    smooth.triangles[0][1][8..].copy_from_slice(&[0.0, 0.0, -1.0]);
    let vertices: Vec<_> = mesh_vertices(&smooth, Mat4::IDENTITY, light).collect();
    let colors: Vec<Vec3> = vertices
        .iter()
        .map(|v| Vec3::from_slice(&v[4..7]))
        .collect();
    assert!(colors[0].abs_diff_eq(lit(Mat4::IDENTITY), 1e-6));
    assert!(colors[1].abs_diff_eq(Vec3::new(0.1, 0.05, 0.025), 1e-6));
}
