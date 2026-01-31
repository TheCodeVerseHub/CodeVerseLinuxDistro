//! Desktop icon representation
//!
//! Each icon represents a file or folder on the desktop.

use anyhow::Result;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::{debug, error, warn};

use crate::config::Config;
use crate::ipc::{
    IconMetadata, IconType as IpcIconType, Position, PositionInput, RenderContext, Request,
    Response,
};
use crate::lua::{DrawCommand, LuaProcess};
use crate::sandbox::SandboxOptions;

/// Timeout for IPC requests to Lua process
const IPC_TIMEOUT: Duration = Duration::from_millis(500);

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

    /// Whether icon is hovered
    hovered: bool,

    /// Lua process for custom scripts (sandboxed)
    lua_process: Option<LuaProcess>,

    /// Path to the IPC handler script
    handler_path: Option<PathBuf>,

    /// Path to the icon widget script for this icon
    script_path: Option<PathBuf>,

    /// Cached draw commands for fallback rendering
    cached_draw_commands: Vec<DrawCommand>,

    /// Icon size from config
    size: u32,

    /// Sandbox options for Lua process
    sandbox_options: SandboxOptions,
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
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let icon_type = Self::determine_type(path);

        // Build sandbox options from config
        let mut sandbox_options = SandboxOptions::default();
        sandbox_options.allow_network = config.sandbox.allow_network;
        for p in &config.sandbox.read_only_paths {
            sandbox_options.read_only_paths.push(p.clone());
        }
        for p in &config.sandbox.read_write_paths {
            sandbox_options.read_write_paths.push(p.clone());
        }

        Ok(Self {
            path: path.to_path_buf(),
            name,
            icon_type,
            grid_x: 0,
            grid_y: 0,
            selected: false,
            hovered: false,
            lua_process: None,
            handler_path: None,
            script_path: None,
            cached_draw_commands: Vec::new(),
            size: config.icon_size,
            sandbox_options,
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

    /// Set the hover state
    pub fn set_hovered(&mut self, hovered: bool) {
        self.hovered = hovered;
    }

    /// Check if hovered
    pub fn is_hovered(&self) -> bool {
        self.hovered
    }

    /// Spawn a sandboxed Lua process for this icon
    ///
    /// # Arguments
    /// * `handler_path` - Path to the IPC handler script (ipc_handler.lua)
    /// * `icon_script_path` - Path to the icon widget script (e.g., file.lua, folder.lua)
    ///
    /// # Returns
    /// Ok(()) if the process was spawned successfully, Err otherwise
    pub fn spawn_lua_process(&mut self, handler_path: &Path, icon_script_path: &Path) -> Result<()> {
        // Kill any existing process first
        if let Some(mut process) = self.lua_process.take() {
            if let Err(e) = process.kill() {
                warn!("Failed to kill existing Lua process: {}", e);
            }
        }

        self.handler_path = Some(handler_path.to_path_buf());
        self.script_path = Some(icon_script_path.to_path_buf());

        match LuaProcess::spawn(
            handler_path.to_path_buf(),
            icon_script_path.to_path_buf(),
            &self.sandbox_options,
        ) {
            Ok(process) => {
                debug!(
                    "Spawned Lua process (pid {}) for icon: {} (handler: {}, script: {})",
                    process.pid(),
                    self.name,
                    handler_path.display(),
                    icon_script_path.display()
                );
                self.lua_process = Some(process);
                Ok(())
            }
            Err(e) => {
                error!(
                    "Failed to spawn Lua process for {}: {}",
                    self.path.display(),
                    e
                );
                Err(e)
            }
        }
    }

    /// Kill the Lua process if it exists
    pub fn kill_lua_process(&mut self) {
        if let Some(mut process) = self.lua_process.take() {
            debug!("Killing Lua process for icon: {}", self.name);
            if let Err(e) = process.kill() {
                warn!("Failed to kill Lua process: {}", e);
            }
        }
    }

    /// Check if the Lua process is still running and restart if crashed
    fn ensure_process_running(&mut self) -> bool {
        if let Some(ref mut process) = self.lua_process {
            if process.is_running() {
                return true;
            }
            // Process crashed, log and attempt restart
            error!("Lua process for {} crashed, attempting restart", self.name);
        }

        // Try to restart if we have both handler and script paths
        if let (Some(handler_path), Some(script_path)) =
            (self.handler_path.clone(), self.script_path.clone())
        {
            match self.spawn_lua_process(&handler_path, &script_path) {
                Ok(()) => {
                    debug!("Successfully restarted Lua process for {}", self.name);
                    true
                }
                Err(e) => {
                    error!("Failed to restart Lua process: {}", e);
                    false
                }
            }
        } else {
            false
        }
    }

    /// Convert local IconType to IPC IconType
    fn to_ipc_icon_type(&self) -> IpcIconType {
        match self.icon_type {
            IconType::Folder => IpcIconType::Directory,
            IconType::File => IpcIconType::File,
            IconType::Symlink => IpcIconType::Symlink,
            IconType::Executable => IpcIconType::Application,
            IconType::Image => IpcIconType::Custom("image".to_string()),
            IconType::Document => IpcIconType::Custom("document".to_string()),
            IconType::Archive => IpcIconType::Custom("archive".to_string()),
            IconType::Video => IpcIconType::Custom("video".to_string()),
            IconType::Audio => IpcIconType::Custom("audio".to_string()),
            IconType::Unknown => IpcIconType::File,
        }
    }

    /// Request a render from the Lua process
    ///
    /// Sends a RenderRequest to the Lua process and returns the DrawCommands.
    /// If the process is not running or times out, returns cached commands or fallback.
    ///
    /// # Arguments
    /// * `canvas_width` - Width of the canvas in pixels
    /// * `canvas_height` - Height of the canvas in pixels
    /// * `device_pixel_ratio` - Device pixel ratio for HiDPI support
    ///
    /// # Returns
    /// Vector of DrawCommands for rendering the icon
    pub fn request_render(
        &mut self,
        canvas_width: u32,
        canvas_height: u32,
        device_pixel_ratio: f32,
    ) -> Vec<DrawCommand> {
        // Check if we have a Lua process
        if self.lua_process.is_none() {
            return self.fallback_render();
        }

        // Ensure process is running (restart if crashed)
        if !self.ensure_process_running() {
            warn!(
                "Lua process not running for {}, using fallback",
                self.name
            );
            return self.cached_draw_commands.clone();
        }

        // Build the render request
        let metadata = IconMetadata {
            path: self.path.to_string_lossy().to_string(),
            name: self.name.clone(),
            mime_type: self.get_mime_type(),
            is_directory: self.icon_type == IconType::Folder,
            size: self.get_file_size(),
            width: self.size,
            height: self.size,
            icon_type: self.to_ipc_icon_type(),
            selected: self.selected,
            hovered: self.hovered,
        };

        let context = RenderContext {
            canvas_width,
            canvas_height,
            device_pixel_ratio,
        };

        let request = Request::Render { metadata, context };

        // Send request and receive response
        if let Some(ref mut process) = self.lua_process {
            match process.send_request(&request) {
                Ok(()) => {
                    match process.receive_response_with_timeout(IPC_TIMEOUT) {
                        Ok(Response::Render { commands }) => {
                            // Cache the commands for fallback
                            self.cached_draw_commands = commands.clone();
                            return commands;
                        }
                        Ok(Response::Error { message }) => {
                            error!("Lua render error for {}: {}", self.name, message);
                        }
                        Ok(other) => {
                            warn!("Unexpected response from Lua: {:?}", other);
                        }
                        Err(e) => {
                            warn!("IPC timeout/error for {}: {}", self.name, e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to send render request: {}", e);
                }
            }
        }

        // Return cached or fallback on error
        if !self.cached_draw_commands.is_empty() {
            self.cached_draw_commands.clone()
        } else {
            self.fallback_render()
        }
    }

    /// Request position calculation from the Lua process
    ///
    /// # Arguments
    /// * `screen_width` - Screen width in pixels
    /// * `screen_height` - Screen height in pixels
    /// * `icon_count` - Total number of icons
    /// * `icon_index` - Index of this icon (0-based)
    /// * `cell_width` - Optional grid cell width
    /// * `cell_height` - Optional grid cell height
    ///
    /// # Returns
    /// Position of the icon on screen
    pub fn request_position(
        &mut self,
        screen_width: u32,
        screen_height: u32,
        icon_count: u32,
        icon_index: u32,
        cell_width: Option<u32>,
        cell_height: Option<u32>,
    ) -> Position {
        // If no Lua process, use default positioning
        if self.lua_process.is_none() || !self.ensure_process_running() {
            return self.default_position(screen_width, icon_index, cell_width, cell_height);
        }

        let input = PositionInput {
            screen_width,
            screen_height,
            icon_count,
            icon_index,
            cell_width,
            cell_height,
        };

        let request = Request::Position { input };

        if let Some(ref mut process) = self.lua_process {
            match process.send_request(&request) {
                Ok(()) => {
                    match process.receive_response_with_timeout(IPC_TIMEOUT) {
                        Ok(Response::Position { position }) => {
                            // Update grid coordinates
                            self.grid_x = position.x as u32;
                            self.grid_y = position.y as u32;
                            return position;
                        }
                        Ok(Response::Error { message }) => {
                            warn!("Lua position error for {}: {}", self.name, message);
                        }
                        Ok(other) => {
                            warn!("Unexpected response from Lua: {:?}", other);
                        }
                        Err(e) => {
                            warn!("IPC timeout/error for {}: {}", self.name, e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to send position request: {}", e);
                }
            }
        }

        self.default_position(screen_width, icon_index, cell_width, cell_height)
    }

    /// Calculate default position using grid layout
    fn default_position(
        &self,
        screen_width: u32,
        icon_index: u32,
        cell_width: Option<u32>,
        cell_height: Option<u32>,
    ) -> Position {
        let cell_w = cell_width.unwrap_or(96) as i32;
        let cell_h = cell_height.unwrap_or(96) as i32;
        let margin = 20i32;
        let cols = ((screen_width as i32 - margin * 2) / cell_w).max(1);

        let col = (icon_index as i32) % cols;
        let row = (icon_index as i32) / cols;

        Position {
            x: margin + col * cell_w,
            y: margin + row * cell_h,
        }
    }

    /// Generate fallback render commands when Lua is not available
    fn fallback_render(&self) -> Vec<DrawCommand> {
        // Simple fallback: just a colored rectangle based on icon type
        let color = match self.icon_type {
            IconType::Folder => "#4A90D9",
            IconType::Executable => "#73D216",
            IconType::Image => "#F57900",
            IconType::Document => "#EDD400",
            IconType::Archive => "#75507B",
            IconType::Video => "#C17D11",
            IconType::Audio => "#CC0000",
            _ => "#888888",
        };

        vec![
            DrawCommand::Clear {
                color: "#00000000".to_string(),
            },
            DrawCommand::FillRect {
                x: 4.0,
                y: 4.0,
                w: (self.size - 8) as f32,
                h: (self.size - 8) as f32,
                color: color.to_string(),
            },
        ]
    }

    /// Get MIME type for the file (if known)
    fn get_mime_type(&self) -> Option<String> {
        // Simple extension-based MIME type detection
        if let Some(ext) = self.path.extension().and_then(|e| e.to_str()) {
            let mime = match ext.to_lowercase().as_str() {
                "png" => "image/png",
                "jpg" | "jpeg" => "image/jpeg",
                "gif" => "image/gif",
                "svg" => "image/svg+xml",
                "pdf" => "application/pdf",
                "txt" => "text/plain",
                "md" => "text/markdown",
                "html" => "text/html",
                "json" => "application/json",
                "xml" => "application/xml",
                "zip" => "application/zip",
                "tar" => "application/x-tar",
                "gz" => "application/gzip",
                "mp3" => "audio/mpeg",
                "mp4" => "video/mp4",
                "mkv" => "video/x-matroska",
                _ => return None,
            };
            Some(mime.to_string())
        } else {
            None
        }
    }

    /// Get file size in bytes
    fn get_file_size(&self) -> Option<u64> {
        self.path.metadata().ok().map(|m| m.len())
    }

    /// Check if icon has a Lua process
    pub fn has_lua_process(&self) -> bool {
        self.lua_process.is_some()
    }

    /// Get the script path if set
    pub fn script_path(&self) -> Option<&Path> {
        self.script_path.as_deref()
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

    fn test_config() -> Config {
        Config::default()
    }

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

    #[test]
    fn test_icon_creation_with_new_fields() {
        let config = test_config();
        let icon = DesktopIcon::new(Path::new("/tmp/test.txt"), &config).unwrap();

        assert!(!icon.hovered);
        assert!(icon.lua_process.is_none());
        assert!(icon.script_path.is_none());
        assert!(icon.cached_draw_commands.is_empty());
    }

    #[test]
    fn test_default_position_calculation() {
        let config = test_config();
        let icon = DesktopIcon::new(Path::new("/tmp/test.txt"), &config).unwrap();

        // With 1920 width, margin 20, cell 96: cols = (1920-40)/96 = 19
        let pos = icon.default_position(1920, 0, Some(96), Some(96));
        assert_eq!(pos.x, 20); // margin
        assert_eq!(pos.y, 20); // margin

        // icon_index 5: col = 5 % 19 = 5, row = 5 / 19 = 0
        let pos = icon.default_position(1920, 5, Some(96), Some(96));
        assert_eq!(pos.x, 20 + 5 * 96); // margin + 5 * cell
        assert_eq!(pos.y, 20); // first row

        // icon_index 20: col = 20 % 19 = 1, row = 20 / 19 = 1
        let pos = icon.default_position(1920, 20, Some(96), Some(96));
        assert_eq!(pos.x, 20 + 1 * 96);
        assert_eq!(pos.y, 20 + 1 * 96);
    }

    #[test]
    fn test_fallback_render_returns_commands() {
        let config = test_config();
        let icon = DesktopIcon::new(Path::new("/tmp/test.txt"), &config).unwrap();

        let commands = icon.fallback_render();
        assert_eq!(commands.len(), 2);

        // First command should be Clear
        match &commands[0] {
            DrawCommand::Clear { color } => {
                assert_eq!(color, "#00000000");
            }
            _ => panic!("Expected Clear command"),
        }

        // Second command should be FillRect
        match &commands[1] {
            DrawCommand::FillRect { x, y, w, h, color: _ } => {
                assert_eq!(*x, 4.0);
                assert_eq!(*y, 4.0);
                assert_eq!(*w, 56.0); // size(64) - 8
                assert_eq!(*h, 56.0);
            }
            _ => panic!("Expected FillRect command"),
        }
    }

    #[test]
    fn test_fallback_render_colors_by_type() {
        let config = test_config();

        // Folder should be blue
        let folder = DesktopIcon::new(Path::new("/tmp"), &config).unwrap();
        let commands = folder.fallback_render();
        if let DrawCommand::FillRect { color, .. } = &commands[1] {
            assert_eq!(color, "#4A90D9");
        }
    }

    #[test]
    fn test_icon_type_to_ipc_conversion() {
        let config = test_config();

        let folder = DesktopIcon::new(Path::new("/tmp"), &config).unwrap();
        assert_eq!(folder.to_ipc_icon_type(), IpcIconType::Directory);

        // Note: .txt is detected as Document type, not File
        let document = DesktopIcon::new(Path::new("/tmp/test.txt"), &config).unwrap();
        assert_eq!(document.to_ipc_icon_type(), IpcIconType::Custom("document".to_string()));

        // Use extension without type mapping for plain file
        let file = DesktopIcon::new(Path::new("/tmp/test.unknown_ext"), &config).unwrap();
        assert_eq!(file.to_ipc_icon_type(), IpcIconType::File);

        let image = DesktopIcon::new(Path::new("/tmp/test.png"), &config).unwrap();
        assert_eq!(image.to_ipc_icon_type(), IpcIconType::Custom("image".to_string()));
    }

    #[test]
    fn test_mime_type_detection() {
        let config = test_config();

        let png = DesktopIcon::new(Path::new("/tmp/test.png"), &config).unwrap();
        assert_eq!(png.get_mime_type(), Some("image/png".to_string()));

        let jpg = DesktopIcon::new(Path::new("/tmp/test.jpg"), &config).unwrap();
        assert_eq!(jpg.get_mime_type(), Some("image/jpeg".to_string()));

        let pdf = DesktopIcon::new(Path::new("/tmp/test.pdf"), &config).unwrap();
        assert_eq!(pdf.get_mime_type(), Some("application/pdf".to_string()));

        let unknown = DesktopIcon::new(Path::new("/tmp/test.xyz"), &config).unwrap();
        assert_eq!(unknown.get_mime_type(), None);
    }

    #[test]
    fn test_set_hovered() {
        let config = test_config();
        let mut icon = DesktopIcon::new(Path::new("/tmp/test.txt"), &config).unwrap();

        assert!(!icon.is_hovered());
        icon.set_hovered(true);
        assert!(icon.is_hovered());
        icon.set_hovered(false);
        assert!(!icon.is_hovered());
    }

    #[test]
    fn test_has_lua_process() {
        let config = test_config();
        let icon = DesktopIcon::new(Path::new("/tmp/test.txt"), &config).unwrap();

        assert!(!icon.has_lua_process());
    }

    #[test]
    fn test_request_render_without_process_returns_fallback() {
        let config = test_config();
        let mut icon = DesktopIcon::new(Path::new("/tmp/test.txt"), &config).unwrap();

        let commands = icon.request_render(128, 128, 1.0);
        assert_eq!(commands.len(), 2); // fallback render returns 2 commands
    }

    #[test]
    fn test_request_position_without_process_returns_default() {
        let config = test_config();
        let mut icon = DesktopIcon::new(Path::new("/tmp/test.txt"), &config).unwrap();

        let pos = icon.request_position(1920, 1080, 25, 5, Some(96), Some(96));
        assert_eq!(pos.x, 20 + 5 * 96);
        assert_eq!(pos.y, 20);
    }
}
