use super::{animation::Animation, LoadedModel};

pub struct LoadedScene {
    pub models: Vec<LoadedModel>,
    pub camera_animations: Vec<Animation>,
}

impl LoadedScene {
    pub fn new() -> Self {
        Self {
            models: Vec::new(),
            camera_animations: Vec::new(),
        }
    }
}
