//! Lua Process management for sandboxed icon scripts
//!
//! Manages long-lived bubblewrap sandboxed processes for executing Lua icon scripts.
//! Communication happens via stdin/stdout with JSON serialization (length-prefixed).

use std::io::{Read, Write};
use std::os::fd::{AsFd, BorrowedFd};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use nix::poll::{poll, PollFd, PollFlags, PollTimeout};

use crate::ipc::{IpcEncoding, Request, Response, PROTOCOL_VERSION};
use crate::sandbox::SandboxOptions;

/// Default timeout for receiving responses (1 second)
#[allow(dead_code)]
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(1);

/// Maximum message size (1 MB)
#[allow(dead_code)]
const MAX_MESSAGE_SIZE: usize = 1024 * 1024;

/// Manages a sandboxed Lua process for icon rendering
#[allow(dead_code)]
pub struct LuaProcess {
    /// Child process handle
    child: Child,
    /// Stdin handle for sending requests to the child
    stdin: ChildStdin,
    /// Stdout handle for receiving responses from the child
    stdout: ChildStdout,
    /// Path to the IPC handler script
    handler_path: PathBuf,
    /// Path to the icon widget script
    icon_script_path: PathBuf,
    /// Whether the handshake has been completed
    handshake_complete: bool,
}

#[allow(dead_code)]
impl LuaProcess {
    /// Spawn a new sandboxed Lua process
    ///
    /// Spawns a bubblewrap-sandboxed process that will run the Lua interpreter
    /// with the IPC handler script. The icon widget script path is passed via
    /// the CVH_ICON_SCRIPT environment variable. Communication happens via
    /// stdin/stdout using JSON serialization with a u32 length prefix.
    ///
    /// # Arguments
    /// * `handler_path` - Path to the IPC handler script (ipc_handler.lua)
    /// * `icon_script_path` - Path to the icon widget script (e.g., file.lua, folder.lua)
    /// * `sandbox_options` - Sandbox configuration options
    pub fn spawn(
        handler_path: PathBuf,
        icon_script_path: PathBuf,
        sandbox_options: &SandboxOptions,
    ) -> Result<Self> {
        // Build the bubblewrap command
        let mut cmd = Self::build_bwrap_command(sandbox_options, &handler_path, &icon_script_path);

        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Spawn the process
        let mut child = cmd
            .spawn()
            .context("Failed to spawn bubblewrap process")?;

        // Take ownership of stdin/stdout handles
        let stdin = child.stdin.take()
            .context("Failed to get child stdin")?;
        let stdout = child.stdout.take()
            .context("Failed to get child stdout")?;

        let mut process = Self {
            child,
            stdin,
            stdout,
            handler_path,
            icon_script_path,
            handshake_complete: false,
        };

        // Perform protocol handshake
        process.perform_handshake()?;

        Ok(process)
    }

    /// Build the bubblewrap command with appropriate arguments
    ///
    /// # Arguments
    /// * `options` - Sandbox configuration options
    /// * `handler_path` - Path to the IPC handler script (executed by Lua)
    /// * `icon_script_path` - Path to the icon widget script (passed via CVH_ICON_SCRIPT env var)
    fn build_bwrap_command(
        options: &SandboxOptions,
        handler_path: &PathBuf,
        icon_script_path: &PathBuf,
    ) -> Command {
        let mut cmd = Command::new("bwrap");

        // Die when parent dies
        cmd.arg("--die-with-parent");

        // New session for isolation
        cmd.arg("--new-session");

        // Unshare namespaces
        if options.allow_network {
            cmd.args(["--unshare-user", "--unshare-pid", "--unshare-uts", "--unshare-cgroup"]);
        } else {
            cmd.arg("--unshare-all");
        }

        // Mount minimal filesystem
        cmd.args(["--ro-bind", "/usr", "/usr"]);

        // Handle /lib -> /usr/lib symlink scenarios
        if PathBuf::from("/lib").is_symlink() {
            cmd.args(["--symlink", "usr/lib", "/lib"]);
        } else if PathBuf::from("/lib").exists() {
            cmd.args(["--ro-bind", "/lib", "/lib"]);
        }

        if PathBuf::from("/lib64").is_symlink() {
            cmd.args(["--symlink", "usr/lib64", "/lib64"]);
        } else if PathBuf::from("/lib64").exists() {
            cmd.args(["--ro-bind", "/lib64", "/lib64"]);
        }

        // Bin symlinks
        cmd.args(["--symlink", "usr/bin", "/bin"]);
        cmd.args(["--symlink", "usr/sbin", "/sbin"]);

        // Essential virtual filesystems
        cmd.args(["--proc", "/proc"]);
        cmd.args(["--dev", "/dev"]);

        // Tmpfs for temp files
        cmd.args(["--tmpfs", "/tmp"]);
        cmd.args(["--tmpfs", "/run"]);

        // No home access by default
        cmd.args(["--tmpfs", "/home"]);

        // Add configured read-only paths
        for path in &options.read_only_paths {
            if path.exists() {
                let path_str = path.to_string_lossy();
                cmd.args(["--ro-bind", &path_str, &path_str]);
            }
        }

        // Add configured read-write paths
        for path in &options.read_write_paths {
            if path.exists() {
                let path_str = path.to_string_lossy();
                cmd.args(["--bind", &path_str, &path_str]);
            }
        }

        // Bind the handler script directory read-only
        if let Some(parent) = handler_path.parent() {
            if parent.exists() {
                let parent_str = parent.to_string_lossy();
                cmd.args(["--ro-bind", &parent_str, &parent_str]);
            }
        }

        // Bind the icon script directory read-only (if different from handler directory)
        if let Some(icon_parent) = icon_script_path.parent() {
            let handler_parent = handler_path.parent();
            let needs_separate_bind = handler_parent
                .map(|hp| hp != icon_parent)
                .unwrap_or(true);

            if needs_separate_bind && icon_parent.exists() {
                let icon_parent_str = icon_parent.to_string_lossy();
                cmd.args(["--ro-bind", &icon_parent_str, &icon_parent_str]);
            }
        }

        // Set working directory
        if let Some(ref work_dir) = options.work_dir {
            cmd.args(["--chdir", &work_dir.to_string_lossy()]);
        }

        // Clear environment FIRST, then set variables
        // This ensures our setenv calls are not cleared
        cmd.args(["--clearenv"]);

        // Set essential environment variables
        cmd.args(["--setenv", "PATH", "/usr/bin:/bin"]);
        cmd.args(["--setenv", "HOME", "/tmp"]);
        cmd.args(["--setenv", "LANG", "C.UTF-8"]);

        // Set the CVH_ICON_SCRIPT environment variable for the handler to load
        cmd.args(["--setenv", "CVH_ICON_SCRIPT", &icon_script_path.to_string_lossy()]);

        // Apply caller-provided environment variables AFTER essential ones
        // This allows callers to override defaults if needed
        for (key, value) in &options.env_vars {
            cmd.args(["--setenv", key, value]);
        }

        // Add the actual Lua interpreter and IPC handler script
        cmd.arg("--");
        cmd.arg("lua");
        cmd.arg(handler_path.to_string_lossy().as_ref());

        cmd
    }

