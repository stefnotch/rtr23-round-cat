use ultraviolet::{Rotor3, Vec3};

pub trait CameraController {
    fn position(&self) -> Vec3;
    fn orientation(&self) -> Rotor3;
}
