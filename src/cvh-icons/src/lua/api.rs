//! CVH Icons Lua API
//!
//! Provides safe functions for icon scripts to interact with the system.

use anyhow::Result;
use mlua::{Lua, UserData, UserDataMethods};

/// Canvas for drawing icons
#[derive(Clone)]
pub struct Canvas {
    pub width: u32,
    pub height: u32,
    // Drawing commands are collected here
    pub commands: Vec<DrawCommand>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum DrawCommand {
    FillRect { x: f32, y: f32, w: f32, h: f32, color: String },
    StrokeRect { x: f32, y: f32, w: f32, h: f32, color: String, width: f32 },
    FillCircle { cx: f32, cy: f32, r: f32, color: String },
    StrokeCircle { cx: f32, cy: f32, r: f32, color: String, width: f32 },
    Line { x1: f32, y1: f32, x2: f32, y2: f32, color: String, width: f32 },
    Text { text: String, x: f32, y: f32, size: f32, color: String, align: String },
    Image { path: String, x: f32, y: f32, w: f32, h: f32 },
    Clear { color: String },
}

#[allow(dead_code)]
impl Canvas {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            commands: Vec::new(),
        }
    }
}

impl UserData for Canvas {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("fill_rect", |_, this, (x, y, w, h, color): (f32, f32, f32, f32, String)| {
            this.commands.push(DrawCommand::FillRect { x, y, w, h, color });
            Ok(())
        });

        methods.add_method_mut("stroke_rect", |_, this, (x, y, w, h, color, width): (f32, f32, f32, f32, String, f32)| {
            this.commands.push(DrawCommand::StrokeRect { x, y, w, h, color, width });
            Ok(())
        });

        methods.add_method_mut("fill_circle", |_, this, (cx, cy, r, color): (f32, f32, f32, String)| {
            this.commands.push(DrawCommand::FillCircle { cx, cy, r, color });
            Ok(())
        });

        methods.add_method_mut("stroke_circle", |_, this, (cx, cy, r, color, width): (f32, f32, f32, String, f32)| {
            this.commands.push(DrawCommand::StrokeCircle { cx, cy, r, color, width });
            Ok(())
        });

        methods.add_method_mut("line", |_, this, (x1, y1, x2, y2, color, width): (f32, f32, f32, f32, String, f32)| {
            this.commands.push(DrawCommand::Line { x1, y1, x2, y2, color, width });
            Ok(())
        });

        methods.add_method_mut("text", |_, this, (text, x, y, size, color, align): (String, f32, f32, f32, String, Option<String>)| {
            this.commands.push(DrawCommand::Text {
                text,
                x,
                y,
                size,
                color,
                align: align.unwrap_or_else(|| "left".to_string()),
            });
            Ok(())
        });

        methods.add_method_mut("image", |_, this, (path, x, y, w, h): (String, f32, f32, f32, f32)| {
            this.commands.push(DrawCommand::Image { path, x, y, w, h });
            Ok(())
        });

        methods.add_method_mut("clear", |_, this, color: String| {
            this.commands.push(DrawCommand::Clear { color });
            Ok(())
        });

        methods.add_method("width", |_, this, ()| Ok(this.width));
        methods.add_method("height", |_, this, ()| Ok(this.height));
    }
}

#[allow(dead_code)]
/// Install the CVH API into Lua globals
pub fn install(lua: &Lua) -> Result<()> {
    let globals = lua.globals();

    // Create the main 'cvh' table
    let cvh = lua.create_table()?;

    // Time functions
    let time = lua.create_table()?;
    time.set("now", lua.create_function(|_, ()| {
        Ok(std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0))
    })?)?;
    time.set("now_ms", lua.create_function(|_, ()| {
        Ok(std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0))
    })?)?;
    time.set("format", lua.create_function(|_, fmt: String| {
        use std::time::SystemTime;
        // Simple time formatting (production would use chrono)
        let now = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = now.as_secs();

        // Very basic formatting for common patterns
        let result = if fmt == "%H:%M:%S" {
            let h = (secs % 86400) / 3600;
            let m = (secs % 3600) / 60;
            let s = secs % 60;
            format!("{:02}:{:02}:{:02}", h, m, s)
        } else if fmt == "%H:%M" {
            let h = (secs % 86400) / 3600;
            let m = (secs % 3600) / 60;
            format!("{:02}:{:02}", h, m)
        } else {
            format!("{}", secs)
        };

        Ok(result)
    })?)?;
    cvh.set("time", time)?;

    // System info (read-only, safe)
    let system = lua.create_table()?;
    system.set("hostname", lua.create_function(|_, ()| {
        Ok(hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string()))
    })?)?;
    cvh.set("system", system)?;

    // File operations (sandboxed)
    let file = lua.create_table()?;
    file.set("exists", lua.create_function(|_, path: String| {
        // Only allow checking existence, not reading
        Ok(std::path::Path::new(&path).exists())
    })?)?;
    file.set("is_dir", lua.create_function(|_, path: String| {
        Ok(std::path::Path::new(&path).is_dir())
    })?)?;
    file.set("is_file", lua.create_function(|_, path: String| {
        Ok(std::path::Path::new(&path).is_file())
    })?)?;
    file.set("basename", lua.create_function(|_, path: String| {
        Ok(std::path::Path::new(&path)
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .unwrap_or_default())
    })?)?;
    file.set("dirname", lua.create_function(|_, path: String| {
        Ok(std::path::Path::new(&path)
            .parent()
            .and_then(|p| p.to_str())
            .map(|s| s.to_string())
            .unwrap_or_default())
    })?)?;
    file.set("extension", lua.create_function(|_, path: String| {
        Ok(std::path::Path::new(&path)
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .unwrap_or_default())
    })?)?;
    cvh.set("file", file)?;

    // Spawn external commands (will be sandboxed by daemon)
    cvh.set("open", lua.create_function(|_, path: String| {
        // Open file/folder with default handler
        // This is handled by the daemon, not executed directly
        tracing::info!("Lua requested open: {}", path);
        Ok(())
    })?)?;

    cvh.set("spawn", lua.create_function(|_, cmd: String| {
        // Spawn command (handled by daemon)
        tracing::info!("Lua requested spawn: {}", cmd);
        Ok(())
    })?)?;

    // Notifications
    cvh.set("notify", lua.create_function(|_, (title, body): (String, String)| {
        tracing::info!("Lua notification: {} - {}", title, body);
        Ok(())
    })?)?;

    globals.set("cvh", cvh)?;

    Ok(())
}
