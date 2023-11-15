use super::LoadedModel;

pub struct LoadedScene {
    pub models: Vec<LoadedModel>,
}

impl LoadedScene {
    pub fn new() -> Self {
        Self { models: Vec::new() }
    }
}
