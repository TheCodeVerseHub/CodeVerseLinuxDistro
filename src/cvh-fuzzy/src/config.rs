//! Configuration module for cvh-fuzzy

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Default search mode
    #[serde(default = "default_mode")]
    pub default_mode: String,

    /// Window height in lines
    #[serde(default = "default_height")]
    pub height: u16,

    /// Show border
    #[serde(default = "default_border")]
    pub border: bool,

    /// Custom application directories
    #[serde(default)]
    pub app_dirs: Vec<PathBuf>,

    /// Files/directories to ignore
    #[serde(default)]
    pub ignore_patterns: Vec<String>,

    /// Colors
    #[serde(default)]
    pub colors: Colors,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Colors {
    #[serde(default = "default_fg")]
    pub fg: String,

    #[serde(default = "default_bg")]
    pub bg: String,

    #[serde(default = "default_highlight")]
    pub highlight: String,

    #[serde(default = "default_border_color")]
    pub border: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_mode: default_mode(),
            height: default_height(),
            border: default_border(),
            app_dirs: Vec::new(),
            ignore_patterns: vec![
                ".git".to_string(),
                "node_modules".to_string(),
                "target".to_string(),
                "__pycache__".to_string(),
            ],
            colors: Colors::default(),
        }
    }
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            fg: default_fg(),
            bg: default_bg(),
            highlight: default_highlight(),
            border: default_border_color(),
        }
    }
}

fn default_mode() -> String {
    "apps".to_string()
}

fn default_height() -> u16 {
    40
}

fn default_border() -> bool {
    true
}

fn default_fg() -> String {
    "#eceff4".to_string()
}

fn default_bg() -> String {
    "#2e3440".to_string()
}

fn default_highlight() -> String {
    "#88c0d0".to_string()
}

fn default_border_color() -> String {
    "#4c566a".to_string()
}

impl Config {
    /// Load configuration from file
    pub fn load() -> Self {
        let config_path = dirs::config_dir()
            .map(|d| d.join("cvh-fuzzy/config.toml"));

        if let Some(path) = config_path {
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(config) = toml::from_str(&content) {
                        return config;
                    }
                }
            }
        }

        Self::default()
    }
}
