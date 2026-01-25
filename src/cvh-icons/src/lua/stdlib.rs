//! Safe standard library for Lua scripts
//!
//! Provides common utilities without dangerous operations.

use anyhow::Result;
use mlua::{Lua, Table};

/// Install safe standard library extensions
pub fn install(lua: &Lua) -> Result<()> {
    let globals = lua.globals();

    // Safe print function (logs instead of stdout)
    globals.set("print", lua.create_function(|_, args: mlua::Variadic<String>| {
        let msg = args.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\t");
        tracing::info!("[Lua] {}", msg);
        Ok(())
    })?)?;

    // Type checking
    globals.set("type", lua.create_function(|_, value: mlua::Value| {
        Ok(match value {
            mlua::Value::Nil => "nil",
            mlua::Value::Boolean(_) => "boolean",
            mlua::Value::Integer(_) => "number",
            mlua::Value::Number(_) => "number",
            mlua::Value::String(_) => "string",
            mlua::Value::Table(_) => "table",
            mlua::Value::Function(_) => "function",
            mlua::Value::Thread(_) => "thread",
            mlua::Value::UserData(_) => "userdata",
            mlua::Value::LightUserData(_) => "userdata",
            mlua::Value::Error(_) => "error",
            _ => "unknown",
        }.to_string())
    })?)?;

    // Safe tonumber
    globals.set("tonumber", lua.create_function(|_, value: mlua::Value| {
        Ok(match value {
            mlua::Value::Integer(n) => Some(n as f64),
            mlua::Value::Number(n) => Some(n),
            mlua::Value::String(s) => s.to_str().ok().and_then(|s| s.parse::<f64>().ok()),
            _ => None,
        })
    })?)?;

    // Safe tostring
    globals.set("tostring", lua.create_function(|_, value: mlua::Value| {
        Ok(match value {
            mlua::Value::Nil => "nil".to_string(),
            mlua::Value::Boolean(b) => b.to_string(),
            mlua::Value::Integer(n) => n.to_string(),
            mlua::Value::Number(n) => n.to_string(),
            mlua::Value::String(s) => s.to_str().map(|s| s.to_string()).unwrap_or_default(),
            mlua::Value::Table(_) => "table".to_string(),
            mlua::Value::Function(_) => "function".to_string(),
            mlua::Value::Thread(_) => "thread".to_string(),
            mlua::Value::UserData(_) => "userdata".to_string(),
            _ => "unknown".to_string(),
        })
    })?)?;

    // Safe pairs iterator
    globals.set("pairs", lua.create_function(|lua, table: Table| {
        let iter = lua.create_function(|_, (t, k): (Table, mlua::Value)| {
            let next = t.pairs::<mlua::Value, mlua::Value>();
            let mut found_current = k == mlua::Value::Nil;

            for pair in next {
                if let Ok((key, value)) = pair {
                    if found_current {
                        return Ok((Some(key), Some(value)));
                    }
                    if key == k {
                        found_current = true;
                    }
                }
            }

            Ok((None, None))
        })?;

        Ok((iter, table, mlua::Value::Nil))
    })?)?;

    // Safe ipairs iterator
    globals.set("ipairs", lua.create_function(|lua, table: Table| {
        let iter = lua.create_function(|_, (t, i): (Table, i64)| {
            let next_i = i + 1;
            match t.get::<mlua::Value>(next_i) {
                Ok(mlua::Value::Nil) => Ok((None::<i64>, None::<mlua::Value>)),
                Ok(v) => Ok((Some(next_i), Some(v))),
                Err(_) => Ok((None, None)),
            }
        })?;

        Ok((iter, table, 0i64))
    })?)?;

    // Math library (safe subset)
    let math = lua.create_table()?;
    math.set("abs", lua.create_function(|_, n: f64| Ok(n.abs()))?)?;
    math.set("floor", lua.create_function(|_, n: f64| Ok(n.floor()))?)?;
    math.set("ceil", lua.create_function(|_, n: f64| Ok(n.ceil()))?)?;
    math.set("min", lua.create_function(|_, (a, b): (f64, f64)| Ok(a.min(b)))?)?;
    math.set("max", lua.create_function(|_, (a, b): (f64, f64)| Ok(a.max(b)))?)?;
    math.set("sqrt", lua.create_function(|_, n: f64| Ok(n.sqrt()))?)?;
    math.set("sin", lua.create_function(|_, n: f64| Ok(n.sin()))?)?;
    math.set("cos", lua.create_function(|_, n: f64| Ok(n.cos()))?)?;
    math.set("tan", lua.create_function(|_, n: f64| Ok(n.tan()))?)?;
    math.set("pi", std::f64::consts::PI)?;
    math.set("random", lua.create_function(|_, (a, b): (Option<i64>, Option<i64>)| {
        use std::time::{SystemTime, UNIX_EPOCH};
        // Simple pseudo-random (not cryptographic!)
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0) as u64;
        let rand = ((seed.wrapping_mul(1103515245).wrapping_add(12345)) >> 16) as f64 / 32768.0;

        Ok(match (a, b) {
            (None, None) => rand,
            (Some(max), None) => (rand * max as f64).floor() + 1.0,
            (Some(min), Some(max)) => (rand * (max - min + 1) as f64).floor() + min as f64,
            (None, Some(max)) => (rand * max as f64).floor() + 1.0,
        })
    })?)?;
    globals.set("math", math)?;

    // Table library (safe subset)
    let table_lib = lua.create_table()?;
    table_lib.set("insert", lua.create_function(|_, (t, pos, value): (Table, Option<i64>, mlua::Value)| {
        match pos {
            Some(p) => {
                // Shift elements and insert at position
                let len = t.len()? as i64;
                for i in (p..=len).rev() {
                    let v: mlua::Value = t.get(i)?;
                    t.set(i + 1, v)?;
                }
                t.set(p, value)?;
            }
            None => {
                let len = t.len()?;
                t.set(len + 1, value)?;
            }
        }
        Ok(())
    })?)?;
    table_lib.set("remove", lua.create_function(|_, (t, pos): (Table, Option<i64>)| {
        let len = t.len()? as i64;
        let p = pos.unwrap_or(len);
        let removed: mlua::Value = t.get(p)?;

        // Shift elements
        for i in p..len {
            let v: mlua::Value = t.get(i + 1)?;
            t.set(i, v)?;
        }
        t.set(len, mlua::Value::Nil)?;

        Ok(removed)
    })?)?;
    table_lib.set("concat", lua.create_function(|_, (t, sep): (Table, Option<String>)| {
        let sep = sep.unwrap_or_default();
        let mut parts = Vec::new();
        for i in 1..=t.len()? {
            if let Ok(s) = t.get::<String>(i) {
                parts.push(s);
            }
        }
        Ok(parts.join(&sep))
    })?)?;
    globals.set("table", table_lib)?;

    // Assert function
    globals.set("assert", lua.create_function(|_, (cond, msg): (bool, Option<String>)| {
        if cond {
            Ok(())
        } else {
            Err(mlua::Error::runtime(msg.unwrap_or_else(|| "assertion failed".to_string())))
        }
    })?)?;

    // Error function
    globals.set("error", lua.create_function(|_, msg: String| {
        Err::<(), _>(mlua::Error::runtime(msg))
    })?)?;

    Ok(())
}
