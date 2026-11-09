use std::time::Duration;

use super::{Scene, data::GRADIENT_PERIOD};

fn assert_close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 1e-6, "{actual} != {expected}");
}

#[test]
fn midpoint_starts_at_centroid_with_average_color() {
    let mut scene = Scene::default();
    scene.update(Duration::ZERO);
    let vertices = scene.vertices();
    let midpoint = vertices[2];
    let expected = [0.0, -0.2, 0.0, 1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0];
    for (actual, expected) in midpoint.iter().zip(expected) {
        assert_close(*actual, expected);
    }
    assert_eq!(vertices[5], midpoint);
    assert_eq!(vertices[8], midpoint);
}

#[test]
fn vertices_cover_each_edge_once() {
    let scene = Scene::default();
    let [a, b, c] = scene.triangle;
    let vertices = scene.vertices();
    assert_eq!([vertices[0], vertices[1]], [a, b]);
    assert_eq!([vertices[3], vertices[4]], [b, c]);
    assert_eq!([vertices[6], vertices[7]], [c, a]);
}

#[test]
fn midpoint_drifts_slightly_over_time() {
    let mut scene = Scene::default();
    let mut moved = false;
    for millis in (0..5_000).step_by(100) {
        scene.update(Duration::from_millis(millis));
        let [x, y] = scene.gradient_offset;
        assert!(x.hypot(y) <= 0.16 + 1e-6);
        moved |= x.abs() > 0.01 || y.abs() > 0.01;
    }
    assert!(moved);
}

#[test]
fn midpoint_returns_to_centroid_each_period() {
    let mut scene = Scene::default();
    scene.update(GRADIENT_PERIOD / 4);
    assert_close(scene.gradient_offset[0], 0.08);
    scene.update(GRADIENT_PERIOD * 2);
    assert_close(scene.gradient_offset[0], 0.0);
    assert_close(scene.gradient_offset[1], 0.0);
}
