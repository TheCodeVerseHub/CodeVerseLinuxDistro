//! IPC module for Lua-Rust communication
//!
//! Provides protocol definitions and message types for inter-process
//! communication between the main Rust daemon and sandboxed Lua processes.

mod protocol;

pub use protocol::*;
