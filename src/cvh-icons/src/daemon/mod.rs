//! Icon daemon - manages desktop icons
//!
//! Watches the desktop directory and manages icon windows.
//! Uses calloop event loop for Wayland integration compatibility.

use anyhow::{Context, Result};
use calloop::channel::{Channel, Sender};
use calloop::timer::{TimeoutAction, Timer};
use calloop::EventLoop;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::{debug, error, info, warn};

use crate::config::Config;
use crate::icons::{DesktopIcon, IconType};

/// Icon daemon that manages desktop icons
pub struct IconDaemon {
    config: Config,
    desktop_dir: PathBuf,
    icons: HashMap<PathBuf, DesktopIcon>,
    watcher: Option<RecommendedWatcher>,
    event_sender: Option<Sender<notify::Result<Event>>>,
}

impl IconDaemon {
    /// Create a new icon daemon
    pub fn new(config: Config, desktop_dir: PathBuf) -> Result<Self> {
        info!("Initializing icon daemon for {}", desktop_dir.display());

        let mut daemon = Self {
            config,
            desktop_dir,
            icons: HashMap::new(),
            watcher: None,
            event_sender: None,
        };

        // Initial scan of desktop directory
        daemon.scan_desktop()?;

        Ok(daemon)
    }

    /// Set up file system watcher with calloop channel
    fn setup_watcher(&mut self, sender: Sender<notify::Result<Event>>) -> Result<()> {
        let tx = sender.clone();

        let watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            notify::Config::default(),
        )?;

        self.watcher = Some(watcher);
        self.event_sender = Some(sender);

        // Start watching
        if let Some(ref mut watcher) = self.watcher {
            watcher.watch(&self.desktop_dir, RecursiveMode::NonRecursive)?;
            info!("Watching desktop directory: {}", self.desktop_dir.display());
        }

