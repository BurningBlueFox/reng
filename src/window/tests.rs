use std::time::Duration;

use glam::{Vec2, Vec3};
use winit::{
    dpi::PhysicalPosition,
    event::{MouseScrollDelta, TouchPhase},
    keyboard::KeyCode,
};

use super::controls::CameraControls;
use crate::scene::OrbitCamera;

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-5
}

#[test]
fn mouse_drag_turns_the_camera_only_while_the_button_is_held() {
    let (mut controls, mut camera) = (CameraControls::default(), OrbitCamera::default());
    let start = camera;
    controls.cursor_moved(Vec2::new(100.0, 100.0), &mut camera);
    controls.cursor_moved(Vec2::new(150.0, 100.0), &mut camera);
    assert_eq!(camera, start, "moving without a button");

    controls.button(true);
    // Right turns the camera left around the target; down raises it.
    controls.cursor_moved(Vec2::new(160.0, 120.0), &mut camera);
    assert!(close(camera.yaw, start.yaw - 0.1));
    assert!(close(camera.pitch, start.pitch + 0.2));

    controls.button(false);
    let released = camera;
    controls.cursor_moved(Vec2::new(300.0, 300.0), &mut camera);
    assert_eq!(camera, released);
}

#[test]
fn scrolling_up_zooms_in_for_lines_and_pixels() {
    let (mut controls, mut camera) = (CameraControls::default(), OrbitCamera::default());
    let start = camera.distance;
    controls.scroll(MouseScrollDelta::LineDelta(0.0, 1.0), 1.0, &mut camera);
    assert!(close(camera.distance, start * 0.9));
    // 200 physical pixels at scale 2 are 100 logical pixels: one line back out.
    let delta = MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, -200.0));
    controls.scroll(delta, 2.0, &mut camera);
    assert!(close(camera.distance, start));
}

#[test]
fn trackpad_pinch_zooms_and_ignores_invalid_deltas() {
    let (mut controls, mut camera) = (CameraControls::default(), OrbitCamera::default());
    let start = camera.distance;
    controls.pinch(0.1, &mut camera);
    assert!(close(camera.distance, start * (-0.1f32).exp()));
    let pinched = camera;
    controls.pinch(f32::NAN, &mut camera);
    assert_eq!(camera, pinched);
}

#[test]
fn one_finger_turns_and_two_fingers_zoom() {
    let (mut controls, mut camera) = (CameraControls::default(), OrbitCamera::default());
    let start = camera;
    controls.touch(1, TouchPhase::Started, Vec2::new(100.0, 100.0), &mut camera);
    controls.touch(1, TouchPhase::Moved, Vec2::new(110.0, 100.0), &mut camera);
    assert!(close(camera.yaw, start.yaw - 0.1));

    // Spreading two fingers from 100 to 200 pixels apart halves the distance.
    controls.touch(2, TouchPhase::Started, Vec2::new(210.0, 100.0), &mut camera);
    let turned = camera;
    controls.touch(2, TouchPhase::Moved, Vec2::new(310.0, 100.0), &mut camera);
    assert!(close(camera.distance, turned.distance * 0.5));
    assert_eq!(camera.yaw, turned.yaw);

    // Lifting one finger hands control back to the other without a jump.
    controls.touch(2, TouchPhase::Ended, Vec2::new(310.0, 100.0), &mut camera);
    controls.touch(1, TouchPhase::Moved, Vec2::new(120.0, 100.0), &mut camera);
    assert!(close(camera.yaw, turned.yaw - 0.1));
    controls.touch(
        1,
        TouchPhase::Cancelled,
        Vec2::new(120.0, 100.0),
        &mut camera,
    );
    let lifted = camera;
    controls.touch(1, TouchPhase::Moved, Vec2::new(500.0, 100.0), &mut camera);
    assert_eq!(camera, lifted);
}

