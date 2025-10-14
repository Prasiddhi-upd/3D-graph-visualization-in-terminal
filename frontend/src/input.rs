use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, KeyEvent},
    keyboard::KeyCode,
};
use glam::{Mat4, Vec3};

use crate::camera::Camera;

pub struct InputState {
    pub mouse_pressed: bool,
    pub last_mouse_pos: (f64, f64),
}

impl InputState {
    pub fn on_keyboard(&mut self, event: &KeyEvent, camera: &mut Camera) {
        if event.state == ElementState::Pressed {
            match event.physical_key {
                winit::keyboard::PhysicalKey::Code(KeyCode::ArrowUp) => camera.zoom(0.1),
                winit::keyboard::PhysicalKey::Code(KeyCode::ArrowDown) => camera.zoom(-0.1),
                _ => {}
            }
        }
    }

    pub fn on_cursor_move(&mut self, position: PhysicalPosition<f64>, camera: &mut Camera) {
        if self.mouse_pressed {
            let dx = (position.x - self.last_mouse_pos.0) as f32 * 0.01;
            let dy = (position.y - self.last_mouse_pos.1) as f32 * 0.01;

            let mut pos = camera.view.inverse().transform_point3(Vec3::ZERO);
            pos.x += dx;
            pos.y -= dy;
            camera.view = Mat4::look_at_rh(pos, Vec3::ZERO, Vec3::Y);
        }
        self.last_mouse_pos = (position.x, position.y);
    }

    pub fn on_mouse(&mut self, state: ElementState) {
        self.mouse_pressed = state == ElementState::Pressed;
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            mouse_pressed: false,
            last_mouse_pos: (0.0, 0.0),
        }
    }
}