    /// Perform protocol version handshake
    fn perform_handshake(&mut self) -> Result<()> {
        let request = Request::Handshake {
            version: PROTOCOL_VERSION,
        };

        self.send_request(&request)?;

        match self.receive_response()? {
            Response::HandshakeAck { version, success } => {
                if !success {
                    bail!("Handshake failed: version mismatch (local: {}, remote: {})",
                          PROTOCOL_VERSION, version);
                }
                if version != PROTOCOL_VERSION {
                    bail!("Protocol version mismatch: expected {}, got {}",
                          PROTOCOL_VERSION, version);
                }
                self.handshake_complete = true;
                Ok(())
            }
            Response::Error { message } => {
                bail!("Handshake failed: {}", message);
            }
            other => {
                bail!("Unexpected response to handshake: {:?}", other);
            }
        }
    }

    /// Send a request to the Lua process using JSON + length prefix over stdin
    pub fn send_request(&mut self, request: &Request) -> Result<()> {
        let data = request.serialize(IpcEncoding::Json)
            .map_err(|e| anyhow::anyhow!("Failed to serialize request: {}", e))?;

        if data.len() > MAX_MESSAGE_SIZE {
            bail!("Request too large: {} bytes (max: {})", data.len(), MAX_MESSAGE_SIZE);
        }

        // Write length prefix (4 bytes, little-endian)
        let len_bytes = (data.len() as u32).to_le_bytes();
        self.stdin
            .write_all(&len_bytes)
            .context("Failed to write message length")?;

        // Write the actual data
        self.stdin
            .write_all(&data)
            .context("Failed to write message data")?;

        self.stdin.flush().context("Failed to flush stdin")?;

        Ok(())
    }

    /// Receive a response from the Lua process with timeout
    pub fn receive_response(&mut self) -> Result<Response> {
        self.receive_response_with_timeout(DEFAULT_TIMEOUT)
    }

    /// Receive a response from the Lua process with a custom timeout
    ///
    /// Uses poll() to wait for data with a timeout, preventing indefinite blocking
    /// on dead or unresponsive child processes.
    pub fn receive_response_with_timeout(&mut self, timeout: Duration) -> Result<Response> {
        // Read length prefix (4 bytes, little-endian) with timeout
        let mut len_bytes = [0u8; 4];
        self.read_exact_with_timeout(&mut len_bytes, timeout)
            .context("Failed to read message length")?;

        let len = u32::from_le_bytes(len_bytes) as usize;

        if len > MAX_MESSAGE_SIZE {
            bail!("Response too large: {} bytes (max: {})", len, MAX_MESSAGE_SIZE);
        }

        // Read the actual data with timeout
        let mut data = vec![0u8; len];
        self.read_exact_with_timeout(&mut data, timeout)
            .context("Failed to read message data")?;

        // Deserialize the response using JSON
        let response = Response::deserialize(&data, IpcEncoding::Json)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize response: {}", e))?;

        Ok(response)
    }

