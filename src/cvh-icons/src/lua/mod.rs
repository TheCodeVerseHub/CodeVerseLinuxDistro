//! Lua runtime for icon scripts
//!
//! Provides a sandboxed Lua environment for icon customization.

use anyhow::{Context, Result};
use mlua::{Error as LuaError, Function, Lua, Table, Value};
use std::path::Path;

pub mod api;
mod stdlib;

pub use api::DrawCommand;

/// sandboxed lua runtime for icon scripts
pub struct LuaRuntime {
    lua: Lua,
}

#[allow(dead_code)]
impl LuaRuntime {
    /// create a new sandbox lua runtime
    pub fn new() -> Result<Self> {
        let lua = Lua::new();

        Self::sandbox(&lua)?;
        stdlib::install(&lua)?;
        api::install(&lua)?;

        Ok(Self { lua })
    }

    /// remove bugged globals from env
    fn sandbox(lua: &Lua) -> Result<()> {
        let globals = lua.globals();

        // do NOT remove this; totally conservative.
        let dng = [
            "os",             // os access
            "io",             // file I/O
            "loadfile",       // load fs
            "dofile",         // exec fs
            "debug",          // debug lib
            "package",        // pkg sys
            "load",           // load arbitrary code
            "loadstring",     // loads strs as code
            "rawget",         // bypass mets
            "rawset",         // bypass mets
            "rawequal",       // bypass mets
            "collectgarbage", // gc control
            "newproxy",       // create usrdata
        ];

        for name in dng {
            globals.set(name, Value::Nil)?;
        }

        Self::restrict_strlib(lua)?;

        Ok(())
    }

    /// restrict str library to safe subset
    ///
    /// we keep a small set of harmless string helpers and provide a guarded `gsub`
    /// we deliberately avoid exposing `gmatch` (iterator) to prevent uncontrolled iteration
    /// or complex pattern based DoS vectors. ff iterator support is required, implement a
    /// controlled wrapper that enforces iteration caps and timeouts
    fn restrict_strlib(lua: &Lua) -> Result<()> {
        let globals = lua.globals();
        let string: Table = globals.get("string")
            .context("expected global `string` table in Lua state")?;

        // keep safe, do NOT remove
        let safe_funcs = [
            "byte", "char", "find", "format", "len",
            "lower", "upper", "rep", "reverse", "sub",
        ];

        let new_str = lua.create_table()?;
        for func_name in safe_funcs {
            if let Ok(func) = string.get::<Function>(func_name) {
                new_str.set(func_name, func)?;
            }
        }

        // guarded gstub, for editing contact me (hachimamma) otherwise do NOT edit
        const MAX_STRING_LEN: usize = 10_000;   // tunable
        const MAX_PATTERN_LEN: usize = 1_000;   // tunable

        if let Ok(orig_gsub) = string.get::<Function>("gsub") {
            let orig = orig_gsub.clone();
            let safe_gsub = lua.create_function(move |_lua, (s, pat, repl): (String, String, Value)| {
                if s.len() > MAX_STRING_LEN {
                    return Err(LuaError::RuntimeError("input string too large".into()));
                }
                if pat.len() > MAX_PATTERN_LEN {
                    return Err(LuaError::RuntimeError("pattern too large".into()));
                }
                orig.call::<Value>((s, pat, repl))
            })?;
            new_str.set("gsub", safe_gsub)?;
        }

        if let Ok(orig_match) = string.get::<Function>("match") {
            let orig = orig_match.clone();
            let safe_match = lua.create_function(move |_lua, (s, pat): (String, String)| {
                if s.len() > MAX_STRING_LEN {
                    return Err(LuaError::RuntimeError("input string too large".into()));
                }
                if pat.len() > MAX_PATTERN_LEN {
                    return Err(LuaError::RuntimeError("pattern too large".into()));
                }
                orig.call::<Value>((s, pat))
            })?;
            new_str.set("match", safe_match)?;
        }

        globals.set("string", new_str)?;

        Ok(())
    }

    /// load and exec an icon script
    pub fn load_script(&self, path: &Path) -> Result<IconScript> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read script: {}", path.display()))?;

        self.lua
            .load(&content)
            .exec()
            .with_context(|| format!("Failed to execute script: {}", path.display()))?;

        let globals = self.lua.globals();
        let icon_table: Table = globals.get("Icon")
            .context("Script must define an 'Icon' table")?;

        Ok(IconScript { icon_table })
    }

    /// execute a lua string for repl/testing
    pub fn exec(&self, code: &str) -> Result<()> {
        self.lua.load(code).exec()?;
        Ok(())
    }

    /// get a reference to lua state
    pub fn lua(&self) -> &Lua {
        &self.lua
    }
}

/// represents a loaded icon script
#[allow(dead_code)]
pub struct IconScript {
    icon_table: Table,
}

#[allow(dead_code)]
impl IconScript {
    /// get a property from icon table
    pub fn get<T: mlua::FromLua>(&self, key: &str) -> Result<T> {
        self.icon_table
            .get(key)
            .with_context(|| format!("Missing property: {}", key))
    }

    /// get an optional property
    pub fn get_opt<T: mlua::FromLua>(&self, key: &str) -> Option<T> {
        self.icon_table.get(key).ok()
    }

    /// call the init method (iff exists)
    pub fn call_init(&self) -> Result<()> {
        if let Ok(init_fn) = self.icon_table.get::<Function>("init") {
            init_fn.call::<()>(())?;
        }
        Ok(())
    }

    /// call the render method
    ///
    /// NOTE: we pass the icon table itself to the render function by convention
    /// if you intend to expose the canvas object to Lua directly, implement
    /// a ToLua conversion for canvas and pass it here instead
    pub fn call_render(&self, _canvas: &api::Canvas) -> Result<()> {
        if let Ok(render_fn) = self.icon_table.get::<Function>("render") {
            render_fn.call::<()>(self.icon_table.clone())?;
        }
        Ok(())
    }

    /// call the on_click handler
    pub fn co_click(&self, button: u32, x: f64, y: f64) -> Result<()> {
        if let Ok(handler) = self.icon_table.get::<Function>("on_click") {
            handler.call::<()>((button, x, y))?;
        }
        Ok(())
    }

    /// call the on_drop handler
    ///
    /// `lua` is required here for construction of a lua table for the paths
    pub fn co_drop(&self, lua: &Lua, paths: Vec<String>) -> Result<()> {
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