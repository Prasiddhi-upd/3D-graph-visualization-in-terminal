use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, MouseButton},
};
use crate::camera::Camera;
use winit::event::MouseScrollDelta;

pub struct InputState {
    pub left_mouse_pressed: bool,
    pub middle_mouse_pressed: bool,
    pub last_mouse_pos: Option<(f64, f64)>, // Option to track initial press
}

impl InputState {
    /// Handle mouse wheel zoom
    pub fn on_scroll(&mut self, delta: &MouseScrollDelta, camera: &mut Camera) {
        let scroll_amount = match delta {
            MouseScrollDelta::LineDelta(_, y) => *y as f32 * 0.1, 
            MouseScrollDelta::PixelDelta(pos) => pos.y as f32 * 0.001, 
        };

        camera.zoom(-scroll_amount);
    }

    /// Handle cursor movement for rotation/panning
    pub fn on_cursor_move(&mut self, position: PhysicalPosition<f64>, camera: &mut Camera) {
        if let Some((last_x, last_y)) = self.last_mouse_pos {
            let dx = (position.x - last_x) as f32;
            let dy = (position.y - last_y) as f32;

            if self.left_mouse_pressed {
                camera.rotate(dx, dy);
            } else if self.middle_mouse_pressed {
                camera.pan(dx, dy);
            }
        }
        self.last_mouse_pos = Some((position.x, position.y));
    }

    /// Handle mouse button presses
    pub fn on_mouse(&mut self, button: MouseButton, state: ElementState) {
        let pressed = state == ElementState::Pressed;
        match button {
            MouseButton::Left => self.left_mouse_pressed = pressed,
            MouseButton::Middle => self.middle_mouse_pressed = pressed,
            _ => {}
        }

        // Reset last_mouse_pos on initial press to avoid huge jump
        if pressed {
            self.last_mouse_pos = None;
        }
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            left_mouse_pressed: false,
            middle_mouse_pressed: false,
            last_mouse_pos: None,
        }
    }
}