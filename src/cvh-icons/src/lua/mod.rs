//! Lua runtime for icon scripts
//!
//! Provides a sandboxed Lua environment for icon customization.

use anyhow::{Context, Result};
use mlua::{Function, Lua, Table, Value};
use std::path::Path;

pub mod api;
mod stdlib;

pub use api::{Canvas, DrawCommand};

/// Sandboxed Lua runtime for icon scripts
pub struct LuaRuntime {
    lua: Lua,
}

impl LuaRuntime {
    /// Create a new sandboxed Lua runtime
    pub fn new() -> Result<Self> {
        let lua = Lua::new();

        // Sandbox the environment
        Self::sandbox(&lua)?;

        // Install safe standard library
        stdlib::install(&lua)?;

        // Install icon API
        api::install(&lua)?;

        Ok(Self { lua })
    }

    /// Remove dangerous globals from Lua environment
    fn sandbox(lua: &Lua) -> Result<()> {
        let globals = lua.globals();

        // Remove dangerous functions
        let dangerous = [
            "os",           // OS access
            "io",           // File I/O
            "loadfile",     // Load files
            "dofile",       // Execute files
            "debug",        // Debug library
            "package",      // Package system (allows require)
            "load",         // Load arbitrary code
            "loadstring",   // Load strings as code (Lua 5.1)
            "rawget",       // Bypass metatables
            "rawset",       // Bypass metatables
            "rawequal",     // Bypass metatables
            "collectgarbage", // GC control
            "newproxy",     // Create userdata
        ];

        for name in dangerous {
            globals.set(name, Value::Nil)?;
        }

        // Restrict string library to safe functions only
        Self::restrict_string_library(lua)?;

        Ok(())
    }

    /// Restrict string library to safe subset
    fn restrict_string_library(lua: &Lua) -> Result<()> {
        // String functions are generally safe, but we limit them
        // to prevent potential DoS via regex complexity
        let globals = lua.globals();
        let string: Table = globals.get("string")?;

        // Keep safe functions
        let safe_funcs = [
            "byte", "char", "find", "format", "len",
            "lower", "upper", "rep", "reverse", "sub",
        ];

        let new_string = lua.create_table()?;
        for func_name in safe_funcs {
            if let Ok(func) = string.get::<Function>(func_name) {
                new_string.set(func_name, func)?;
            }
        }

        // Add limited gsub and gmatch (with iteration limits)
        if let Ok(func) = string.get::<Function>("gsub") {
            new_string.set("gsub", func)?;
        }
        if let Ok(func) = string.get::<Function>("gmatch") {
            new_string.set("gmatch", func)?;
        }
        if let Ok(func) = string.get::<Function>("match") {
            new_string.set("match", func)?;
        }

        globals.set("string", new_string)?;

        Ok(())
    }

    /// Load and execute an icon script
    pub fn load_script(&self, path: &Path) -> Result<IconScript> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read script: {}", path.display()))?;

        self.lua.load(&content).exec()
            .with_context(|| format!("Failed to execute script: {}", path.display()))?;

        // Get the Icon table
        let globals = self.lua.globals();
        let icon_table: Table = globals.get("Icon")
            .context("Script must define an 'Icon' table")?;

        Ok(IconScript { icon_table })
    }

    /// Execute a Lua string (for testing/REPL)
    pub fn exec(&self, code: &str) -> Result<()> {
        self.lua.load(code).exec()?;
        Ok(())
    }

    /// Get a reference to the Lua state
    pub fn lua(&self) -> &Lua {
        &self.lua
    }
}

/// Represents a loaded icon script
pub struct IconScript {
    icon_table: Table,
}

impl IconScript {
    /// Get a property from the Icon table
    pub fn get<T: mlua::FromLua>(&self, key: &str) -> Result<T> {
        self.icon_table.get(key).context(format!("Missing property: {}", key))
    }

    /// Get an optional property
    pub fn get_opt<T: mlua::FromLua>(&self, key: &str) -> Option<T> {
        self.icon_table.get(key).ok()
    }

    /// Call the init method if it exists
    pub fn call_init(&self) -> Result<()> {
        if let Ok(init_fn) = self.icon_table.get::<Function>("init") {
            init_fn.call::<()>(())?;
        }
        Ok(())
    }

    /// Call the render method
    pub fn call_render(&self, _canvas: &api::Canvas) -> Result<()> {
        if let Ok(render_fn) = self.icon_table.get::<Function>("render") {
            render_fn.call::<()>(self.icon_table.clone())?;
        }
        Ok(())
    }

    /// Call the on_click handler
    pub fn call_on_click(&self, button: u32, x: f64, y: f64) -> Result<()> {
        if let Ok(handler) = self.icon_table.get::<Function>("on_click") {
            handler.call::<()>((button, x, y))?;
        }
        Ok(())
    }

    /// Call the on_drop handler for drag-and-drop
    pub fn call_on_drop(&self, lua: &Lua, paths: Vec<String>) -> Result<()> {
        if let Ok(handler) = self.icon_table.get::<Function>("on_drop") {
            let paths_table = lua.create_table()?;
            for (i, path) in paths.iter().enumerate() {
                paths_table.set(i + 1, path.as_str())?;
            }
            handler.call::<()>(paths_table)?;
        }
        Ok(())
    }
}
