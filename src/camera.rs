use ultraviolet::{projection, Rotor3, Vec3};

pub struct Camera {
    pub position: Vec3,
    pub orientation: Rotor3,
    pub settings: CameraSettings,
}

pub struct CameraSettings {
    pub z_near: f32,
    pub z_far: f32,
    pub fov: f32,
    pub aspect_ratio: f32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            z_near: 0.1,
            z_far: 100.0,
            fov: 60.0,
            aspect_ratio: 1.0,
        }
    }
}

impl Camera {
    pub fn new(settings: CameraSettings) -> Self {
        Self {
            position: Vec3::zero(),
            orientation: Rotor3::identity(),
            settings,
        }
    }

    /// Positions the camera
    pub fn view_matrix(&self) -> ultraviolet::Mat4 {
        let translation = ultraviolet::Mat4::from_translation(-self.position);
        let rotation = self.orientation.into_matrix().into_homogeneous();
        rotation * translation
    }

    pub fn projection_matrix(&self) -> ultraviolet::Mat4 {
        projection::rh_ydown::perspective_vk(
            self.settings.fov,
            self.settings.aspect_ratio,
            self.settings.z_near,
            self.settings.z_far,
        )
    }
}
