pub mod camera_controller;
pub mod freecam_controller;

use ultraviolet::{projection, Mat4, Rotor3, Vec3};

use self::camera_controller::CameraController;

#[derive(Debug)]
pub struct Camera {
    pub position: Vec3,
    pub orientation: Rotor3,
    pub settings: CameraSettings,

    pub view: Mat4,
    pub proj: Mat4,
}

#[derive(Debug)]
pub struct CameraSettings {
    pub z_near: f32,
    pub z_far: f32,
    pub fov: f32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            z_near: 0.1,
            z_far: 100.0,
            fov: 60.0,
        }
    }
}

impl Camera {
    pub fn new(aspect_ratio: f32, settings: CameraSettings) -> Self {
        let position = Vec3::zero();
        let orientation = Rotor3::identity();

        let proj =
            calculate_projection(aspect_ratio, settings.fov, settings.z_near, settings.z_far);

        let view = calculate_view(position, orientation);

        Self {
            position,
            orientation,
            settings,
            proj,
            view,
        }
    }

    /// Positions the camera
    pub fn view_matrix(&self) -> ultraviolet::Mat4 {
        self.view
    }

    pub fn projection_matrix(&self) -> ultraviolet::Mat4 {
        self.proj
    }

    pub fn update_camera(&mut self, controller: &impl CameraController) {
        self.position = controller.position();
        self.orientation = controller.orientation();

        self.view = calculate_view(self.position, self.orientation);
    }

    pub fn update_aspect_ratio(&mut self, aspect_ratio: f32) {
        self.proj[0][0] = -self.proj[1][1].clone() / aspect_ratio;
    }

    /// in world-space
    pub const fn forward() -> Vec3 {
        Vec3::new(0.0, 0.0, -1.0)
    }

    /// in world-space
    pub const fn right() -> Vec3 {
        Vec3::new(1.0, 0.0, 0.0)
    }

    /// in world-space
    pub const fn up() -> Vec3 {
        Vec3::new(0.0, 1.0, 0.0)
    }
}

fn calculate_projection(aspect_ratio: f32, fov: f32, near: f32, far: f32) -> Mat4 {
    projection::rh_yup::perspective_vk(fov.to_radians(), aspect_ratio, near, far)
}

fn calculate_view(position: Vec3, orientation: Rotor3) -> Mat4 {
    let cam_direction = orientation * Camera::forward();
    let target = position + cam_direction;

    Mat4::look_at(position, target, Camera::up())
}
