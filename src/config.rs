use serde::Deserialize;
use std::fs;
use directories::ProjectDirs;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub font_family: String,
    pub font_size: f32,
    pub padding: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            font_family: "FiraCode Nerd Font".to_string(),
            font_size: 22.0,
            padding: 10.0,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let config_path = ProjectDirs::from("", "", "mortty")
            .map(|dirs| dirs.config_dir().join("config.toml"));

        if let Some(path) = config_path {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(config) = toml::from_str::<Config>(&content) {
                        log::info!("Loaded config from {:?}", path);
                        return config;
                    } else {
                        log::warn!("Failed to parse config at {:?}, using defaults", path);
                    }
                }
            } else {
                // Optionally create the default config file if it doesn't exist?
                // For now, just return defaults.
            }
        }
        Config::default()
    }
}
