//! Sandbox module for secure Lua script execution
//!
//! Provides multi-layer sandboxing:
//! 1. Bubblewrap container isolation
//! 2. Restricted Lua environment

use anyhow::Result;
use std::path::PathBuf;
use std::process::Command;

mod bubblewrap;

/// Sandbox configuration for icon scripts
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SandboxOptions {
    /// Allow network access
    pub allow_network: bool,

    /// Read-only filesystem paths
    pub read_only_paths: Vec<PathBuf>,

    /// Read-write filesystem paths
    pub read_write_paths: Vec<PathBuf>,

    /// Environment variables to pass
    pub env_vars: Vec<(String, String)>,

    /// Working directory
    pub work_dir: Option<PathBuf>,
}

impl Default for SandboxOptions {
    fn default() -> Self {
        Self {
            allow_network: false,
            read_only_paths: vec![
                PathBuf::from("/usr"),
                PathBuf::from("/lib"),
                PathBuf::from("/lib64"),
            ],
            read_write_paths: Vec::new(),
            env_vars: Vec::new(),
            work_dir: None,
        }
    }
}

/// Check if bubblewrap is available
pub fn _is_bubblewrap_available() -> bool {
    Command::new("bwrap")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Validate sandbox configuration
pub fn _validate_config(options: &SandboxOptions) -> Result<()> {
    // Check that all specified paths exist
    for path in &options.read_only_paths {
        if !path.exists() {
            tracing::warn!("Sandbox read-only path does not exist: {}", path.display());
        }
    }

    for path in &options.read_write_paths {
        if !path.exists() {
            tracing::warn!("Sandbox read-write path does not exist: {}", path.display());
        }
    }

    Ok(())
}
