//! Icon daemon - manages desktop icons
//!
//! Watches the desktop directory and manages icon windows.

use anyhow::{Context, Result};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::config::Config;
use crate::icons::DesktopIcon;

/// Icon daemon that manages desktop icons
pub struct IconDaemon {
    config: Config,
    desktop_dir: PathBuf,
    icons: HashMap<PathBuf, DesktopIcon>,
    watcher: Option<RecommendedWatcher>,
    event_rx: Option<mpsc::Receiver<notify::Result<Event>>>,
}

impl IconDaemon {
    /// Create a new icon daemon
    pub async fn new(config: Config, desktop_dir: PathBuf) -> Result<Self> {
        info!("Initializing icon daemon for {}", desktop_dir.display());

        let mut daemon = Self {
            config,
            desktop_dir,
            icons: HashMap::new(),
            watcher: None,
            event_rx: None,
        };

        // Set up file watcher
        daemon.setup_watcher()?;

        // Initial scan of desktop directory
        daemon.scan_desktop().await?;

        Ok(daemon)
    }

    /// Set up file system watcher for the desktop directory
    fn setup_watcher(&mut self) -> Result<()> {
        let (tx, rx) = mpsc::channel();

        let watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            notify::Config::default(),
        )?;

        self.watcher = Some(watcher);
        self.event_rx = Some(rx);

        // Start watching
        if let Some(ref mut watcher) = self.watcher {
            watcher.watch(&self.desktop_dir, RecursiveMode::NonRecursive)?;
            info!("Watching desktop directory: {}", self.desktop_dir.display());
        }

