use ultraviolet::{Rotor3, Vec3};

use crate::transform::Transform;

#[derive(Default)]
pub struct Animation {
    pub timestamps: Vec<f32>,
    pub translations: Vec<Vec3>,
    pub rotations: Vec<Rotor3>,
}

impl Animation {
    pub fn duration(&self) -> f32 {
        self.timestamps.last().copied().unwrap_or_default()
    }

    pub fn get_keyframe(
        &self,
        timestamp: f32,
        last_keyframe: AnimationKeyframe,
    ) -> AnimationKeyframe {
        let mut keyframe_index = None;

        if self.timestamps.is_empty() {
            return AnimationKeyframe(0);
        }

        let start = if self.timestamps[last_keyframe.0] > timestamp {
            0
        } else {
            last_keyframe.0
        };

        for i in start..self.timestamps.len() {
            if self.timestamps[i] >= timestamp {
                keyframe_index = Some(i);
                break;
            }
        }

        AnimationKeyframe(keyframe_index.unwrap_or_default())
    }

    pub fn sample(&self, keyframe: AnimationKeyframe, _timestamp: f32) -> Transform {
        // TODO: Interpolation
        let position = self.translations.get(keyframe.0);
        let orientation = self.rotations.get(keyframe.0);

        match (position, orientation) {
            (Some(position), Some(orientation)) => Transform {
                position: *position,
                orientation: *orientation,
                ..Default::default()
            },
            _ => Default::default(),
        }
    }
}

#[derive(Default, Copy, Clone)]
pub struct AnimationKeyframe(usize);
