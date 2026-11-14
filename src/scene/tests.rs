use std::{f32::consts::FRAC_PI_2, time::Duration};

use glam::{Mat4, Vec2, Vec3};
use web_time::Instant;

use super::{FrameClock, Mesh, OrbitCamera, Scene};

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
    assert_eq!(clock.frame(), Duration::from_micros(16_667));
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
        // At the origin, looking down -Z: an identity view.
        camera: OrbitCamera {
            target: Vec3::new(0.0, 0.0, -3.0),
            yaw: 0.0,
            pitch: 0.0,
            distance: 3.0,
        },
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
        texture: None,
    });
    scene.animate(Duration::from_secs_f64(std::f64::consts::FRAC_PI_2));
    let turned = scene.meshes[0].transform.transform_point3(Vec3::X);
    assert!(turned.abs_diff_eq(Vec3::NEG_Z, 1e-6), "{turned}");
    assert_eq!(scene.meshes[1].transform, Mat4::from_translation(Vec3::X));
}

#[test]
fn camera_starts_above_and_right_of_the_cube_looking_at_it() {
    let camera = OrbitCamera::default();
    assert!(camera.eye().abs_diff_eq(Vec3::new(1.5, 1.5, 3.0), 1e-5));
    let target = camera.view().transform_point3(camera.target);
    assert!(target.abs_diff_eq(Vec3::new(0.0, 0.0, -camera.distance), 1e-5));
}

#[test]
fn camera_orbit_wraps_yaw_and_stops_short_of_the_poles() {
    let mut camera = OrbitCamera::default();
    let start = camera.yaw;
    camera.orbit(std::f32::consts::TAU + 0.5, 10.0);
    assert!((camera.yaw - (start + 0.5)).abs() < 1e-5);
    assert!(camera.pitch < FRAC_PI_2);
    camera.orbit(0.0, -20.0);
    assert!(camera.pitch > -FRAC_PI_2);
    assert!(camera.view().is_finite());
}

#[test]
fn camera_zoom_stays_outside_the_cube_and_within_the_far_plane() {
    let mut camera = OrbitCamera::default();
    camera.zoom(0.0);
    assert!(camera.distance > 3f32.sqrt() / 2.0, "inside the unit cube");
    camera.zoom(1e9);
    assert!(camera.distance < 100.0, "beyond the far plane");
}

#[test]
fn camera_pan_moves_the_way_it_faces() {
    let mut camera = OrbitCamera {
        yaw: 0.0,
        ..OrbitCamera::default()
    };
    camera.pan(1.0, 2.0);
    assert!(camera.target.abs_diff_eq(Vec3::new(2.0, 0.0, -1.0), 1e-6));

    // From the +X side the camera faces -X, with -Z to its right.
    let mut camera = OrbitCamera {
        yaw: FRAC_PI_2,
        ..OrbitCamera::default()
    };
    camera.pan(1.0, 2.0);
    assert!(camera.target.abs_diff_eq(Vec3::new(-1.0, 0.0, -2.0), 1e-6));
    let eye_offset = camera.eye() - camera.target;
    assert!(
        eye_offset.abs_diff_eq(
            OrbitCamera {
                yaw: FRAC_PI_2,
                ..OrbitCamera::default()
            }
            .eye(),
            1e-5
        )
    );
}

#[test]
fn cube_faces_map_to_dice_cells_with_opposites_adding_to_seven() {
    let mesh = &Scene::default().meshes[0];
    assert_eq!(mesh.texture, Some(0));
    let mut pips_by_normal = Vec::new();
    for face in mesh.triangles.chunks(2) {
        let [a, b, c] = face[0].map(|v| Vec3::from_slice(&v));
        let normal = (b - a).cross(c - a).normalize().round();
        let vertices = face.as_flattened();
        let (min, max) = vertices.iter().fold(
            (Vec2::splat(f32::MAX), Vec2::splat(f32::MIN)),
            |(min, max), v| {
                (
                    min.min(Vec2::new(v[6], v[7])),
                    max.max(Vec2::new(v[6], v[7])),
                )
            },
        );
        // Each face covers exactly one cell of the 3×2 atlas.
        assert!(
            (max - min).abs_diff_eq(Vec2::new(1.0 / 3.0, 0.5), 1e-6),
            "{min} {max}"
        );
        let cell = min * Vec2::new(3.0, 2.0);
        let pips = cell.y.round() as u32 * 3 + cell.x.round() as u32 + 1;
        pips_by_normal.push((normal, pips));
    }
    let mut all: Vec<u32> = pips_by_normal.iter().map(|&(_, pips)| pips).collect();
    all.sort();
    assert_eq!(all, [1, 2, 3, 4, 5, 6]);
    for &(normal, pips) in &pips_by_normal {
        let opposite = pips_by_normal
            .iter()
            .find(|(n, _)| *n == -normal)
            .unwrap()
            .1;
        assert_eq!(pips + opposite, 7, "faces {normal} and {}", -normal);
    }
}