        Ok(())
    }

    /// Scan the desktop directory for files/folders
    async fn scan_desktop(&mut self) -> Result<()> {
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

            self.add_icon(&path).await?;
        }

        info!("Loaded {} desktop icons", self.icons.len());
        Ok(())
    }

    /// Add an icon for a file/folder
    async fn add_icon(&mut self, path: &Path) -> Result<()> {
        if self.icons.contains_key(path) {
            return Ok(());
        }

        let icon = DesktopIcon::new(path, &self.config)?;
        debug!("Added icon for: {}", path.display());
        self.icons.insert(path.to_path_buf(), icon);

        Ok(())
    }

    /// Remove an icon
    fn remove_icon(&mut self, path: &Path) {
        if self.icons.remove(path).is_some() {
            debug!("Removed icon for: {}", path.display());
        }
    }

    /// Handle a file system event
    async fn handle_fs_event(&mut self, event: Event) -> Result<()> {
        use notify::EventKind;

        match event.kind {
            EventKind::Create(_) => {
                for path in event.paths {
                    self.add_icon(&path).await?;
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
                        self.add_icon(&path).await?;
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Run the main daemon loop
    pub async fn run(&mut self) -> Result<()> {
        info!("Icon daemon running");

        // Take ownership of the receiver
        let event_rx = self.event_rx.take()
            .context("Event receiver not initialized")?;

        loop {
            // Check for file system events (non-blocking)
            match event_rx.try_recv() {
                Ok(Ok(event)) => {
                    if let Err(e) = self.handle_fs_event(event).await {
                        error!("Error handling fs event: {}", e);
                    }
                }
                Ok(Err(e)) => {
                    error!("Watcher error: {}", e);
                }
                Err(mpsc::TryRecvError::Empty) => {
                    // No events, continue
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    error!("Watcher disconnected");
                    break;
                }
            }

            // Update icons
            for icon in self.icons.values_mut() {
                if let Err(e) = icon.update() {
                    warn!("Error updating icon: {}", e);
                }
            }

            // Small sleep to avoid busy loop
            tokio::time::sleep(tokio::time::Duration::from_millis(16)).await;
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
            event_rx: None,
        }
    }

    // ========================================================================
    // File Create Event Tests
    // ========================================================================

    #[tokio::test]
    async fn test_file_create_event_adds_icon_to_hashmap() {
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

        daemon.handle_fs_event(event).await.unwrap();

        assert!(daemon.has_icon(&test_file), "Icon should be added to HashMap after Create event");
        assert_eq!(daemon.icon_count(), 1, "Should have exactly 1 icon");
    }

    #[tokio::test]
    async fn test_folder_create_event_adds_icon_to_hashmap() {
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

        daemon.handle_fs_event(event).await.unwrap();

        assert!(daemon.has_icon(&test_folder), "Icon should be added for folder Create event");
    }

    #[tokio::test]
    async fn test_create_event_multiple_paths() {
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

        daemon.handle_fs_event(event).await.unwrap();

        assert!(daemon.has_icon(&test_file1), "Icon should be added for first file");
        assert!(daemon.has_icon(&test_file2), "Icon should be added for second file");
        assert_eq!(daemon.icon_count(), 2, "Should have exactly 2 icons");
    }

    // ========================================================================
    // File Delete Event Tests
    // ========================================================================

    #[tokio::test]
    async fn test_file_delete_event_removes_icon_from_hashmap() {
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
        daemon.handle_fs_event(create_event).await.unwrap();
        assert!(daemon.has_icon(&test_file), "Icon should exist after Create");

        // Now simulate a Remove event
        let remove_event = Event {
            kind: EventKind::Remove(RemoveKind::File),
            paths: vec![test_file.clone()],
            attrs: Default::default(),
        };

        daemon.handle_fs_event(remove_event).await.unwrap();

        assert!(!daemon.has_icon(&test_file), "Icon should be removed from HashMap after Remove event");
        assert_eq!(daemon.icon_count(), 0, "Should have no icons after removal");
    }

    #[tokio::test]
    async fn test_delete_event_for_nonexistent_icon_is_harmless() {
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
        let result = daemon.handle_fs_event(remove_event).await;
        assert!(result.is_ok(), "Removing nonexistent icon should not error");
    }

    // ========================================================================
    // File Modify Event Tests
    // ========================================================================

    #[tokio::test]
    async fn test_file_modify_event_refreshes_icon() {
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
        daemon.handle_fs_event(create_event).await.unwrap();
        assert_eq!(daemon.icon_count(), 1, "Should have 1 icon before modify");

        // Modify the file
        fs::write(&test_file, "modified content").unwrap();

        // Simulate a Modify event
        let modify_event = Event {
            kind: EventKind::Modify(ModifyKind::Data(notify::event::DataChange::Content)),
            paths: vec![test_file.clone()],
            attrs: Default::default(),
        };

        daemon.handle_fs_event(modify_event).await.unwrap();

        // Icon should still exist (was refreshed)
        assert!(daemon.has_icon(&test_file), "Icon should still exist after Modify event");
        assert_eq!(daemon.icon_count(), 1, "Should still have exactly 1 icon");
    }

    #[tokio::test]
    async fn test_modify_event_on_unknown_path_is_ignored() {
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
        daemon.handle_fs_event(modify_event).await.unwrap();

        assert!(!daemon.has_icon(&unknown_file), "Modify event should not add new icons");
        assert_eq!(daemon.icon_count(), 0, "Should have no icons");
    }

    // ========================================================================
    // Hidden File Filtering Tests
    // ========================================================================

    #[tokio::test]
    async fn test_scan_desktop_ignores_hidden_files() {
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
            event_rx: None,
        };

        daemon.scan_desktop().await.unwrap();

        // Only the visible file should be added
        assert!(daemon.has_icon(&visible_file), "Visible file should have an icon");
        assert!(!daemon.has_icon(&hidden_file), "Hidden file (.hidden_file) should be ignored");
        assert!(!daemon.has_icon(&hidden_folder), "Hidden folder (.hidden_folder) should be ignored");
        assert_eq!(daemon.icon_count(), 1, "Should have exactly 1 icon (visible file only)");
    }

    #[tokio::test]
    async fn test_scan_desktop_ignores_files_starting_with_dot() {
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
            event_rx: None,
        };

        daemon.scan_desktop().await.unwrap();

        assert!(!daemon.has_icon(&dotfile), ".bashrc should be ignored");
        assert!(!daemon.has_icon(&dot_config), ".config should be ignored");
        assert!(!daemon.has_icon(&dot_ds_store), ".DS_Store should be ignored");
        assert!(daemon.has_icon(&normal_file), "readme.txt should have an icon");
        assert_eq!(daemon.icon_count(), 1);
    }

    #[tokio::test]
    async fn test_scan_desktop_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let desktop_path = temp_dir.path().to_path_buf();

        let mut daemon = IconDaemon {
            config: test_config(),
            desktop_dir: desktop_path,
            icons: HashMap::new(),
            watcher: None,
            event_rx: None,
        };

        daemon.scan_desktop().await.unwrap();

        assert_eq!(daemon.icon_count(), 0, "Empty directory should have no icons");
    }

    #[tokio::test]
    async fn test_scan_desktop_nonexistent_directory() {
        let nonexistent_path = PathBuf::from("/nonexistent/desktop/path/12345");

        let mut daemon = IconDaemon {
            config: test_config(),
            desktop_dir: nonexistent_path,
            icons: HashMap::new(),
            watcher: None,
            event_rx: None,
        };

        // Should not error, just return Ok with no icons
        let result = daemon.scan_desktop().await;
        assert!(result.is_ok(), "Scanning nonexistent directory should not error");
        assert_eq!(daemon.icon_count(), 0);
    }

    // ========================================================================
    // Icon Count Tests
    // ========================================================================

    #[tokio::test]
    async fn test_icon_count_accuracy() {
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
            daemon.handle_fs_event(event).await.unwrap();
        }

        assert_eq!(daemon.icon_count(), 5, "Should have 5 icons after adding 5 files");
    }

    // ========================================================================
    // Duplicate Add Prevention Tests
    // ========================================================================

    #[tokio::test]
    async fn test_add_icon_prevents_duplicates() {
        let temp_dir = TempDir::new().unwrap();
        let desktop_path = temp_dir.path().to_path_buf();
        let mut daemon = create_test_daemon(desktop_path.clone());

        let test_file = desktop_path.join("test_file.txt");
        fs::write(&test_file, "content").unwrap();

        // Add the same icon twice
        daemon.add_icon(&test_file).await.unwrap();
        daemon.add_icon(&test_file).await.unwrap();

        assert_eq!(daemon.icon_count(), 1, "Should still have only 1 icon after duplicate add");
    }
}