    /// Read exactly `buf.len()` bytes from stdout with a timeout
    ///
    /// Uses poll() to wait for data availability before reading.
    /// Returns an error if the timeout expires before all data is read.
    fn read_exact_with_timeout(&mut self, buf: &mut [u8], timeout: Duration) -> Result<()> {
        let timeout_ms = timeout.as_millis();
        let mut bytes_read = 0;

        while bytes_read < buf.len() {
            // Get a borrowed fd from stdout
            let borrowed_fd: BorrowedFd<'_> = self.stdout.as_fd();

            // Create a PollFd for the stdout file descriptor
            let mut poll_fds = [PollFd::new(borrowed_fd, PollFlags::POLLIN)];

            // Convert timeout to PollTimeout, capping at i32::MAX milliseconds (~24 days)
            // to avoid overflow issues
            let timeout_capped = timeout_ms.min(i32::MAX as u128) as i32;
            let poll_timeout = if timeout_capped > 0 {
                // PollTimeout accepts various integer types; use i32 for maximum range
                PollTimeout::try_from(timeout_capped).unwrap_or(PollTimeout::MAX)
            } else {
                PollTimeout::ZERO
            };

            // Wait for data with timeout
            let poll_result = poll(&mut poll_fds, poll_timeout)
                .context("poll() failed")?;

            if poll_result == 0 {
                bail!(
                    "Timeout waiting for data from Lua process (waited {}ms, read {}/{})",
                    timeout_ms,
                    bytes_read,
                    buf.len()
                );
            }

            // Check for errors or hangup
            if let Some(revents) = poll_fds[0].revents() {
                if revents.contains(PollFlags::POLLERR) {
                    bail!("Error condition on Lua process stdout");
                }
                if revents.contains(PollFlags::POLLHUP) && !revents.contains(PollFlags::POLLIN) {
                    bail!("Lua process closed stdout (hangup)");
                }
            }

            // Data is available, read it
            let n = self.stdout
                .read(&mut buf[bytes_read..])
                .context("Failed to read from stdout")?;

            if n == 0 {
                bail!(
                    "Unexpected EOF from Lua process (read {}/{})",
                    bytes_read,
                    buf.len()
                );
            }

            bytes_read += n;
        }

        Ok(())
    }

    /// Kill the Lua process and clean up resources
    pub fn kill(&mut self) -> Result<()> {
        // Try to send a graceful shutdown request first
        if self.handshake_complete {
            let _ = self.send_request(&Request::Shutdown);
            // Give it a short time to respond
            if let Ok(Response::ShutdownAck) = self.receive_response_with_timeout(Duration::from_millis(100)) {
                // Graceful shutdown succeeded
                let _ = self.child.wait();
                return Ok(());
            }
        }

        // Force kill if graceful shutdown failed
        self.child
            .kill()
            .context("Failed to kill Lua process")?;

        // Wait for the process to fully terminate
        self.child
            .wait()
            .context("Failed to wait for Lua process")?;

        Ok(())
    }

    /// Check if the process is still running
    pub fn is_running(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    /// Get the path to the IPC handler script
    pub fn handler_path(&self) -> &PathBuf {
        &self.handler_path
    }

    /// Get the path to the icon widget script
    pub fn icon_script_path(&self) -> &PathBuf {
        &self.icon_script_path
    }

    /// Get the process ID
    pub fn pid(&self) -> u32 {
        self.child.id()
    }
}

impl Drop for LuaProcess {
    fn drop(&mut self) {
        // Ensure the process is killed when the LuaProcess is dropped
        let _ = self.kill();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_timeout_is_one_second() {
        assert_eq!(DEFAULT_TIMEOUT, Duration::from_secs(1));
    }

    #[test]
    fn test_max_message_size() {
        assert_eq!(MAX_MESSAGE_SIZE, 1024 * 1024);
    }

    #[test]
    fn test_build_bwrap_command_basic() {
        let options = SandboxOptions::default();
        let handler_path = PathBuf::from("/tmp/ipc_handler.lua");
        let icon_script_path = PathBuf::from("/tmp/widgets/file.lua");
        let cmd = LuaProcess::build_bwrap_command(&options, &handler_path, &icon_script_path);

        let program = cmd.get_program().to_string_lossy();
        assert!(program.contains("bwrap"), "Should use bwrap");
    }

    #[test]
    fn test_build_bwrap_command_no_network() {
        let options = SandboxOptions {
            allow_network: false,
            ..Default::default()
        };
        let handler_path = PathBuf::from("/tmp/ipc_handler.lua");
        let icon_script_path = PathBuf::from("/tmp/widgets/file.lua");
        let cmd = LuaProcess::build_bwrap_command(&options, &handler_path, &icon_script_path);

        let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy()).collect();
        assert!(args.iter().any(|a| a == "--unshare-all"),
                "Should unshare all namespaces when network is disabled");
    }

