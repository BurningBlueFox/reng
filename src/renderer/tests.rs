use super::text::label_vertices;

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
    assert!(vertices.iter().all(|v| v[3..] == [1.0, 1.0, 1.0]));
}
