use std::{collections::HashSet, time::Duration};

use glam::Vec2;
use winit::{
    event::{MouseScrollDelta, TouchPhase},
    keyboard::KeyCode,
};

use crate::scene::OrbitCamera;

/// Radians the camera circles per logical pixel of drag.
const ROTATE_PER_PIXEL: f32 = 0.01;
/// Distance factor per scroll line; scrolling up or away zooms in.
const ZOOM_PER_LINE: f32 = 0.9;
/// Logical pixels of smooth scrolling that count as one line.
const PIXELS_PER_LINE: f32 = 100.0;
/// World units per second that W, A, S and D move the camera.
const MOVE_SPEED: f32 = 2.0;
/// Longest frame step applied to held keys, so a stalled frame does not jump the camera.
const MAX_STEP: Duration = Duration::from_millis(100);

/// Turns keyboard, mouse, wheel, pinch and touch input into camera moves.
/// Positions are in logical pixels.
#[derive(Default)]
pub(super) struct CameraControls {
    /// The view Space returns to.
    home: OrbitCamera,
    held: HashSet<KeyCode>,
    /// Whether Ctrl is held, which marks a browser's trackpad pinch.
    ctrl: bool,
    cursor: Option<Vec2>,
    dragging: bool,
    /// Fingers on the surface, oldest first.
    touches: Vec<(u64, Vec2)>,
}

impl CameraControls {
    /// Controls whose Space key returns the camera to `home`.
    pub(super) fn new(home: OrbitCamera) -> Self {
        Self {
            home,
            ..Self::default()
        }
    }

    /// Space resets the camera; W, A, S and D move it while held.
    pub(super) fn key(
        &mut self,
        code: KeyCode,
        pressed: bool,
        repeat: bool,
        camera: &mut OrbitCamera,
    ) {
        if code == KeyCode::Space {
            if pressed && !repeat {
                *camera = self.home;
            }
        } else if pressed {
            self.held.insert(code);
        } else if matches!(code, KeyCode::SuperLeft | KeyCode::SuperRight) {
            // Browsers on macOS send no release for keys let go while Command is down.
            self.held.clear();
        } else {
            self.held.remove(&code);
        }
    }

    /// Tracks Ctrl from the window's modifier state.
    pub(super) fn ctrl(&mut self, held: bool) {
        self.ctrl = held;
    }

    /// Starts or ends a drag with the rotate button.
    pub(super) fn button(&mut self, pressed: bool) {
        self.dragging = pressed;
    }

    /// Circles the camera by the pointer movement while dragging.
    pub(super) fn cursor_moved(&mut self, position: Vec2, camera: &mut OrbitCamera) {
        // Touch screens can also send mouse events; the touch path handles those fingers.
        if self.dragging
            && self.touches.is_empty()
            && let Some(last) = self.cursor
        {
            rotate(camera, position - last);
        }
        self.cursor = Some(position);
    }

    /// Zooms for a mouse wheel or trackpad scroll. `scale_factor` converts pixel deltas to
    /// logical pixels.
    pub(super) fn scroll(
        &mut self,
        delta: MouseScrollDelta,
        scale_factor: f64,
        camera: &mut OrbitCamera,
    ) {
        let factor = match delta {
            MouseScrollDelta::LineDelta(_, y) => ZOOM_PER_LINE.powf(y),
            MouseScrollDelta::PixelDelta(position) => {
                let pixels = (position.y / scale_factor) as f32;
                if self.ctrl {
                    // Browsers send a trackpad pinch as a Ctrl scroll of 100 × ln(scale) pixels;
                    // undoing that makes it zoom like a native or touch pinch.
                    (-pixels / 100.0).exp()
                } else {
                    ZOOM_PER_LINE.powf(pixels / PIXELS_PER_LINE)
                }
            }
        };
        camera.zoom(factor);
    }

    /// Zooms for a trackpad pinch; positive `delta` magnifies.
    pub(super) fn pinch(&mut self, delta: f32, camera: &mut OrbitCamera) {
        if delta.is_finite() {
            camera.zoom((-delta).exp());
        }
    }

    /// One finger circles the camera; two fingers zoom by how far apart they move.
    pub(super) fn touch(
        &mut self,
        id: u64,
        phase: TouchPhase,
        position: Vec2,
        camera: &mut OrbitCamera,
    ) {
        let index = self.touches.iter().position(|&(touch, _)| touch == id);
        match (phase, index) {
            (TouchPhase::Started, None) => self.touches.push((id, position)),
            (TouchPhase::Moved, Some(index)) => {
                let before = self.touches[index].1;
                if self.touches.len() == 1 {
                    rotate(camera, position - before);
                } else if index < 2 {
                    let other = self.touches[1 - index].1;
                    let (old_span, new_span) = (before.distance(other), position.distance(other));
                    if old_span > 0.0 && new_span > 0.0 {
                        camera.zoom(old_span / new_span);
                    }
                }
                self.touches[index].1 = position;
            }
            (TouchPhase::Ended | TouchPhase::Cancelled, Some(index)) => {
                self.touches.remove(index);
            }
            _ => {}
        }
    }

    /// Forgets held keys, drags and touches, whose releases may never arrive once the window
    /// loses focus.
    pub(super) fn release(&mut self) {
        self.held.clear();
        self.ctrl = false;
        self.dragging = false;
        self.touches.clear();
    }

    /// Moves the camera for the keys held during a frame that took `frame`.
    pub(super) fn update(&self, frame: Duration, camera: &mut OrbitCamera) {
        let held = |code| f32::from(u8::from(self.held.contains(&code)));
        let direction = Vec2::new(
            held(KeyCode::KeyW) - held(KeyCode::KeyS),
            held(KeyCode::KeyD) - held(KeyCode::KeyA),
        );
        // Normalized so diagonals are no faster than straight moves.
        let step = direction.normalize_or_zero() * MOVE_SPEED * frame.min(MAX_STEP).as_secs_f32();
        camera.pan(step.x, step.y);
    }
}

/// Drags rotate the scene under the pointer: moving right turns the camera left around the
/// target, and moving down raises it.
fn rotate(camera: &mut OrbitCamera, delta: Vec2) {
    camera.orbit(-delta.x * ROTATE_PER_PIXEL, delta.y * ROTATE_PER_PIXEL);
}