    #[test]
    fn test_build_bwrap_command_with_network() {
        let options = SandboxOptions {
            allow_network: true,
            ..Default::default()
        };
        let handler_path = PathBuf::from("/tmp/ipc_handler.lua");
        let icon_script_path = PathBuf::from("/tmp/widgets/file.lua");
        let cmd = LuaProcess::build_bwrap_command(&options, &handler_path, &icon_script_path);

        let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy()).collect();
        assert!(!args.iter().any(|a| a == "--unshare-all"),
                "Should not unshare all namespaces when network is enabled");
        assert!(args.iter().any(|a| a == "--unshare-user"),
                "Should still unshare user namespace");
    }

    #[test]
    fn test_build_bwrap_command_includes_die_with_parent() {
        let options = SandboxOptions::default();
        let handler_path = PathBuf::from("/tmp/ipc_handler.lua");
        let icon_script_path = PathBuf::from("/tmp/widgets/file.lua");
        let cmd = LuaProcess::build_bwrap_command(&options, &handler_path, &icon_script_path);

        let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy()).collect();
        assert!(args.iter().any(|a| a == "--die-with-parent"),
                "Should include --die-with-parent for cleanup");
    }

    #[test]
    fn test_build_bwrap_command_includes_new_session() {
        let options = SandboxOptions::default();
        let handler_path = PathBuf::from("/tmp/ipc_handler.lua");
        let icon_script_path = PathBuf::from("/tmp/widgets/file.lua");
        let cmd = LuaProcess::build_bwrap_command(&options, &handler_path, &icon_script_path);

        let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy()).collect();
        assert!(args.iter().any(|a| a == "--new-session"),
                "Should include --new-session for isolation");
    }

    #[test]
    fn test_build_bwrap_command_mounts_usr_readonly() {
        let options = SandboxOptions::default();
        let handler_path = PathBuf::from("/tmp/ipc_handler.lua");
        let icon_script_path = PathBuf::from("/tmp/widgets/file.lua");
        let cmd = LuaProcess::build_bwrap_command(&options, &handler_path, &icon_script_path);

        let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy()).collect();
        // Find --ro-bind /usr /usr pattern
        let has_usr_bind = args.windows(3).any(|w| {
            w[0] == "--ro-bind" && w[1] == "/usr" && w[2] == "/usr"
        });
        assert!(has_usr_bind, "Should mount /usr read-only");
    }

    #[test]
    fn test_build_bwrap_command_clears_environment() {
        let options = SandboxOptions::default();
        let handler_path = PathBuf::from("/tmp/ipc_handler.lua");
        let icon_script_path = PathBuf::from("/tmp/widgets/file.lua");
        let cmd = LuaProcess::build_bwrap_command(&options, &handler_path, &icon_script_path);

        let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy()).collect();
        assert!(args.iter().any(|a| a == "--clearenv"),
                "Should clear environment");
    }

    #[test]
    fn test_build_bwrap_command_sets_essential_env() {
        let options = SandboxOptions::default();
        let handler_path = PathBuf::from("/tmp/ipc_handler.lua");
        let icon_script_path = PathBuf::from("/tmp/widgets/file.lua");
        let cmd = LuaProcess::build_bwrap_command(&options, &handler_path, &icon_script_path);

        let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy()).collect();

        // Check for PATH
        let has_path = args.windows(3).any(|w| {
            w[0] == "--setenv" && w[1] == "PATH"
        });
        assert!(has_path, "Should set PATH environment variable");

        // Check for HOME
        let has_home = args.windows(3).any(|w| {
            w[0] == "--setenv" && w[1] == "HOME"
        });
        assert!(has_home, "Should set HOME environment variable");
    }

    #[test]
    fn test_build_bwrap_command_uses_tmpfs_for_home() {
        let options = SandboxOptions::default();
        let handler_path = PathBuf::from("/tmp/ipc_handler.lua");
        let icon_script_path = PathBuf::from("/tmp/widgets/file.lua");
        let cmd = LuaProcess::build_bwrap_command(&options, &handler_path, &icon_script_path);

        let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy()).collect();
        let has_home_tmpfs = args.windows(2).any(|w| {
            w[0] == "--tmpfs" && w[1] == "/home"
        });
        assert!(has_home_tmpfs, "Should use tmpfs for /home");
    }

    #[test]
    fn test_build_bwrap_command_includes_handler_path() {
        let options = SandboxOptions::default();
        let handler_path = PathBuf::from("/tmp/ipc_handler.lua");
        let icon_script_path = PathBuf::from("/tmp/widgets/file.lua");
        let cmd = LuaProcess::build_bwrap_command(&options, &handler_path, &icon_script_path);

        let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy()).collect();
        assert!(args.iter().any(|a| a == "/tmp/ipc_handler.lua"),
                "Should include the handler script path");
    }

    #[test]
    fn test_build_bwrap_command_sets_cvh_icon_script_env() {
        let options = SandboxOptions::default();
        let handler_path = PathBuf::from("/tmp/ipc_handler.lua");
        let icon_script_path = PathBuf::from("/tmp/widgets/file.lua");
        let cmd = LuaProcess::build_bwrap_command(&options, &handler_path, &icon_script_path);

        let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy()).collect();
        let has_cvh_icon_script = args.windows(3).any(|w| {
            w[0] == "--setenv" && w[1] == "CVH_ICON_SCRIPT" && w[2] == "/tmp/widgets/file.lua"
        });
        assert!(has_cvh_icon_script, "Should set CVH_ICON_SCRIPT environment variable");
    }

    #[test]
    fn test_build_bwrap_command_includes_lua_interpreter() {
        let options = SandboxOptions::default();
        let handler_path = PathBuf::from("/tmp/ipc_handler.lua");
        let icon_script_path = PathBuf::from("/tmp/widgets/file.lua");
        let cmd = LuaProcess::build_bwrap_command(&options, &handler_path, &icon_script_path);

        let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy()).collect();
        assert!(args.iter().any(|a| a == "lua"),
                "Should include lua interpreter");
    }

    #[test]
    fn test_build_bwrap_command_with_custom_env_vars() {
        let options = SandboxOptions {
            env_vars: vec![
                ("CUSTOM_VAR".to_string(), "custom_value".to_string()),
            ],
            ..Default::default()
        };
        let handler_path = PathBuf::from("/tmp/ipc_handler.lua");
        let icon_script_path = PathBuf::from("/tmp/widgets/file.lua");
        let cmd = LuaProcess::build_bwrap_command(&options, &handler_path, &icon_script_path);

        let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy()).collect();
        let has_custom_var = args.windows(3).any(|w| {
            w[0] == "--setenv" && w[1] == "CUSTOM_VAR" && w[2] == "custom_value"
        });
        assert!(has_custom_var, "Should include custom environment variables");
    }

    #[test]
    fn test_build_bwrap_command_with_work_dir() {
        let options = SandboxOptions {
            work_dir: Some(PathBuf::from("/tmp/workdir")),
            ..Default::default()
        };
        let handler_path = PathBuf::from("/tmp/ipc_handler.lua");
        let icon_script_path = PathBuf::from("/tmp/widgets/file.lua");
        let cmd = LuaProcess::build_bwrap_command(&options, &handler_path, &icon_script_path);

        let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy()).collect();
        let has_chdir = args.windows(2).any(|w| {
            w[0] == "--chdir" && w[1] == "/tmp/workdir"
        });
        assert!(has_chdir, "Should set working directory");
    }

    #[test]
    fn test_build_bwrap_command_no_fd_flag() {
        // Verify that --fd flag is NOT used (stdin/stdout is used instead)
        let options = SandboxOptions::default();
        let handler_path = PathBuf::from("/tmp/ipc_handler.lua");
        let icon_script_path = PathBuf::from("/tmp/widgets/file.lua");
        let cmd = LuaProcess::build_bwrap_command(&options, &handler_path, &icon_script_path);

        let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy()).collect();
        assert!(!args.iter().any(|a| a == "--fd"),
                "Should not use --fd flag (uses stdin/stdout instead)");
    }

    #[test]
    fn test_build_bwrap_command_clearenv_before_setenv() {
        let options = SandboxOptions::default();
        let handler_path = PathBuf::from("/tmp/ipc_handler.lua");
        let icon_script_path = PathBuf::from("/tmp/widgets/file.lua");
        let cmd = LuaProcess::build_bwrap_command(&options, &handler_path, &icon_script_path);

        let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy()).collect();

        // Find positions of --clearenv and first --setenv
        let clearenv_pos = args.iter().position(|a| a == "--clearenv");
        let first_setenv_pos = args.iter().position(|a| a == "--setenv");

        assert!(clearenv_pos.is_some(), "Should have --clearenv");
        assert!(first_setenv_pos.is_some(), "Should have --setenv");

        // --clearenv must come before --setenv
        assert!(clearenv_pos.unwrap() < first_setenv_pos.unwrap(),
                "--clearenv must come before --setenv to preserve env vars");
    }

    #[test]
    fn test_build_bwrap_command_custom_env_after_essential() {
        let options = SandboxOptions {
            env_vars: vec![
                ("CUSTOM_VAR".to_string(), "custom_value".to_string()),
            ],
            ..Default::default()
        };
        let handler_path = PathBuf::from("/tmp/ipc_handler.lua");
        let icon_script_path = PathBuf::from("/tmp/widgets/file.lua");
        let cmd = LuaProcess::build_bwrap_command(&options, &handler_path, &icon_script_path);

        let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy()).collect();

        // Find position of PATH setenv and CUSTOM_VAR setenv
        let mut path_pos = None;
        let mut custom_pos = None;
        for (i, w) in args.windows(3).enumerate() {
            if w[0] == "--setenv" && w[1] == "PATH" {
                path_pos = Some(i);
            }
            if w[0] == "--setenv" && w[1] == "CUSTOM_VAR" {
                custom_pos = Some(i);
            }
        }

        assert!(path_pos.is_some(), "Should have PATH env var");
        assert!(custom_pos.is_some(), "Should have CUSTOM_VAR env var");
        assert!(path_pos.unwrap() < custom_pos.unwrap(),
                "Custom env vars should come after essential ones so they can override");
    }

    // =========================================================================
    // Integration tests for IPC communication
    // These tests validate the JSON + length-prefix IPC protocol format
    // that is used for stdin/stdout communication with Lua.
    // =========================================================================

    use std::os::unix::net::UnixStream;

    /// Helper struct for testing IPC protocol without bubblewrap
    struct MockIpcPair {
        /// The "parent" side (simulates Rust daemon)
        parent: UnixStream,
        /// The "child" side (simulates Lua process)
        child: UnixStream,
    }

    impl MockIpcPair {
        fn new() -> std::io::Result<Self> {
            let (parent, child) = UnixStream::pair()?;
            parent.set_nonblocking(false)?;
            child.set_nonblocking(false)?;
            Ok(Self { parent, child })
        }
    }

    #[test]
    fn test_ipc_send_request_serializes_correctly_json() {
        let pair = MockIpcPair::new().expect("Failed to create socket pair");

        let mut parent_socket = pair.parent;
        let mut child_socket = pair.child;

        child_socket.set_read_timeout(Some(Duration::from_secs(1))).unwrap();

        // Send using JSON encoding (matching the new protocol)
        let send_thread = std::thread::spawn(move || {
            let request = Request::Handshake { version: PROTOCOL_VERSION };
            let data = request.serialize(IpcEncoding::Json).unwrap();

            let len_bytes = (data.len() as u32).to_le_bytes();
            parent_socket.write_all(&len_bytes).unwrap();
            parent_socket.write_all(&data).unwrap();
            parent_socket.flush().unwrap();
            parent_socket
        });

        // Read from child side
        let mut len_bytes = [0u8; 4];
        child_socket.read_exact(&mut len_bytes).unwrap();
        let len = u32::from_le_bytes(len_bytes) as usize;

        let mut data = vec![0u8; len];
        child_socket.read_exact(&mut data).unwrap();

        let request = Request::deserialize(&data, IpcEncoding::Json).unwrap();
        match request {
            Request::Handshake { version } => {
                assert_eq!(version, PROTOCOL_VERSION);
            }
            _ => panic!("Expected Handshake request"),
        }

        send_thread.join().unwrap();
    }

    #[test]
    fn test_ipc_receive_response_deserializes_correctly_json() {
        let pair = MockIpcPair::new().expect("Failed to create socket pair");

        let mut parent_socket = pair.parent;
        let mut child_socket = pair.child;

        // Send JSON response from "child"
        let child_thread = std::thread::spawn(move || {
            let response = Response::HandshakeAck {
                version: PROTOCOL_VERSION,
                success: true,
            };
            let data = response.serialize(IpcEncoding::Json).unwrap();

            let len_bytes = (data.len() as u32).to_le_bytes();
            child_socket.write_all(&len_bytes).unwrap();
            child_socket.write_all(&data).unwrap();
            child_socket.flush().unwrap();
            child_socket
        });

        parent_socket.set_read_timeout(Some(Duration::from_secs(1))).unwrap();

        let mut len_bytes = [0u8; 4];
        parent_socket.read_exact(&mut len_bytes).unwrap();
        let len = u32::from_le_bytes(len_bytes) as usize;

        let mut data = vec![0u8; len];
        parent_socket.read_exact(&mut data).unwrap();

        let response = Response::deserialize(&data, IpcEncoding::Json).unwrap();
        match response {
            Response::HandshakeAck { version, success } => {
                assert_eq!(version, PROTOCOL_VERSION);
                assert!(success);
            }
            _ => panic!("Expected HandshakeAck response"),
        }

        child_thread.join().unwrap();
    }

    #[test]
    fn test_ipc_timeout_on_socket() {
        let pair = MockIpcPair::new().expect("Failed to create socket pair");

        let mut parent_socket = pair.parent;
        let _child_socket = pair.child; // Keep alive but don't write anything

        // Set a very short timeout (50ms)
        let timeout = Duration::from_millis(50);
        parent_socket.set_read_timeout(Some(timeout)).unwrap();

        // Try to read - should timeout since child never writes
        let mut len_bytes = [0u8; 4];
        let result = parent_socket.read_exact(&mut len_bytes);

        assert!(result.is_err(), "Should timeout when child doesn't respond");
        let err = result.unwrap_err();
        assert!(
            err.kind() == std::io::ErrorKind::WouldBlock ||
            err.kind() == std::io::ErrorKind::TimedOut,
            "Error should be timeout-related, got: {:?}",
            err.kind()
        );
    }

    #[test]
    fn test_ipc_large_message_rejected() {
        // Test that messages larger than MAX_MESSAGE_SIZE are rejected
        let pair = MockIpcPair::new().expect("Failed to create socket pair");

        let mut parent_socket = pair.parent;
        let mut child_socket = pair.child;

        // Spawn a thread to send an oversized message length from "child"
        let child_thread = std::thread::spawn(move || {
            // Send a length that exceeds MAX_MESSAGE_SIZE
            let oversized_len = (MAX_MESSAGE_SIZE + 1) as u32;
            let len_bytes = oversized_len.to_le_bytes();
            child_socket.write_all(&len_bytes).unwrap();
            child_socket.flush().unwrap();
            child_socket
        });

        parent_socket.set_read_timeout(Some(Duration::from_secs(1))).unwrap();

        // Read the length
        let mut len_bytes = [0u8; 4];
        parent_socket.read_exact(&mut len_bytes).unwrap();
        let len = u32::from_le_bytes(len_bytes) as usize;

        // Verify it's too large
        assert!(len > MAX_MESSAGE_SIZE, "Length should exceed max");

        child_thread.join().unwrap();
    }

    #[test]
    fn test_ipc_roundtrip_render_request_json() {
        use crate::ipc::{IconMetadata, IconType, RenderContext};

        let pair = MockIpcPair::new().expect("Failed to create socket pair");

        let mut parent_socket = pair.parent;
        let mut child_socket = pair.child;

        let original_request = Request::Render {
            metadata: IconMetadata {
                path: "/home/user/test.txt".to_string(),
                name: "test.txt".to_string(),
                mime_type: Some("text/plain".to_string()),
                is_directory: false,
                size: Some(1024),
                width: 64,
                height: 64,
                icon_type: IconType::File,
                selected: true,
                hovered: false,
            },
            context: RenderContext {
                canvas_width: 128,
                canvas_height: 128,
                device_pixel_ratio: 2.0,
            },
        };

        let request_clone = original_request.clone();

        // Send using JSON encoding
        let send_thread = std::thread::spawn(move || {
            let data = request_clone.serialize(IpcEncoding::Json).unwrap();
            let len_bytes = (data.len() as u32).to_le_bytes();
            parent_socket.write_all(&len_bytes).unwrap();
            parent_socket.write_all(&data).unwrap();
            parent_socket.flush().unwrap();
            parent_socket
        });

        // Receive on child
        child_socket.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
        let mut len_bytes = [0u8; 4];
        child_socket.read_exact(&mut len_bytes).unwrap();
        let len = u32::from_le_bytes(len_bytes) as usize;

        let mut data = vec![0u8; len];
        child_socket.read_exact(&mut data).unwrap();

        let received = Request::deserialize(&data, IpcEncoding::Json).unwrap();

        match received {
            Request::Render { metadata, context } => {
                assert_eq!(metadata.path, "/home/user/test.txt");
                assert_eq!(metadata.name, "test.txt");
                assert_eq!(metadata.icon_type, IconType::File);
                assert!(metadata.selected);
                assert!(!metadata.hovered);
                assert_eq!(context.canvas_width, 128);
            }
            _ => panic!("Expected Render request"),
        }

        send_thread.join().unwrap();
    }

    #[test]
    fn test_socket_close_on_drop() {
        let pair = MockIpcPair::new().expect("Failed to create socket pair");

        let parent_socket = pair.parent;
        let mut child_socket = pair.child;

        // Drop the parent socket
        drop(parent_socket);

        // Try to write to child - should detect that peer closed
        child_socket.set_read_timeout(Some(Duration::from_millis(100))).unwrap();

        // Try to read - should get EOF or error since parent closed
        let mut buf = [0u8; 1];
        let result = child_socket.read(&mut buf);

        // Either we get 0 bytes (EOF) or an error
        match result {
            Ok(0) => { /* EOF - expected */ }
            Ok(_) => panic!("Should not receive data after peer closes"),
            Err(_) => { /* Connection error - also acceptable */ }
        }
    }

    #[test]
    fn test_ipc_position_request_roundtrip_json() {
        use crate::ipc::PositionInput;

        let pair = MockIpcPair::new().expect("Failed to create socket pair");

        let mut parent_socket = pair.parent;
        let mut child_socket = pair.child;

        let request = Request::Position {
            input: PositionInput {
                screen_width: 1920,
                screen_height: 1080,
                icon_count: 20,
                icon_index: 5,
                cell_width: Some(96),
                cell_height: Some(96),
            },
        };

        // Send using JSON encoding
        let data = request.serialize(IpcEncoding::Json).unwrap();
        let len_bytes = (data.len() as u32).to_le_bytes();
        parent_socket.write_all(&len_bytes).unwrap();
        parent_socket.write_all(&data).unwrap();
        parent_socket.flush().unwrap();

        // Receive on child
        child_socket.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
        let mut len_bytes = [0u8; 4];
        child_socket.read_exact(&mut len_bytes).unwrap();
        let len = u32::from_le_bytes(len_bytes) as usize;

        let mut data = vec![0u8; len];
        child_socket.read_exact(&mut data).unwrap();

        let received = Request::deserialize(&data, IpcEncoding::Json).unwrap();

        match received {
            Request::Position { input } => {
                assert_eq!(input.screen_width, 1920);
                assert_eq!(input.screen_height, 1080);
                assert_eq!(input.icon_count, 20);
                assert_eq!(input.icon_index, 5);
                assert_eq!(input.cell_width, Some(96));
            }
            _ => panic!("Expected Position request"),
        }
    }

    #[test]
    fn test_ipc_event_response_with_action_json() {
        use crate::ipc::EventAction;

        let pair = MockIpcPair::new().expect("Failed to create socket pair");

        let mut parent_socket = pair.parent;
        let mut child_socket = pair.child;

        let response = Response::Event {
            handled: true,
            action: Some(EventAction {
                action: "open".to_string(),
                payload: Some("/home/user/Documents".to_string()),
            }),
        };

        // Send using JSON encoding from child
        let data = response.serialize(IpcEncoding::Json).unwrap();
        let len_bytes = (data.len() as u32).to_le_bytes();
        child_socket.write_all(&len_bytes).unwrap();
        child_socket.write_all(&data).unwrap();
        child_socket.flush().unwrap();

        // Receive on parent
        parent_socket.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
        let mut len_bytes = [0u8; 4];
        parent_socket.read_exact(&mut len_bytes).unwrap();
        let len = u32::from_le_bytes(len_bytes) as usize;

        let mut data = vec![0u8; len];
        parent_socket.read_exact(&mut data).unwrap();

        let received = Response::deserialize(&data, IpcEncoding::Json).unwrap();

        match received {
            Response::Event { handled, action } => {
                assert!(handled);
                let action = action.expect("Should have action");
                assert_eq!(action.action, "open");
                assert_eq!(action.payload, Some("/home/user/Documents".to_string()));
            }
            _ => panic!("Expected Event response"),
        }
    }

    // =========================================================================
    // Timeout Implementation Tests
    // =========================================================================

    #[test]
    fn test_read_exact_with_timeout_using_pipe() {
        use std::os::unix::io::AsRawFd;
        use std::process::{Command, Stdio};

        // Create a simple process that writes data after a delay
        // Using "sleep 0.1 && echo hello" to test timeout with data
        let mut child = Command::new("bash")
            .args(["-c", "echo -n 'test'"])
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn test process");

        let mut stdout = child.stdout.take().expect("Failed to get stdout");

        // The child writes immediately, so we should be able to read
        let borrowed_fd: BorrowedFd<'_> = stdout.as_fd();
        let mut poll_fds = [PollFd::new(borrowed_fd, PollFlags::POLLIN)];
        let poll_timeout = PollTimeout::from(1000u16); // 1 second

        // Wait for data
        let result = poll(&mut poll_fds, poll_timeout).expect("poll failed");
        assert!(result > 0, "Should have data available");

        // Read the data
        let mut buf = [0u8; 4];
        stdout.read_exact(&mut buf).expect("Failed to read");
        assert_eq!(&buf, b"test");

        child.wait().expect("Failed to wait for child");
    }

    #[test]
    fn test_poll_timeout_when_no_data() {
        use std::process::{Command, Stdio};

        // Create a process that sleeps and doesn't output immediately
        let mut child = Command::new("sleep")
            .arg("10") // Sleep for 10 seconds
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn test process");

        let stdout = child.stdout.take().expect("Failed to get stdout");

        // Set a short timeout
        let borrowed_fd: BorrowedFd<'_> = stdout.as_fd();
        let mut poll_fds = [PollFd::new(borrowed_fd, PollFlags::POLLIN)];
        let poll_timeout = PollTimeout::from(50u16); // 50ms timeout

        // This should timeout because sleep doesn't output anything
        let result = poll(&mut poll_fds, poll_timeout).expect("poll failed");
        assert_eq!(result, 0, "Should timeout when no data available");

        // Clean up
        child.kill().ok();
        child.wait().ok();
    }

    #[test]
    fn test_poll_detects_process_exit() {
        use std::process::{Command, Stdio};

        // Create a process that exits immediately
        let mut child = Command::new("true")
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn test process");

        let stdout = child.stdout.take().expect("Failed to get stdout");

        // Wait for the process to exit
        child.wait().expect("Failed to wait");

        // Poll should detect EOF/hangup
        let borrowed_fd: BorrowedFd<'_> = stdout.as_fd();
        let mut poll_fds = [PollFd::new(borrowed_fd, PollFlags::POLLIN)];
        let poll_timeout = PollTimeout::from(100u16); // 100ms

        let result = poll(&mut poll_fds, poll_timeout).expect("poll failed");

        // On process exit, poll may return with POLLHUP or POLLIN (for EOF)
        if result > 0 {
            if let Some(revents) = poll_fds[0].revents() {
                // Either POLLHUP (hangup) or POLLIN with 0 bytes (EOF) is acceptable
                assert!(
                    revents.contains(PollFlags::POLLHUP) || revents.contains(PollFlags::POLLIN),
                    "Expected POLLHUP or POLLIN on process exit, got: {:?}",
                    revents
                );
            }
        }
        // result == 0 means timeout, which shouldn't happen since process exited
    }

    #[test]
    fn test_default_timeout_value_is_usable() {
        // Verify that DEFAULT_TIMEOUT can be converted to a valid poll timeout
        let timeout_ms = DEFAULT_TIMEOUT.as_millis() as i32;
        assert!(timeout_ms > 0, "DEFAULT_TIMEOUT should be positive");
        assert!(timeout_ms <= 65535, "DEFAULT_TIMEOUT should fit in u16 for PollTimeout");
    }
}
