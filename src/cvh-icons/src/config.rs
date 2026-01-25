//! Configuration for cvh-icons

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Main configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Icon size in pixels
    #[serde(default = "default_icon_size")]
    pub icon_size: u32,

    /// Grid spacing
    #[serde(default = "default_grid_spacing")]
    pub grid_spacing: u32,

    /// Icon label font size
    #[serde(default = "default_font_size")]
    pub font_size: f32,

    /// Icon label max width (chars)
    #[serde(default = "default_label_width")]
    pub label_width: usize,

    /// Directories to search for Lua scripts
    #[serde(default = "default_script_dirs")]
    pub script_dirs: Vec<PathBuf>,

    /// Default icon theme
    #[serde(default = "default_icon_theme")]
    pub icon_theme: String,

    /// Sandbox configuration
    #[serde(default)]
    pub sandbox: SandboxConfig,

    /// Colors
    #[serde(default)]
    pub colors: Colors,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Enable sandboxing
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Allow network access
    #[serde(default)]
    pub allow_network: bool,

    /// Read-only paths (in addition to defaults)
    #[serde(default)]
    pub read_only_paths: Vec<PathBuf>,

    /// Read-write paths (in addition to defaults)
    #[serde(default)]
    pub read_write_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Colors {
    #[serde(default = "default_label_fg")]
    pub label_fg: String,

    #[serde(default = "default_label_bg")]
    pub label_bg: String,

    #[serde(default = "default_label_shadow")]
    pub label_shadow: String,

    #[serde(default = "default_selection")]
    pub selection: String,
}

// Default functions
fn default_icon_size() -> u32 { 64 }
fn default_grid_spacing() -> u32 { 20 }
fn default_font_size() -> f32 { 12.0 }
fn default_label_width() -> usize { 12 }
fn default_true() -> bool { true }

fn default_script_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from("/usr/share/cvh-icons/scripts"),
        PathBuf::from("/usr/lib/cvh-icons/lua"),
    ];

    if let Some(config_dir) = dirs::config_dir() {
        dirs.push(config_dir.join("cvh-icons/scripts"));
    }

    dirs
}

fn default_icon_theme() -> String {
    "Adwaita".to_string()
}

fn default_label_fg() -> String { "#ffffff".to_string() }
fn default_label_bg() -> String { "#00000080".to_string() }
fn default_label_shadow() -> String { "#000000".to_string() }
fn default_selection() -> String { "#88c0d040".to_string() }

impl Default for Config {
    fn default() -> Self {
        Self {
            icon_size: default_icon_size(),
            grid_spacing: default_grid_spacing(),
            font_size: default_font_size(),
            label_width: default_label_width(),
            script_dirs: default_script_dirs(),
            icon_theme: default_icon_theme(),
            sandbox: SandboxConfig::default(),
            colors: Colors::default(),
        }
    }
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            allow_network: false,
            read_only_paths: Vec::new(),
            read_write_paths: Vec::new(),
        }
    }
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            label_fg: default_label_fg(),
            label_bg: default_label_bg(),
            label_shadow: default_label_shadow(),
            selection: default_selection(),
        }
    }
}

impl Config {
    /// Load configuration from file or defaults
    pub fn load(path: Option<&Path>) -> Result<Self> {
        // Try explicit path first
        if let Some(p) = path {
            if p.exists() {
                let content = std::fs::read_to_string(p)?;
                return Ok(toml::from_str(&content)?);
            }
        }

        // Try XDG config
        if let Some(config_dir) = dirs::config_dir() {
            let config_file = config_dir.join("cvh-icons/config.toml");
            if config_file.exists() {
                let content = std::fs::read_to_string(&config_file)?;
                return Ok(toml::from_str(&content)?);
            }
        }

        // Use defaults
        Ok(Self::default())
    }
}
