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
        // Try multiple locations:
        // 1. ~/.config/mortty/config.toml (XDG/Handy for terminal users)
        // 2. ~/Library/Application Support/mortty/config.toml (macOS Standard)
        
        let mut paths = Vec::new();
        
        // Potential XDG path
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".config").join("mortty").join("config.toml"));
        }
        
        // Standard App Support path
        if let Some(proj_dirs) = ProjectDirs::from("", "", "mortty") {
            paths.push(proj_dirs.config_dir().join("config.toml"));
        }

        for path in paths {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(config) = toml::from_str::<Config>(&content) {
                        println!("Loaded config from {:?}", path);
                        return config;
                    }
                }
            }
        }
        
        Config::default()
    }
}
