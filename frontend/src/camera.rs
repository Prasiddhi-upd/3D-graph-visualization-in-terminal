use glam::{Mat4, Vec3};

pub struct Camera {
    pub aspect: f32,
    pub fov: f32,
    pub near: f32,
    pub far: f32,

    pub target: Vec3,   // The point we're looking at
    pub distance: f32,  // Distance from camera to target
    pub yaw: f32,       // Rotation around Y axis
    pub pitch: f32,     // Rotation around X axis
    pub up: Vec3,       // Up direction (usually Y)
}

impl Camera {
    pub fn default_with_aspect(aspect: f32) -> Self {
        Self {
            aspect,
            fov: 45.0f32.to_radians(),
            near: 0.1,
            far: 200.0,
            target: Vec3::ZERO,
            distance: 5.0,
            yaw: 0.0,
            pitch: 0.0,
            up: Vec3::Y,
        }
    }

    /// Computes the view matrix based on yaw, pitch, and distance
    pub fn view_matrix(&self) -> Mat4 {
        let rotation = Mat4::from_euler(glam::EulerRot::YXZ, self.yaw, self.pitch, 0.0);
        let offset = rotation.transform_vector3(Vec3::new(0.0, 0.0, self.distance));
        let position = self.target + offset;

        Mat4::look_at_rh(position, self.target, self.up)
    }

    /// Computes the full projection * view matrix
    pub fn build_view_projection_matrix(&self) -> Mat4 {
        let proj = Mat4::perspective_rh(self.fov, self.aspect, self.near, self.far);
        proj * self.view_matrix()
    }

    /// Zoom in/out (mouse scroll)
    pub fn zoom(&mut self, delta: f32) {
        self.distance = (self.distance - delta).clamp(1.0, 100.0);
    }

    /// Rotate camera (right mouse drag)
    pub fn rotate(&mut self, delta_x: f32, delta_y: f32) {
        const ROTATION_SPEED: f32 = 0.005;
        self.yaw -= delta_x * ROTATION_SPEED;
        self.pitch = (self.pitch - delta_y * ROTATION_SPEED).clamp(-1.5, 1.5);
    }

    /// Pan camera (middle mouse drag)
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        const PAN_SPEED: f32 = 0.01;
        // Compute camera's right and up directions in world space
        let rotation = Mat4::from_euler(glam::EulerRot::YXZ, self.yaw, self.pitch, 0.0);
        let right = rotation.x_axis.truncate();
        let up = rotation.y_axis.truncate();

        // Move the target (camera follows)
        self.target -= right * delta_x * PAN_SPEED;
        self.target += up * delta_y * PAN_SPEED;
    }

    /// Update aspect ratio (e.g., when resizing window)
    pub fn set_aspect(&mut self, aspect: f32) {
        self.aspect = aspect;
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::default_with_aspect(16.0 / 9.0)
    }
}