        Ok(())
    }

    /// Scan the desktop directory for files/folders
    fn scan_desktop(&mut self) -> Result<()> {
        if !self.desktop_dir.exists() {
            warn!("Desktop directory does not exist: {}", self.desktop_dir.display());
            return Ok(());
        }

        let entries = std::fs::read_dir(&self.desktop_dir)
            .context("Failed to read desktop directory")?;

        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();

            // Skip hidden files
            if path.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with('.'))
                .unwrap_or(false)
            {
                continue;
            }

            self.add_icon(&path)?;
        }

        info!("Loaded {} desktop icons", self.icons.len());
        Ok(())
    }

    /// Add an icon for a file/folder
    fn add_icon(&mut self, path: &Path) -> Result<()> {
        if self.icons.contains_key(path) {
            return Ok(());
        }

        let mut icon = DesktopIcon::new(path, &self.config)?;

        // Try to spawn a Lua process for this icon
        if let Some((handler_path, widget_script_path)) = self.find_script_for_icon(&icon) {
            match icon.spawn_lua_process(&handler_path, &widget_script_path) {
                Ok(()) => {
                    debug!(
                        "Spawned Lua process for icon: {} (handler: {}, script: {})",
                        path.display(),
                        handler_path.display(),
                        widget_script_path.display()
                    );
                }
                Err(e) => {
                    warn!(
                        "Failed to spawn Lua process for {}: {} (using fallback)",
                        path.display(),
                        e
                    );
                    // Icon will use fallback rendering
                }
            }
        }

        debug!("Added icon for: {}", path.display());
        self.icons.insert(path.to_path_buf(), icon);

        Ok(())
    }

    /// Find the IPC handler and appropriate widget script for an icon based on its type
    ///
    /// Returns a tuple of (handler_path, widget_script_path) if both are found
    fn find_script_for_icon(&self, icon: &DesktopIcon) -> Option<(PathBuf, PathBuf)> {
        let script_name = match icon.icon_type() {
            IconType::Folder => "folder.lua",
            IconType::File => "file.lua",
            IconType::Symlink => "symlink.lua",
            IconType::Executable => "executable.lua",
            IconType::Image => "image.lua",
            IconType::Document => "document.lua",
            IconType::Archive => "archive.lua",
            IconType::Video => "video.lua",
            IconType::Audio => "audio.lua",
            IconType::Unknown => "file.lua",
        };

        // First, find the IPC handler script
        let mut handler_path = None;
        for dir in &self.config.script_dirs {
            let path = dir.join("ipc_handler.lua");
            if path.exists() {
                handler_path = Some(path);
                break;
            }
        }

        // If no handler found, we can't spawn a Lua process
        let handler_path = handler_path?;

        // Search through script directories for the widget script
        for dir in &self.config.script_dirs {
            let script_path = dir.join(script_name);
            if script_path.exists() {
                return Some((handler_path.clone(), script_path));
            }

            // Also check in widgets subdirectory
            let widgets_path = dir.join("widgets").join(script_name);
            if widgets_path.exists() {
                return Some((handler_path.clone(), widgets_path));
            }
        }

        // No matching widget script found
        None
    }

    /// Remove an icon
    fn remove_icon(&mut self, path: &Path) {
        if let Some(mut icon) = self.icons.remove(path) {
            // Kill the Lua process before removing the icon
            icon.kill_lua_process();
            debug!("Removed icon for: {}", path.display());
        }
    }

    /// Handle a file system event
    fn handle_fs_event(&mut self, event: Event) -> Result<()> {
        use notify::EventKind;

        match event.kind {
            EventKind::Create(_) => {
                for path in event.paths {
                    self.add_icon(&path)?;
                }
            }
            EventKind::Remove(_) => {
                for path in event.paths {
                    self.remove_icon(&path);
                }
            }
            EventKind::Modify(_) => {
                // Refresh icons if metadata changed
                for path in event.paths {
                    if self.icons.contains_key(&path) {
                        self.remove_icon(&path);
                        self.add_icon(&path)?;
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Update all icons
    fn update_icons(&mut self) {
        // Collect paths of icons to remove (file no longer exists)
        let mut to_remove = Vec::new();

        for (path, icon) in self.icons.iter_mut() {
            if let Err(e) = icon.update() {
                warn!("Error updating icon: {}", e);
                to_remove.push(path.clone());
            }
        }

        // Remove icons for deleted files
        for path in to_remove {
            self.remove_icon(&path);
        }
    }

    /// Request render for all icons (called when display needs update)
    ///
    /// Returns a vector of (path, draw_commands) pairs
    #[allow(dead_code)]
    pub fn render_all_icons(
        &mut self,
        canvas_width: u32,
        canvas_height: u32,
        device_pixel_ratio: f32,
    ) -> Vec<(PathBuf, Vec<crate::lua::DrawCommand>)> {
        self.icons
            .iter_mut()
            .map(|(path, icon)| {
                let commands = icon.request_render(canvas_width, canvas_height, device_pixel_ratio);
                (path.clone(), commands)
            })
            .collect()
    }

    /// Calculate positions for all icons
    ///
    /// Returns a vector of (path, position) pairs
    #[allow(dead_code)]
    pub fn position_all_icons(
        &mut self,
        screen_width: u32,
        screen_height: u32,
        cell_width: Option<u32>,
        cell_height: Option<u32>,
    ) -> Vec<(PathBuf, crate::ipc::Position)> {
        let icon_count = self.icons.len() as u32;

        self.icons
            .iter_mut()
            .enumerate()
            .map(|(index, (path, icon))| {
                let position = icon.request_position(
                    screen_width,
                    screen_height,
                    icon_count,
                    index as u32,
                    cell_width,
                    cell_height,
                );
                (path.clone(), position)
            })
            .collect()
    }

    /// Get an icon by path
    #[allow(dead_code)]
    pub fn get_icon(&self, path: &Path) -> Option<&DesktopIcon> {
        self.icons.get(path)
    }

    /// Get a mutable icon by path
    #[allow(dead_code)]
    pub fn get_icon_mut(&mut self, path: &Path) -> Option<&mut DesktopIcon> {
        self.icons.get_mut(path)
    }

    /// Run the main daemon loop using calloop
    pub fn run(&mut self) -> Result<()> {
        info!("Icon daemon running with calloop event loop");

        // Create the calloop event loop
        let mut event_loop: EventLoop<DaemonState> = EventLoop::try_new()
            .context("Failed to create calloop event loop")?;
        let loop_handle = event_loop.handle();

        // Create a channel for file watcher events
        let (sender, channel): (Sender<notify::Result<Event>>, Channel<notify::Result<Event>>) =
            calloop::channel::channel();

        // Set up the file watcher with the calloop channel sender
        self.setup_watcher(sender)?;

        // Register the file watcher channel as an event source
        loop_handle
            .insert_source(channel, |event, _, state: &mut DaemonState| {
                match event {
                    calloop::channel::Event::Msg(Ok(fs_event)) => {
                        state.pending_events.push(fs_event);
                    }
                    calloop::channel::Event::Msg(Err(e)) => {
                        error!("Watcher error: {}", e);
                    }
                    calloop::channel::Event::Closed => {
                        error!("Watcher channel closed");
                        state.should_stop = true;
                    }
                }
            })
            .map_err(|e| anyhow::anyhow!("Failed to register file watcher channel: {:?}", e))?;

        // Register a timer for periodic icon updates (16ms = ~60 FPS)
        let timer = Timer::from_duration(Duration::from_millis(16));
        loop_handle
            .insert_source(timer, |_, _, state: &mut DaemonState| {
                state.should_update_icons = true;
                TimeoutAction::ToDuration(Duration::from_millis(16))
            })
            .map_err(|e| anyhow::anyhow!("Failed to register update timer: {:?}", e))?;

        // Create the daemon state for the event loop
        let mut state = DaemonState {
            pending_events: Vec::new(),
            should_update_icons: false,
            should_stop: false,
        };

        info!("Entering calloop dispatch loop");

        // Main event loop
        loop {
            // Dispatch events (blocking with timeout)
            event_loop
                .dispatch(Some(Duration::from_millis(16)), &mut state)
                .context("Event loop dispatch failed")?;

            // Process pending file system events
            for event in state.pending_events.drain(..) {
                if let Err(e) = self.handle_fs_event(event) {
                    error!("Error handling fs event: {}", e);
                }
            }

            // Update icons if timer fired
            if state.should_update_icons {
                self.update_icons();
                state.should_update_icons = false;
            }

            // Check if we should stop
            if state.should_stop {
                info!("Daemon stopping");
                break;
            }
        }

        Ok(())
    }

    /// Get the number of active icons
    #[allow(dead_code)]
    pub fn icon_count(&self) -> usize {
        self.icons.len()
    }

    /// Check if a path has an icon (for testing)
    #[cfg(test)]
    pub fn has_icon(&self, path: &Path) -> bool {
        self.icons.contains_key(path)
    }

    /// Get reference to icons HashMap (for testing)
    #[cfg(test)]
    pub fn icons(&self) -> &HashMap<PathBuf, DesktopIcon> {
        &self.icons
    }
}

/// State passed to the calloop event loop callbacks
struct DaemonState {
    pending_events: Vec<Event>,
    should_update_icons: bool,
    should_stop: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::{event::CreateKind, event::RemoveKind, event::ModifyKind, EventKind};
    use std::fs;
    use tempfile::TempDir;

    /// Helper to create a test config
    fn test_config() -> Config {
        Config::default()
    }

    /// Helper to create a test daemon without watchers (for unit testing)
    fn create_test_daemon(desktop_dir: PathBuf) -> IconDaemon {
        IconDaemon {
            config: test_config(),
            desktop_dir,
            icons: HashMap::new(),
            watcher: None,
            event_sender: None,
        }
    }

    // ========================================================================
    // File Create Event Tests
    // ========================================================================

    #[test]
    fn test_file_create_event_adds_icon_to_hashmap() {
        let temp_dir = TempDir::new().unwrap();
        let desktop_path = temp_dir.path().to_path_buf();
        let mut daemon = create_test_daemon(desktop_path.clone());

        // Create a test file
        let test_file = desktop_path.join("test_file.txt");
        fs::write(&test_file, "test content").unwrap();

        // Simulate a Create event
        let event = Event {
            kind: EventKind::Create(CreateKind::File),
            paths: vec![test_file.clone()],
            attrs: Default::default(),
        };

        daemon.handle_fs_event(event).unwrap();

        assert!(daemon.has_icon(&test_file), "Icon should be added to HashMap after Create event");
        assert_eq!(daemon.icon_count(), 1, "Should have exactly 1 icon");
    }

    #[test]
    fn test_folder_create_event_adds_icon_to_hashmap() {
        let temp_dir = TempDir::new().unwrap();
        let desktop_path = temp_dir.path().to_path_buf();
        let mut daemon = create_test_daemon(desktop_path.clone());

        // Create a test folder
        let test_folder = desktop_path.join("test_folder");
        fs::create_dir(&test_folder).unwrap();

        // Simulate a Create event
        let event = Event {
            kind: EventKind::Create(CreateKind::Folder),
            paths: vec![test_folder.clone()],
            attrs: Default::default(),
        };

        daemon.handle_fs_event(event).unwrap();

        assert!(daemon.has_icon(&test_folder), "Icon should be added for folder Create event");
    }

    #[test]
    fn test_create_event_multiple_paths() {
        let temp_dir = TempDir::new().unwrap();
        let desktop_path = temp_dir.path().to_path_buf();
        let mut daemon = create_test_daemon(desktop_path.clone());

        // Create multiple test files
        let test_file1 = desktop_path.join("file1.txt");
        let test_file2 = desktop_path.join("file2.txt");
        fs::write(&test_file1, "content1").unwrap();
        fs::write(&test_file2, "content2").unwrap();

        // Simulate a Create event with multiple paths
        let event = Event {
            kind: EventKind::Create(CreateKind::File),
            paths: vec![test_file1.clone(), test_file2.clone()],
            attrs: Default::default(),
        };

        daemon.handle_fs_event(event).unwrap();

        assert!(daemon.has_icon(&test_file1), "Icon should be added for first file");
        assert!(daemon.has_icon(&test_file2), "Icon should be added for second file");
        assert_eq!(daemon.icon_count(), 2, "Should have exactly 2 icons");
    }

    // ========================================================================
    // File Delete Event Tests
    // ========================================================================

    #[test]
    fn test_file_delete_event_removes_icon_from_hashmap() {
        let temp_dir = TempDir::new().unwrap();
        let desktop_path = temp_dir.path().to_path_buf();
        let mut daemon = create_test_daemon(desktop_path.clone());

        // Create a test file and add it to icons
        let test_file = desktop_path.join("test_file.txt");
        fs::write(&test_file, "test content").unwrap();

        // First add the icon via Create event
        let create_event = Event {
            kind: EventKind::Create(CreateKind::File),
            paths: vec![test_file.clone()],
            attrs: Default::default(),
        };
        daemon.handle_fs_event(create_event).unwrap();
        assert!(daemon.has_icon(&test_file), "Icon should exist after Create");

        // Now simulate a Remove event
        let remove_event = Event {
            kind: EventKind::Remove(RemoveKind::File),
            paths: vec![test_file.clone()],
            attrs: Default::default(),
        };

        daemon.handle_fs_event(remove_event).unwrap();

        assert!(!daemon.has_icon(&test_file), "Icon should be removed from HashMap after Remove event");
        assert_eq!(daemon.icon_count(), 0, "Should have no icons after removal");
    }

    #[test]
    fn test_delete_event_for_nonexistent_icon_is_harmless() {
        let temp_dir = TempDir::new().unwrap();
        let desktop_path = temp_dir.path().to_path_buf();
        let mut daemon = create_test_daemon(desktop_path.clone());

        // Try to remove a file that was never added
        let nonexistent = desktop_path.join("never_existed.txt");

        let remove_event = Event {
            kind: EventKind::Remove(RemoveKind::File),
            paths: vec![nonexistent],
            attrs: Default::default(),
        };

        // Should not panic or error
        let result = daemon.handle_fs_event(remove_event);
        assert!(result.is_ok(), "Removing nonexistent icon should not error");
    }

    // ========================================================================
    // File Modify Event Tests
    // ========================================================================

    #[test]
    fn test_file_modify_event_refreshes_icon() {
        let temp_dir = TempDir::new().unwrap();
        let desktop_path = temp_dir.path().to_path_buf();
        let mut daemon = create_test_daemon(desktop_path.clone());

        // Create a test file and add it to icons
        let test_file = desktop_path.join("test_file.txt");
        fs::write(&test_file, "initial content").unwrap();

        // Add the icon via Create event
        let create_event = Event {
            kind: EventKind::Create(CreateKind::File),
            paths: vec![test_file.clone()],
            attrs: Default::default(),
        };
        daemon.handle_fs_event(create_event).unwrap();
        assert_eq!(daemon.icon_count(), 1, "Should have 1 icon before modify");

        // Modify the file
        fs::write(&test_file, "modified content").unwrap();

        // Simulate a Modify event
        let modify_event = Event {
            kind: EventKind::Modify(ModifyKind::Data(notify::event::DataChange::Content)),
            paths: vec![test_file.clone()],
            attrs: Default::default(),
        };

        daemon.handle_fs_event(modify_event).unwrap();

        // Icon should still exist (was refreshed)
        assert!(daemon.has_icon(&test_file), "Icon should still exist after Modify event");
        assert_eq!(daemon.icon_count(), 1, "Should still have exactly 1 icon");
    }

    #[test]
    fn test_modify_event_on_unknown_path_is_ignored() {
        let temp_dir = TempDir::new().unwrap();
        let desktop_path = temp_dir.path().to_path_buf();
        let mut daemon = create_test_daemon(desktop_path.clone());

        // Try to modify a file that was never added
        let unknown_file = desktop_path.join("unknown.txt");
        fs::write(&unknown_file, "content").unwrap();

        let modify_event = Event {
            kind: EventKind::Modify(ModifyKind::Data(notify::event::DataChange::Content)),
            paths: vec![unknown_file.clone()],
            attrs: Default::default(),
        };

        // Should not add the icon (only modifies existing ones)
        daemon.handle_fs_event(modify_event).unwrap();

        assert!(!daemon.has_icon(&unknown_file), "Modify event should not add new icons");
        assert_eq!(daemon.icon_count(), 0, "Should have no icons");
    }

    // ========================================================================
    // Hidden File Filtering Tests
    // ========================================================================

    #[test]
    fn test_scan_desktop_ignores_hidden_files() {
        let temp_dir = TempDir::new().unwrap();
        let desktop_path = temp_dir.path().to_path_buf();

        // Create both visible and hidden files
        let visible_file = desktop_path.join("visible.txt");
        let hidden_file = desktop_path.join(".hidden_file");
        let hidden_folder = desktop_path.join(".hidden_folder");

        fs::write(&visible_file, "visible content").unwrap();
        fs::write(&hidden_file, "hidden content").unwrap();
        fs::create_dir(&hidden_folder).unwrap();

        // Create daemon and scan
        let mut daemon = IconDaemon {
            config: test_config(),
            desktop_dir: desktop_path.clone(),
            icons: HashMap::new(),
            watcher: None,
            event_sender: None,
        };

        daemon.scan_desktop().unwrap();

        // Only the visible file should be added
        assert!(daemon.has_icon(&visible_file), "Visible file should have an icon");
        assert!(!daemon.has_icon(&hidden_file), "Hidden file (.hidden_file) should be ignored");
        assert!(!daemon.has_icon(&hidden_folder), "Hidden folder (.hidden_folder) should be ignored");
        assert_eq!(daemon.icon_count(), 1, "Should have exactly 1 icon (visible file only)");
    }

    #[test]
    fn test_scan_desktop_ignores_files_starting_with_dot() {
        let temp_dir = TempDir::new().unwrap();
        let desktop_path = temp_dir.path().to_path_buf();

        // Create files with various hidden patterns
        let dotfile = desktop_path.join(".bashrc");
        let dot_config = desktop_path.join(".config");
        let dot_ds_store = desktop_path.join(".DS_Store");
        let normal_file = desktop_path.join("readme.txt");

        fs::write(&dotfile, "").unwrap();
        fs::create_dir(&dot_config).unwrap();
        fs::write(&dot_ds_store, "").unwrap();
        fs::write(&normal_file, "").unwrap();

        let mut daemon = IconDaemon {
            config: test_config(),
            desktop_dir: desktop_path,
            icons: HashMap::new(),
            watcher: None,
            event_sender: None,
        };

        daemon.scan_desktop().unwrap();

        assert!(!daemon.has_icon(&dotfile), ".bashrc should be ignored");
        assert!(!daemon.has_icon(&dot_config), ".config should be ignored");
        assert!(!daemon.has_icon(&dot_ds_store), ".DS_Store should be ignored");
        assert!(daemon.has_icon(&normal_file), "readme.txt should have an icon");
        assert_eq!(daemon.icon_count(), 1);
    }

    #[test]
    fn test_scan_desktop_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let desktop_path = temp_dir.path().to_path_buf();

        let mut daemon = IconDaemon {
            config: test_config(),
            desktop_dir: desktop_path,
            icons: HashMap::new(),
            watcher: None,
            event_sender: None,
        };

        daemon.scan_desktop().unwrap();

        assert_eq!(daemon.icon_count(), 0, "Empty directory should have no icons");
    }

    #[test]
    fn test_scan_desktop_nonexistent_directory() {
        let nonexistent_path = PathBuf::from("/nonexistent/desktop/path/12345");

        let mut daemon = IconDaemon {
            config: test_config(),
            desktop_dir: nonexistent_path,
            icons: HashMap::new(),
            watcher: None,
            event_sender: None,
        };

        // Should not error, just return Ok with no icons
        let result = daemon.scan_desktop();
        assert!(result.is_ok(), "Scanning nonexistent directory should not error");
        assert_eq!(daemon.icon_count(), 0);
    }

    // ========================================================================
    // Icon Count Tests
    // ========================================================================

    #[test]
    fn test_icon_count_accuracy() {
        let temp_dir = TempDir::new().unwrap();
        let desktop_path = temp_dir.path().to_path_buf();
        let mut daemon = create_test_daemon(desktop_path.clone());

        assert_eq!(daemon.icon_count(), 0, "Initial count should be 0");

        // Add some files
        for i in 0..5 {
            let file = desktop_path.join(format!("file{}.txt", i));
            fs::write(&file, "content").unwrap();
            let event = Event {
                kind: EventKind::Create(CreateKind::File),
                paths: vec![file],
                attrs: Default::default(),
            };
            daemon.handle_fs_event(event).unwrap();
        }

        assert_eq!(daemon.icon_count(), 5, "Should have 5 icons after adding 5 files");
    }

    // ========================================================================
    // Duplicate Add Prevention Tests
    // ========================================================================

    #[test]
    fn test_add_icon_prevents_duplicates() {
        let temp_dir = TempDir::new().unwrap();
        let desktop_path = temp_dir.path().to_path_buf();
        let mut daemon = create_test_daemon(desktop_path.clone());

        let test_file = desktop_path.join("test_file.txt");
        fs::write(&test_file, "content").unwrap();

        // Add the same icon twice
        daemon.add_icon(&test_file).unwrap();
        daemon.add_icon(&test_file).unwrap();

        assert_eq!(daemon.icon_count(), 1, "Should still have only 1 icon after duplicate add");
    }
}
