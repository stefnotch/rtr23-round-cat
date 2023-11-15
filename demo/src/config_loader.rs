use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use ultraviolet::Vec3;

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub scene_path: String,
    pub cached: CachedData,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CachedData {
    pub camera_position: Option<CameraPosition>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CameraPosition {
    pub position: Vec3,
    pub pitch: f32,
    pub yaw: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            scene_path: "assets/scene-local/sponza/sponza.glb".to_string(),
            cached: CachedData::default(),
        }
    }
}

impl Default for CachedData {
    fn default() -> Self {
        Self {
            camera_position: None,
        }
    }
}

impl Config {
    pub fn from_str(value: &str) -> Self {
        serde_json::from_str(value).unwrap()
    }
}

pub struct ConfigFileLoader {
    pub path: PathBuf,
    config: Option<Config>,
}

impl ConfigFileLoader {
    pub fn new(path: &str) -> Self {
        Self {
            path: path.into(),
            config: None,
        }
    }

    pub fn load_config(&mut self) -> &mut Config {
        let config = match std::fs::read_to_string(&self.path) {
            Ok(content) => Config::from_str(&content),
            Err(_) => {
                let config = Config::default();
                let content = serde_json::to_string_pretty(&config).unwrap();
                std::fs::write(&self.path, content).unwrap();
                config
            }
        };
        self.config = Some(config);
        self.config.as_mut().unwrap()
    }

    pub fn get_or_load_config(&mut self) -> &mut Config {
        if self.config.is_none() {
            self.load_config();
        }
        self.config.as_mut().unwrap()
    }

    pub fn save_config(&self) {
        if let Some(config) = &self.config {
            let content = serde_json::to_string_pretty(config).unwrap();
            std::fs::write(self.path.clone(), content).unwrap();
        }
    }
}
