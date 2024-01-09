use ultraviolet::{Lerp, Rotor3, Vec3};

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

    pub fn sample(&self, keyframe: AnimationKeyframe, timestamp: f32) -> Transform {
        if self.timestamps.get(keyframe.0).is_none() {
            return Default::default();
        }

        let position = get_and_next(&self.translations, keyframe.0, || Vec3::zero());
        let orientation = get_and_next(&self.rotations, keyframe.0, Rotor3::identity);
        let t = get_and_next(&self.timestamps, keyframe.0, || 0.0);

        let t = (timestamp - t.0) / (t.1 - t.0).max(0.0001);
        Transform {
            position: position.0.lerp(position.1, t),
            orientation: orientation.0.lerp(orientation.1, t),
            ..Default::default()
        }
    }
}

fn get_and_next<T: Copy>(values: &Vec<T>, index: usize, make_default: fn() -> T) -> (T, T) {
    let value = values.get(index).copied().unwrap_or_else(make_default);
    let next_value = values
        .get((index + 1).rem_euclid(values.len()))
        .copied()
        .unwrap_or_else(make_default);

    (value, next_value)
}
#[derive(Default, Copy, Clone)]
pub struct AnimationKeyframe(usize);
