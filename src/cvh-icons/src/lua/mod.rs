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

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a sandboxed Lua runtime for testing
    fn create_test_runtime() -> LuaRuntime {
        LuaRuntime::new().expect("Failed to create Lua runtime")
    }

    // ========================================================================
    // Lua Sandboxing Tests - Dangerous Globals Removal
    // ========================================================================

    #[test]
    fn test_dangerous_global_os_is_nil() {
        let rt = create_test_runtime();
        let result: Value = rt.lua().globals().get("os").unwrap();
        assert!(matches!(result, Value::Nil), "os global should be nil");
    }

    #[test]
    fn test_dangerous_global_io_is_nil() {
        let rt = create_test_runtime();
        let result: Value = rt.lua().globals().get("io").unwrap();
        assert!(matches!(result, Value::Nil), "io global should be nil");
    }

    #[test]
    fn test_dangerous_global_loadfile_is_nil() {
        let rt = create_test_runtime();
        let result: Value = rt.lua().globals().get("loadfile").unwrap();
        assert!(matches!(result, Value::Nil), "loadfile global should be nil");
    }

    #[test]
    fn test_dangerous_global_dofile_is_nil() {
        let rt = create_test_runtime();
        let result: Value = rt.lua().globals().get("dofile").unwrap();
        assert!(matches!(result, Value::Nil), "dofile global should be nil");
    }

    #[test]
    fn test_dangerous_global_debug_is_nil() {
        let rt = create_test_runtime();
        let result: Value = rt.lua().globals().get("debug").unwrap();
        assert!(matches!(result, Value::Nil), "debug global should be nil");
    }

    #[test]
    fn test_dangerous_global_package_is_nil() {
        let rt = create_test_runtime();
        let result: Value = rt.lua().globals().get("package").unwrap();
        assert!(matches!(result, Value::Nil), "package global should be nil");
    }

    #[test]
    fn test_dangerous_global_load_is_nil() {
        let rt = create_test_runtime();
        let result: Value = rt.lua().globals().get("load").unwrap();
        assert!(matches!(result, Value::Nil), "load global should be nil");
    }

    #[test]
    fn test_dangerous_global_loadstring_is_nil() {
        let rt = create_test_runtime();
        let result: Value = rt.lua().globals().get("loadstring").unwrap();
        assert!(matches!(result, Value::Nil), "loadstring global should be nil");
    }

    #[test]
    fn test_dangerous_global_rawget_is_nil() {
        let rt = create_test_runtime();
        let result: Value = rt.lua().globals().get("rawget").unwrap();
        assert!(matches!(result, Value::Nil), "rawget global should be nil");
    }

    #[test]
    fn test_dangerous_global_rawset_is_nil() {
        let rt = create_test_runtime();
        let result: Value = rt.lua().globals().get("rawset").unwrap();
        assert!(matches!(result, Value::Nil), "rawset global should be nil");
    }

    #[test]
    fn test_dangerous_global_rawequal_is_nil() {
        let rt = create_test_runtime();
        let result: Value = rt.lua().globals().get("rawequal").unwrap();
        assert!(matches!(result, Value::Nil), "rawequal global should be nil");
    }

    #[test]
    fn test_dangerous_global_collectgarbage_is_nil() {
        let rt = create_test_runtime();
        let result: Value = rt.lua().globals().get("collectgarbage").unwrap();
        assert!(matches!(result, Value::Nil), "collectgarbage global should be nil");
    }

    #[test]
    fn test_dangerous_global_newproxy_is_nil() {
        let rt = create_test_runtime();
        let result: Value = rt.lua().globals().get("newproxy").unwrap();
        assert!(matches!(result, Value::Nil), "newproxy global should be nil");
    }

    // ========================================================================
    // String Library Restriction Tests
    // ========================================================================

    #[test]
    fn test_string_gsub_rejects_oversized_string() {
        let rt = create_test_runtime();
        // Create a string larger than MAX_STRING_LEN (10,000)
        let oversized = "a".repeat(10_001);
        rt.lua().globals().set("test_str", oversized).unwrap();

        let result = rt.exec("result = string.gsub(test_str, 'a', 'b')");
        assert!(result.is_err(), "string.gsub should reject strings > 10,000 chars");

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("input string too large"),
            "Error should mention 'input string too large', got: {}", err_msg
        );
    }

    #[test]
    fn test_string_gsub_accepts_valid_sized_string() {
        let rt = create_test_runtime();
        // Create a string at exactly MAX_STRING_LEN (10,000)
        let valid = "a".repeat(10_000);
        rt.lua().globals().set("test_str", valid).unwrap();

        let result = rt.exec("result = string.gsub(test_str, 'a', 'b')");
        assert!(result.is_ok(), "string.gsub should accept strings of exactly 10,000 chars");
    }

    #[test]
    fn test_string_gsub_rejects_oversized_pattern() {
        let rt = create_test_runtime();
        // Create a pattern larger than MAX_PATTERN_LEN (1,000)
        let oversized_pattern = "a".repeat(1_001);
        rt.lua().globals().set("test_pattern", oversized_pattern).unwrap();

        let result = rt.exec("result = string.gsub('hello', test_pattern, 'x')");
        assert!(result.is_err(), "string.gsub should reject patterns > 1,000 chars");

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("pattern too large"),
            "Error should mention 'pattern too large', got: {}", err_msg
        );
    }

    #[test]
    fn test_string_match_rejects_oversized_string() {
        let rt = create_test_runtime();
        // Create a string larger than MAX_STRING_LEN (10,000)
        let oversized = "a".repeat(10_001);
        rt.lua().globals().set("test_str", oversized).unwrap();

        let result = rt.exec("result = string.match(test_str, 'a')");
        assert!(result.is_err(), "string.match should reject strings > 10,000 chars");

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("input string too large"),
            "Error should mention 'input string too large', got: {}", err_msg
        );
    }

    #[test]
    fn test_string_match_rejects_oversized_pattern() {
        let rt = create_test_runtime();
        // Create a pattern larger than MAX_PATTERN_LEN (1,000)
        let oversized_pattern = "a".repeat(1_001);
        rt.lua().globals().set("test_pattern", oversized_pattern).unwrap();

        let result = rt.exec("result = string.match('hello', test_pattern)");
        assert!(result.is_err(), "string.match should reject patterns > 1,000 chars");

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("pattern too large"),
            "Error should mention 'pattern too large', got: {}", err_msg
        );
    }

    #[test]
    fn test_string_match_accepts_valid_sized_pattern() {
        let rt = create_test_runtime();
        // Create a pattern at exactly MAX_PATTERN_LEN (1,000)
        let valid_pattern = "a".repeat(1_000);
        rt.lua().globals().set("test_pattern", valid_pattern).unwrap();

        let result = rt.exec("result = string.match('hello', test_pattern)");
        assert!(result.is_ok(), "string.match should accept patterns of exactly 1,000 chars");
    }

    // ========================================================================
    // Safe String Functions Tests
    // ========================================================================

    #[test]
    fn test_safe_string_functions_available() {
        let rt = create_test_runtime();
        let safe_funcs = ["byte", "char", "find", "format", "len", "lower", "upper", "rep", "reverse", "sub"];

        for func_name in safe_funcs {
            let code = format!("return string.{}", func_name);
            let result = rt.lua().load(&code).eval::<Value>().unwrap();
            assert!(
                matches!(result, Value::Function(_)),
                "string.{} should be a function", func_name
            );
        }
    }

    #[test]
    fn test_string_len_works() {
        let rt = create_test_runtime();
        rt.exec("test_result = string.len('hello')").unwrap();
        let result: i64 = rt.lua().globals().get("test_result").unwrap();
        assert_eq!(result, 5, "string.len('hello') should return 5");
    }

    #[test]
    fn test_string_upper_works() {
        let rt = create_test_runtime();
        rt.exec("test_result = string.upper('hello')").unwrap();
        let result: String = rt.lua().globals().get("test_result").unwrap();
        assert_eq!(result, "HELLO", "string.upper('hello') should return 'HELLO'");
    }

    #[test]
    fn test_string_lower_works() {
        let rt = create_test_runtime();
        rt.exec("test_result = string.lower('HELLO')").unwrap();
        let result: String = rt.lua().globals().get("test_result").unwrap();
        assert_eq!(result, "hello", "string.lower('HELLO') should return 'hello'");
    }

    // ========================================================================
    // Safe Standard Library Tests
    // ========================================================================

    #[test]
    fn test_print_is_available() {
        let rt = create_test_runtime();
        let result: Value = rt.lua().globals().get("print").unwrap();
        assert!(matches!(result, Value::Function(_)), "print should be a function");
    }

    #[test]
    fn test_type_function_works() {
        let rt = create_test_runtime();
        rt.exec("test_result = type('hello')").unwrap();
        let result: String = rt.lua().globals().get("test_result").unwrap();
        assert_eq!(result, "string", "type('hello') should return 'string'");

        rt.exec("test_result = type(42)").unwrap();
        let result: String = rt.lua().globals().get("test_result").unwrap();
        assert_eq!(result, "number", "type(42) should return 'number'");

        rt.exec("test_result = type({})").unwrap();
        let result: String = rt.lua().globals().get("test_result").unwrap();
        assert_eq!(result, "table", "type(table) should return 'table'");
    }

    #[test]
    fn test_tonumber_works() {
        let rt = create_test_runtime();
        rt.exec("test_result = tonumber('42')").unwrap();
        let result: f64 = rt.lua().globals().get("test_result").unwrap();
        assert_eq!(result, 42.0, "tonumber('42') should return 42");

        rt.exec("test_result = tonumber('3.14')").unwrap();
        let result: f64 = rt.lua().globals().get("test_result").unwrap();
        assert!((result - 3.14).abs() < 0.001, "tonumber('3.14') should return 3.14");
    }

    #[test]
    fn test_tostring_works() {
        let rt = create_test_runtime();
        rt.exec("test_result = tostring(42)").unwrap();
        let result: String = rt.lua().globals().get("test_result").unwrap();
        assert_eq!(result, "42", "tostring(42) should return '42'");

        rt.exec("test_result = tostring(true)").unwrap();
        let result: String = rt.lua().globals().get("test_result").unwrap();
        assert_eq!(result, "true", "tostring(true) should return 'true'");
    }

    #[test]
    fn test_pairs_is_available() {
        let rt = create_test_runtime();
        let result: Value = rt.lua().globals().get("pairs").unwrap();
        assert!(matches!(result, Value::Function(_)), "pairs should be a function");
    }

    #[test]
    fn test_ipairs_is_available() {
        let rt = create_test_runtime();
        let result: Value = rt.lua().globals().get("ipairs").unwrap();
        assert!(matches!(result, Value::Function(_)), "ipairs should be a function");
    }

    #[test]
    fn test_math_library_available() {
        let rt = create_test_runtime();
        let math: Table = rt.lua().globals().get("math").unwrap();

        // Check key math functions
        let funcs = ["abs", "floor", "ceil", "min", "max", "sqrt", "sin", "cos", "tan", "random"];
        for func_name in funcs {
            let func: Value = math.get(func_name).unwrap();
            assert!(matches!(func, Value::Function(_)), "math.{} should be a function", func_name);
        }

        // Check pi constant
        let pi: f64 = math.get("pi").unwrap();
        assert!((pi - std::f64::consts::PI).abs() < 0.0001, "math.pi should equal PI");
    }

    #[test]
    fn test_math_abs_works() {
        let rt = create_test_runtime();
        rt.exec("test_result = math.abs(-42)").unwrap();
        let result: f64 = rt.lua().globals().get("test_result").unwrap();
        assert_eq!(result, 42.0, "math.abs(-42) should return 42");
    }

    #[test]
    fn test_math_floor_works() {
        let rt = create_test_runtime();
        rt.exec("test_result = math.floor(3.7)").unwrap();
        let result: f64 = rt.lua().globals().get("test_result").unwrap();
        assert_eq!(result, 3.0, "math.floor(3.7) should return 3");
    }

    #[test]
    fn test_table_library_available() {
        let rt = create_test_runtime();
        let table_lib: Table = rt.lua().globals().get("table").unwrap();

        let funcs = ["insert", "remove", "concat"];
        for func_name in funcs {
            let func: Value = table_lib.get(func_name).unwrap();
            assert!(matches!(func, Value::Function(_)), "table.{} should be a function", func_name);
        }
    }

    #[test]
    fn test_table_insert_works() {
        // NOTE: The current stdlib implementation requires 3 arguments for table.insert
        // (table, position, value). The standard Lua 2-argument form table.insert(t, value)
        // does NOT work correctly - this is a known bug in stdlib.rs:132-149.
        // Using the 3-argument form as a workaround.
        let rt = create_test_runtime();
        rt.exec(r#"
            t = {}
            table.insert(t, 1, "a")
            table.insert(t, 2, "b")
            test_len = #t
            test_first = t[1]
            test_second = t[2]
        "#).unwrap();

        let len: i64 = rt.lua().globals().get("test_len").unwrap();
        let first: String = rt.lua().globals().get("test_first").unwrap();
        let second: String = rt.lua().globals().get("test_second").unwrap();

        assert_eq!(len, 2, "table should have 2 elements");
        assert_eq!(first, "a", "first element should be 'a'");
        assert_eq!(second, "b", "second element should be 'b'");
    }

    #[test]
    fn test_table_insert_two_arg_form_is_broken() {
        // BUG DOCUMENTATION: The 2-argument form of table.insert(t, value)
        // does not work as expected in the current stdlib implementation.
        // This test documents the current (buggy) behavior.
        let rt = create_test_runtime();

        // This should work in standard Lua but fails here
        let result = rt.exec(r#"
            t = {}
            table.insert(t, "a")
        "#);

        // Current behavior: the 2-argument form fails
        assert!(result.is_err(),
            "BUG: table.insert(t, value) two-argument form does not work correctly");
    }

    #[test]
    fn test_assert_function_available() {
        let rt = create_test_runtime();
        let result: Value = rt.lua().globals().get("assert").unwrap();
        assert!(matches!(result, Value::Function(_)), "assert should be a function");
    }

    #[test]
    fn test_error_function_available() {
        let rt = create_test_runtime();
        let result: Value = rt.lua().globals().get("error").unwrap();
        assert!(matches!(result, Value::Function(_)), "error should be a function");
    }

    // ========================================================================
    // CVH API Tests
    // ========================================================================

    #[test]
    fn test_cvh_api_available() {
        let rt = create_test_runtime();
        let cvh: Table = rt.lua().globals().get("cvh").unwrap();
        assert!(cvh.len().unwrap() >= 0, "cvh table should exist");
    }

    #[test]
    fn test_cvh_file_exists_available() {
        let rt = create_test_runtime();
        let cvh: Table = rt.lua().globals().get("cvh").unwrap();
        let file: Table = cvh.get("file").unwrap();
        let exists: Value = file.get("exists").unwrap();
        assert!(matches!(exists, Value::Function(_)), "cvh.file.exists should be a function");
    }

    #[test]
    fn test_cvh_file_exists_works() {
        let rt = create_test_runtime();
        // Test with a path that should exist (the Cargo.toml for example)
        rt.exec("test_result = cvh.file.exists('/tmp')").unwrap();
        let result: bool = rt.lua().globals().get("test_result").unwrap();
        assert!(result, "cvh.file.exists('/tmp') should return true");

        rt.exec("test_result = cvh.file.exists('/nonexistent/path/12345')").unwrap();
        let result: bool = rt.lua().globals().get("test_result").unwrap();
        assert!(!result, "cvh.file.exists for nonexistent path should return false");
    }

    #[test]
    fn test_cvh_file_is_dir_available() {
        let rt = create_test_runtime();
        let cvh: Table = rt.lua().globals().get("cvh").unwrap();
        let file: Table = cvh.get("file").unwrap();
        let is_dir: Value = file.get("is_dir").unwrap();
        assert!(matches!(is_dir, Value::Function(_)), "cvh.file.is_dir should be a function");
    }

    #[test]
    fn test_cvh_file_is_file_available() {
        let rt = create_test_runtime();
        let cvh: Table = rt.lua().globals().get("cvh").unwrap();
        let file: Table = cvh.get("file").unwrap();
        let is_file: Value = file.get("is_file").unwrap();
        assert!(matches!(is_file, Value::Function(_)), "cvh.file.is_file should be a function");
    }

    #[test]
    fn test_cvh_time_now_available() {
        let rt = create_test_runtime();
        let cvh: Table = rt.lua().globals().get("cvh").unwrap();
        let time: Table = cvh.get("time").unwrap();
        let now: Value = time.get("now").unwrap();
        assert!(matches!(now, Value::Function(_)), "cvh.time.now should be a function");
    }

    #[test]
    fn test_cvh_time_now_works() {
        let rt = create_test_runtime();
        rt.exec("test_result = cvh.time.now()").unwrap();
        let result: u64 = rt.lua().globals().get("test_result").unwrap();
        // Should return a reasonable Unix timestamp (after year 2020)
        assert!(result > 1577836800, "cvh.time.now() should return timestamp after 2020");
    }

    #[test]
    fn test_cvh_time_now_ms_available() {
        let rt = create_test_runtime();
        let cvh: Table = rt.lua().globals().get("cvh").unwrap();
        let time: Table = cvh.get("time").unwrap();
        let now_ms: Value = time.get("now_ms").unwrap();
        assert!(matches!(now_ms, Value::Function(_)), "cvh.time.now_ms should be a function");
    }

    #[test]
    fn test_cvh_time_format_available() {
        let rt = create_test_runtime();
        let cvh: Table = rt.lua().globals().get("cvh").unwrap();
        let time: Table = cvh.get("time").unwrap();
        let format: Value = time.get("format").unwrap();
        assert!(matches!(format, Value::Function(_)), "cvh.time.format should be a function");
    }

    #[test]
    fn test_cvh_system_hostname_available() {
        let rt = create_test_runtime();
        let cvh: Table = rt.lua().globals().get("cvh").unwrap();
        let system: Table = cvh.get("system").unwrap();
        let hostname: Value = system.get("hostname").unwrap();
        assert!(matches!(hostname, Value::Function(_)), "cvh.system.hostname should be a function");
    }

    #[test]
    fn test_cvh_system_hostname_works() {
        let rt = create_test_runtime();
        rt.exec("test_result = cvh.system.hostname()").unwrap();
        let result: String = rt.lua().globals().get("test_result").unwrap();
        assert!(!result.is_empty(), "cvh.system.hostname() should return non-empty string");
    }

    #[test]
    fn test_cvh_open_available() {
        let rt = create_test_runtime();
        let cvh: Table = rt.lua().globals().get("cvh").unwrap();
        let open: Value = cvh.get("open").unwrap();
        assert!(matches!(open, Value::Function(_)), "cvh.open should be a function");
    }

    #[test]
    fn test_cvh_spawn_available() {
        let rt = create_test_runtime();
        let cvh: Table = rt.lua().globals().get("cvh").unwrap();
        let spawn: Value = cvh.get("spawn").unwrap();
        assert!(matches!(spawn, Value::Function(_)), "cvh.spawn should be a function");
    }

    #[test]
    fn test_cvh_notify_available() {
        let rt = create_test_runtime();
        let cvh: Table = rt.lua().globals().get("cvh").unwrap();
        let notify: Value = cvh.get("notify").unwrap();
        assert!(matches!(notify, Value::Function(_)), "cvh.notify should be a function");
    }
}