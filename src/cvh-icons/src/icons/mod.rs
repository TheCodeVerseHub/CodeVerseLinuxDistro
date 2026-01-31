//! Desktop icon representation
//!
//! Each icon represents a file or folder on the desktop.

use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::lua::LuaRuntime;

/// Represents a desktop icon
#[allow(dead_code)]
pub struct DesktopIcon {
    /// Path to the file/folder
    path: PathBuf,

    /// Display name
    name: String,

    /// Icon type
    icon_type: IconType,

    /// Position on desktop (grid coordinates)
    grid_x: u32,
    grid_y: u32,

    /// Whether icon is selected
    selected: bool,

    /// Lua runtime for custom scripts
    lua: Option<LuaRuntime>,

    /// Icon size from config
    size: u32,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconType {
    File,
    Folder,
    Symlink,
    Executable,
    Image,
    Document,
    Archive,
    Video,
    Audio,
    Unknown,
}

#[allow(dead_code)]
impl DesktopIcon {
    /// Create a new desktop icon
    pub fn new(path: &Path, config: &Config) -> Result<Self> {
        let name = path.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let icon_type = Self::determine_type(path);

        Ok(Self {
            path: path.to_path_buf(),
            name,
            icon_type,
            grid_x: 0,
            grid_y: 0,
            selected: false,
            lua: None,
            size: config.icon_size,
        })
    }

    /// Determine the icon type based on the file
    fn determine_type(path: &Path) -> IconType {
        if path.is_symlink() {
            return IconType::Symlink;
        }

        if path.is_dir() {
            return IconType::Folder;
        }

        // Check extension
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            match ext.to_lowercase().as_str() {
                // Executables
                "sh" | "bash" | "zsh" | "fish" | "py" | "rb" | "pl" => IconType::Executable,

                // Images
                "png" | "jpg" | "jpeg" | "gif" | "bmp" | "svg" | "webp" | "ico" => IconType::Image,

                // Documents
                "pdf" | "doc" | "docx" | "odt" | "txt" | "md" | "rst" => IconType::Document,

                // Archives
                "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" | "zst" => IconType::Archive,

                // Video
                "mp4" | "mkv" | "avi" | "mov" | "webm" | "flv" => IconType::Video,

                // Audio
                "mp3" | "flac" | "wav" | "ogg" | "m4a" | "opus" => IconType::Audio,

                _ => IconType::File,
            }
        } else {
            // Check if executable
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(metadata) = path.metadata() {
                    if metadata.permissions().mode() & 0o111 != 0 {
                        return IconType::Executable;
                    }
                }
            }

            IconType::File
        }
    }

    /// Get the icon name for freedesktop icon themes
    pub fn icon_name(&self) -> &'static str {
        match self.icon_type {
            IconType::Folder => "folder",
            IconType::File => "text-x-generic",
            IconType::Symlink => "emblem-symbolic-link",
            IconType::Executable => "application-x-executable",
            IconType::Image => "image-x-generic",
            IconType::Document => "x-office-document",
            IconType::Archive => "package-x-generic",
            IconType::Video => "video-x-generic",
            IconType::Audio => "audio-x-generic",
            IconType::Unknown => "unknown",
        }
    }

    /// Get the display name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the path
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get the icon type
    pub fn icon_type(&self) -> IconType {
        self.icon_type
    }

    /// Set grid position
    pub fn set_position(&mut self, x: u32, y: u32) {
        self.grid_x = x;
        self.grid_y = y;
    }

    /// Get grid position
    pub fn position(&self) -> (u32, u32) {
        (self.grid_x, self.grid_y)
    }

    /// Set selection state
    pub fn set_selected(&mut self, selected: bool) {
        self.selected = selected;
    }

    /// Check if selected
    pub fn is_selected(&self) -> bool {
        self.selected
    }

    /// Update the icon (called each frame)
    pub fn update(&mut self) -> Result<()> {
        // Check if file still exists
        if !self.path.exists() {
            return Err(anyhow::anyhow!("File no longer exists"));
        }

        Ok(())
    }

    /// Handle click event
    pub fn on_click(&mut self, button: u32) -> Result<ClickAction> {
        match button {
            1 => {
                // Left click - select
                self.selected = !self.selected;
                Ok(ClickAction::Select)
            }
            2 => {
                // Middle click - open in terminal
                Ok(ClickAction::OpenInTerminal)
            }
            3 => {
                // Right click - context menu
                Ok(ClickAction::ContextMenu)
            }
            _ => Ok(ClickAction::None),
        }
    }

    /// Handle double-click
    pub fn on_double_click(&self) -> Result<ClickAction> {
        Ok(ClickAction::Open)
    }

    /// Load a custom Lua script for this icon
    pub fn load_script(&mut self, script_path: &Path) -> Result<()> {
        let lua = LuaRuntime::new()?;
        lua.load_script(script_path)?;
        self.lua = Some(lua);
        Ok(())
    }
}

/// Action to take after a click
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickAction {
    None,
    Select,
    Open,
    OpenInTerminal,
    ContextMenu,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icon_type_detection() {
        assert_eq!(
            DesktopIcon::determine_type(Path::new("/tmp/test.png")),
            IconType::Image
        );
        assert_eq!(
            DesktopIcon::determine_type(Path::new("/tmp/test.mp3")),
            IconType::Audio
        );
        assert_eq!(
            DesktopIcon::determine_type(Path::new("/tmp/test.zip")),
            IconType::Archive
        );
    }
}
