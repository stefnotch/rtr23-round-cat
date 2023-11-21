use std::ops::Mul;

use serde::{Deserialize, Serialize};
use ultraviolet::{Mat4, Rotor3, Vec3};

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Transform {
    pub position: Vec3,
    pub orientation: Rotor3,
    pub scale: Vec3,
}

impl Transform {
    fn transform_point(&self, point: Vec3) -> Vec3 {
        self.position + (self.orientation * (point * self.scale))
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec3::zero(),
            orientation: Rotor3::identity(),
            scale: Vec3::one(),
        }
    }
}

impl From<Transform> for Mat4 {
    fn from(transform: Transform) -> Self {
        let isometry = ultraviolet::Isometry3::new(transform.position, transform.orientation);
        isometry.into_homogeneous_matrix() * Mat4::from_nonuniform_scale(transform.scale)
    }
}

impl Mul<Transform> for &Transform {
    type Output = Transform;

    fn mul(self, rhs: Transform) -> Self::Output {
        Transform {
            position: self.transform_point(rhs.position),
            orientation: self.orientation * rhs.orientation,
            scale: self.scale * rhs.scale,
        }
    }
}
