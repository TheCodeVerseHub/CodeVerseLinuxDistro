//! Application discovery module
//!
//! Finds and parses .desktop files for application launching

use anyhow::Result;
use std::{
    env,
    fs,
    path::PathBuf,
};

use crate::Item;

/// Standard XDG application directories
fn get_application_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // User applications
    if let Some(data_home) = env::var_os("XDG_DATA_HOME") {
        dirs.push(PathBuf::from(data_home).join("applications"));
    } else if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".local/share/applications"));
    }

    // System applications
    if let Some(data_dirs) = env::var_os("XDG_DATA_DIRS") {
        for dir in env::split_paths(&data_dirs) {
            dirs.push(dir.join("applications"));
        }
    } else {
        dirs.push(PathBuf::from("/usr/local/share/applications"));
        dirs.push(PathBuf::from("/usr/share/applications"));
    }

    dirs
}

/// Parse a .desktop file and extract relevant info
fn parse_desktop_file(path: &PathBuf) -> Option<Item> {
    let content = fs::read_to_string(path).ok()?;

    let mut name = None;
    let mut exec = None;
    let mut icon = None;
    let mut no_display = false;
    let mut hidden = false;
    let mut terminal = false;
    let mut in_desktop_entry = false;

    for line in content.lines() {
        let line = line.trim();

        if line.starts_with('[') {
            in_desktop_entry = line == "[Desktop Entry]";
            continue;
        }

        if !in_desktop_entry {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            match key.trim() {
                "Name" if name.is_none() => name = Some(value.trim().to_string()),
                "Exec" => exec = Some(value.trim().to_string()),
                "Icon" => icon = Some(value.trim().to_string()),
                "NoDisplay" => no_display = value.trim().eq_ignore_ascii_case("true"),
                "Hidden" => hidden = value.trim().eq_ignore_ascii_case("true"),
                "Terminal" => terminal = value.trim().eq_ignore_ascii_case("true"),
                _ => {}
            }
        }
    }

    // Skip hidden or no-display entries
    if no_display || hidden {
        return None;
    }

    let name = name?;
    let exec = exec?;

    // Clean up exec (remove field codes like %f, %u, etc.)
    let exec_clean = exec
        .replace("%f", "")
        .replace("%F", "")
        .replace("%u", "")
        .replace("%U", "")
        .replace("%d", "")
        .replace("%D", "")
        .replace("%n", "")
        .replace("%N", "")
        .replace("%i", "")
        .replace("%c", "")
        .replace("%k", "")
        .replace("%v", "")
        .replace("%m", "")
        .trim()
        .to_string();

    // Wrap in terminal if needed
    let final_exec = if terminal {
        format!("foot -e {}", exec_clean)
    } else {
        exec_clean
    };

    Some(Item {
        display: name,
        value: final_exec,
        icon,
    })
}

/// Load all applications from XDG directories
pub fn load_applications() -> Result<Vec<Item>> {
    let mut items = Vec::new();
    let mut seen_names = std::collections::HashSet::new();

    for dir in get_application_dirs() {
        if !dir.exists() {
            continue;
        }

        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "desktop") {
                if let Some(item) = parse_desktop_file(&path) {
                    // Deduplicate by name
                    if seen_names.insert(item.display.clone()) {
                        items.push(item);
                    }
                }
            }
        }
    }

    // Sort alphabetically
    items.sort_by(|a, b| a.display.to_lowercase().cmp(&b.display.to_lowercase()));

    Ok(items)
}
