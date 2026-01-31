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
}
