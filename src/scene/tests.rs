use std::time::Duration;

use glam::{Mat4, Vec3};
use web_time::Instant;

use super::{FrameClock, Mesh, Scene};

#[test]
fn reports_frame_and_elapsed_time() {
    let start = Instant::now();
    let mut clock = FrameClock::new(start);
    assert_eq!(
        clock.tick(start + Duration::from_millis(1000)),
        ["FRAME 1000.00 MS", "ELAPSED 1.00 S"]
    );
    assert_eq!(
        clock.tick(start + Duration::from_micros(1_016_667)),
        ["FRAME 16.67 MS", "ELAPSED 1.02 S"]
    );
    assert_eq!(clock.elapsed(), Duration::from_micros(1_016_667));
}

#[test]
fn cube_faces_point_outward_on_all_six_sides() {
    let triangles = &Scene::default().meshes[0].triangles;
    assert_eq!(triangles.len(), 12);
    assert!(
        triangles
            .as_flattened()
            .iter()
            .all(|v| v[..3].iter().all(|c| c.abs() == 0.5))
    );
    let normals: Vec<Vec3> = triangles
        .iter()
        .map(|triangle| {
            let [a, b, c] = [0, 1, 2].map(|i| Vec3::from_slice(&triangle[i]));
            let normal = (b - a).cross(c - a).normalize();
            // Counter-clockwise seen from outside: the normal points away from the center.
            assert!(normal.dot(a + b + c) > 0.0, "inward triangle {triangle:?}");
            normal
        })
        .collect();
    for axis in [
        Vec3::X,
        Vec3::NEG_X,
        Vec3::Y,
        Vec3::NEG_Y,
        Vec3::Z,
        Vec3::NEG_Z,
    ] {
        let faces = normals.iter().filter(|n| n.abs_diff_eq(axis, 1e-6)).count();
        assert_eq!(faces, 2, "triangles facing {axis}");
    }
}

#[test]
fn projection_keeps_square_pixels_and_webgpu_depth() {
    let scene = Scene {
        view: Mat4::IDENTITY,
        ..Scene::default()
    };
    for (width, height) in [(960.0, 540.0), (640.0, 480.0), (360.0, 640.0)] {
        let projection = scene.view_projection(width / height);
        let right = projection.project_point3(Vec3::new(0.5, 0.0, -3.0));
        let up = projection.project_point3(Vec3::new(0.0, 0.5, -3.0));
        // Equal world lengths cover equal pixel lengths on any surface shape.
        assert!(
            (right.x * width - up.y * height).abs() < 1e-3,
            "{width}×{height}"
        );
        assert!(up.y > 0.0, "Y must point up in clip space");
        assert!(right.z > 0.0 && right.z < 1.0);
        // The near plane maps to depth 0 (WebGPU), not -1 (OpenGL).
        let near = projection.project_point3(Vec3::new(0.0, 0.0, -0.1));
        assert!(near.z.abs() < 1e-6);
    }
}

#[test]
fn animate_turns_only_the_first_mesh_about_y() {
    let mut scene = Scene::default();
    scene.meshes.push(Mesh {
        triangles: Vec::new(),
        transform: Mat4::from_translation(Vec3::X),
    });
    scene.animate(Duration::from_secs_f64(std::f64::consts::FRAC_PI_2));
    let turned = scene.meshes[0].transform.transform_point3(Vec3::X);
    assert!(turned.abs_diff_eq(Vec3::NEG_Z, 1e-6), "{turned}");
    assert_eq!(scene.meshes[1].transform, Mat4::from_translation(Vec3::X));
}
