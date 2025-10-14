use glam::Mat4;

pub struct Camera {
    pub aspect: f32,
    pub fov: f32,
    pub near: f32,
    pub far: f32,
    pub view: Mat4,
}

impl Camera {
    pub fn default_with_aspect(aspect: f32) -> Self {
        Self {
            aspect,
            fov: 45.0f32.to_radians(),
            near: 0.1,
            far: 100.0,
            view: Mat4::look_at_rh(
                glam::Vec3::new(0.0, 0.0, 2.0),
                glam::Vec3::ZERO,
                glam::Vec3::Y,
            ),
        }
    }

    pub fn set_aspect(&mut self, aspect: f32) {
        self.aspect = aspect;
    }

    pub fn zoom(&mut self, delta: f32) {
        // Simple zoom by moving camera forward/backward
        let mut position = self.view.inverse().transform_point3(glam::Vec3::ZERO);
        position.z += delta;
        self.view = Mat4::look_at_rh(position, glam::Vec3::ZERO, glam::Vec3::Y);
    }

    pub fn build_view_projection_matrix(&self) -> Mat4 {
        let projection = Mat4::perspective_rh(self.fov, self.aspect, self.near, self.far);
        projection * self.view
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::default_with_aspect(16.0 / 9.0)
    }
}