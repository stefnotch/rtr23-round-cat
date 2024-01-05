use ultraviolet::{Rotor3, Vec3};

pub struct Animation {
    pub timestamps: Vec<f32>,
    // TODO: Refactor to use full Transforms
    pub translations: Vec<Vec3>,
    pub rotations: Vec<Rotor3>,
}
