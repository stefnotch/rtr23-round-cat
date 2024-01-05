use crate::{
    loader::{Animation, AnimationKeyframe},
    time::Time,
    transform::Transform,
};

use super::camera_controller::CameraController;

pub struct AnimationCameraController {
    animation: Animation,
    last_keyframe: AnimationKeyframe,

    transform: Transform,
}

impl AnimationCameraController {
    pub fn new(animation: Animation) -> Self {
        Self {
            animation,
            last_keyframe: Default::default(),
            transform: Default::default(),
        }
    }

    pub fn update(&mut self, time: &Time) {
        let elapsed_seconds = time
            .elapsed()
            .as_secs_f32()
            .rem_euclid(self.animation.duration());
        let keyframe = self
            .animation
            .get_keyframe(elapsed_seconds, self.last_keyframe);
        self.last_keyframe = keyframe;

        self.transform = self.animation.sample(keyframe, elapsed_seconds);
    }
}

impl CameraController for AnimationCameraController {
    fn position(&self) -> ultraviolet::Vec3 {
        self.transform.position
    }

    fn orientation(&self) -> ultraviolet::Rotor3 {
        self.transform.orientation
    }
}