#[test]
fn held_keys_move_the_camera_by_frame_time() {
    let (mut controls, mut camera) = (CameraControls::default(), OrbitCamera::default());
    camera.yaw = 0.0; // Facing -Z, with +X to the right.
    controls.key(KeyCode::KeyW, true, false, &mut camera);
    controls.update(Duration::from_millis(50), &mut camera);
    assert!(camera.target.abs_diff_eq(Vec3::new(0.0, 0.0, -0.1), 1e-5));

    // A stalled frame moves at most 100 ms worth, and diagonals are no faster.
    controls.key(KeyCode::KeyD, true, false, &mut camera);
    controls.update(Duration::from_secs(2), &mut camera);
    let step = 0.2 / 2f32.sqrt();
    assert!(
        camera
            .target
            .abs_diff_eq(Vec3::new(step, 0.0, -0.1 - step), 1e-5)
    );

    controls.key(KeyCode::KeyW, false, false, &mut camera);
    controls.key(KeyCode::KeyD, false, false, &mut camera);
    let stopped = camera;
    controls.update(Duration::from_millis(50), &mut camera);
    assert_eq!(camera, stopped);
}

#[test]
fn space_returns_to_the_starting_camera_once_per_press() {
    let home = OrbitCamera {
        target: Vec3::Y,
        distance: 10.0,
        ..OrbitCamera::default()
    };
    let (mut controls, mut camera) = (CameraControls::new(home), home);
    camera.orbit(1.0, 0.5);
    camera.zoom(2.0);
    camera.pan(3.0, 1.0);
    controls.key(KeyCode::Space, true, false, &mut camera);
    assert_eq!(camera, home);

    camera.zoom(2.0);
    controls.key(KeyCode::Space, true, true, &mut camera);
    assert_ne!(camera, home, "auto-repeat");
}

#[test]
fn ctrl_scroll_zooms_like_a_pinch() {
    let (mut controls, mut camera) = (CameraControls::default(), OrbitCamera::default());
    let start = camera.distance;
    // Browsers send a 2x trackpad pinch as a Ctrl scroll of 100 × ln 2 logical pixels.
    let pinch = MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, 2.0 * 100.0 * 2f64.ln()));
    controls.ctrl(true);
    controls.scroll(pinch, 2.0, &mut camera);
    assert!(close(camera.distance, start / 2.0));

    controls.ctrl(false);
    controls.scroll(pinch, 2.0, &mut camera);
    assert!(close(camera.distance, start / 2.0 * 0.9f32.powf(2f32.ln())));
}

#[test]
fn releasing_command_releases_held_keys() {
    let (mut controls, mut camera) = (CameraControls::default(), OrbitCamera::default());
    controls.key(KeyCode::SuperLeft, true, false, &mut camera);
    controls.key(KeyCode::KeyS, true, false, &mut camera);
    // A browser on macOS never reports S released while Command was down.
    controls.key(KeyCode::SuperLeft, false, false, &mut camera);
    controls.update(Duration::from_millis(50), &mut camera);
    assert_eq!(camera, OrbitCamera::default());
}

#[test]
fn losing_focus_releases_keys_and_drags() {
    let (mut controls, mut camera) = (CameraControls::default(), OrbitCamera::default());
    controls.key(KeyCode::KeyW, true, false, &mut camera);
    controls.ctrl(true);
    controls.button(true);
    controls.cursor_moved(Vec2::new(100.0, 100.0), &mut camera);
    controls.touch(1, TouchPhase::Started, Vec2::new(100.0, 100.0), &mut camera);
    controls.release();

    let released = camera;
    controls.update(Duration::from_millis(50), &mut camera);
    controls.cursor_moved(Vec2::new(200.0, 200.0), &mut camera);
    controls.touch(1, TouchPhase::Moved, Vec2::new(200.0, 200.0), &mut camera);
    assert_eq!(camera, released);
    // Ctrl is released too: a pixel scroll zooms at the normal rate.
    let delta = MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, 100.0));
    controls.scroll(delta, 1.0, &mut camera);
    assert!(close(camera.distance, released.distance * 0.9));
}
