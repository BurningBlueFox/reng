use super::{text::label_vertices, texture::decode_png};

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
