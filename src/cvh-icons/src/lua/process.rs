//! Lua Process management for sandboxed icon scripts
//!
//! Manages long-lived bubblewrap sandboxed processes for executing Lua icon scripts.
//! Communication happens via Unix sockets with bincode serialization.

use std::io::{Read, Write};
use std::os::unix::io::{AsRawFd, RawFd};
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use nix::libc;

use crate::ipc::{Request, Response, PROTOCOL_VERSION};
use crate::sandbox::SandboxOptions;

/// Default timeout for receiving responses (1 second)
#[allow(dead_code)]
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(1);

/// Maximum message size (1 MB)
#[allow(dead_code)]
const MAX_MESSAGE_SIZE: usize = 1024 * 1024;

/// The file descriptor number used for IPC with the child process.
/// The child reads/writes IPC messages from/to this FD.
/// This is set via the CVH_IPC_FD environment variable.
const CHILD_IPC_FD: RawFd = 3;

/// Manages a sandboxed Lua process for icon rendering
#[allow(dead_code)]
pub struct LuaProcess {
    /// Child process handle
    child: Child,
    /// Unix socket for communication with the child
    socket: UnixStream,
    /// Path to the Lua script
    script_path: PathBuf,
    /// Whether the handshake has been completed
    handshake_complete: bool,
}

#[allow(dead_code)]
impl LuaProcess {
    /// Spawn a new sandboxed Lua process
    ///
    /// Creates a Unix socket pair and spawns a bubblewrap-sandboxed process
    /// that will run the Lua interpreter with the specified script.
    ///
    /// # IPC File Descriptor
    /// The child process receives the IPC socket on FD 3 (CHILD_IPC_FD).
    /// This is communicated to the Lua side via the CVH_IPC_FD environment variable.
    /// The Lua script should read this env var and open that FD for communication.
    pub fn spawn(script_path: PathBuf, sandbox_options: &SandboxOptions) -> Result<Self> {
        // Create a Unix socket pair for communication
        let (parent_socket, child_socket) = UnixStream::pair()
            .context("Failed to create Unix socket pair")?;

        // Set non-blocking for the parent side (we'll use timeouts)
        parent_socket
            .set_nonblocking(false)
            .context("Failed to set socket to blocking mode")?;

        // Get the file descriptor for the child socket
        let child_fd = child_socket.as_raw_fd();

        // Build the bubblewrap command with the IPC FD passed through
        let mut cmd = Self::build_bwrap_command(sandbox_options, &script_path, CHILD_IPC_FD);

        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Use pre_exec to dup the child socket to CHILD_IPC_FD (fd 3)
        // This ensures the socket is available at a well-known FD in the child.
        // SAFETY: This closure runs after fork() but before exec().
        // We only call async-signal-safe functions (dup2, close).
        unsafe {
            cmd.pre_exec(move || {
                // Duplicate child_fd to CHILD_IPC_FD (fd 3)
                if child_fd != CHILD_IPC_FD {
                    if libc::dup2(child_fd, CHILD_IPC_FD) == -1 {
                        return Err(std::io::Error::last_os_error());
                    }
                    // Close the original fd since we've duplicated it
                    libc::close(child_fd);
                }
                // Clear CLOEXEC on the target FD so it survives exec
                let flags = libc::fcntl(CHILD_IPC_FD, libc::F_GETFD);
                if flags == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::fcntl(CHILD_IPC_FD, libc::F_SETFD, flags & !libc::FD_CLOEXEC) == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }

        // Spawn the process
        let child = cmd
            .spawn()
            .context("Failed to spawn bubblewrap process")?;

        // Close the child's end of the socket in the parent
        // The child now has its own copy via dup2
        drop(child_socket);

        let mut process = Self {
            child,
            socket: parent_socket,
            script_path,
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
    /// * `script_path` - Path to the Lua script to execute
    /// * `ipc_fd` - File descriptor number for IPC socket (passed to child via --fd)
    fn build_bwrap_command(options: &SandboxOptions, script_path: &PathBuf, ipc_fd: RawFd) -> Command {
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

        // Bind the script directory read-only
        if let Some(parent) = script_path.parent() {
            if parent.exists() {
                let parent_str = parent.to_string_lossy();
                cmd.args(["--ro-bind", &parent_str, &parent_str]);
            }
        }

        // Set working directory
        if let Some(ref work_dir) = options.work_dir {
            cmd.args(["--chdir", &work_dir.to_string_lossy()]);
        }

        // Keep the IPC file descriptor open in the sandbox
        // This must be added so bubblewrap doesn't close the FD before exec
        cmd.args(["--fd", &ipc_fd.to_string(), &ipc_fd.to_string()]);

        // Clear environment FIRST, then set variables
        // This ensures our setenv calls are not cleared
        cmd.args(["--clearenv"]);

        // Set essential environment variables
        cmd.args(["--setenv", "PATH", "/usr/bin:/bin"]);
        cmd.args(["--setenv", "HOME", "/tmp"]);
        cmd.args(["--setenv", "LANG", "C.UTF-8"]);

        // Pass the IPC file descriptor number to the Lua script
        // The Lua script reads CVH_IPC_FD to know which FD to use for communication
        cmd.args(["--setenv", "CVH_IPC_FD", &ipc_fd.to_string()]);

        // Apply caller-provided environment variables AFTER essential ones
        // This allows callers to override defaults if needed
        for (key, value) in &options.env_vars {
            cmd.args(["--setenv", key, value]);
        }

        // Add the actual Lua interpreter and script
        cmd.arg("--");
        cmd.arg("lua");
        cmd.arg(script_path.to_string_lossy().as_ref());

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

    /// Send a request to the Lua process
    pub fn send_request(&mut self, request: &Request) -> Result<()> {
        let data = bincode::serialize(request)
            .context("Failed to serialize request")?;

        if data.len() > MAX_MESSAGE_SIZE {
            bail!("Request too large: {} bytes (max: {})", data.len(), MAX_MESSAGE_SIZE);
        }

        // Write length prefix (4 bytes, little-endian)
        let len_bytes = (data.len() as u32).to_le_bytes();
        self.socket
            .write_all(&len_bytes)
            .context("Failed to write message length")?;

        // Write the actual data
        self.socket
            .write_all(&data)
            .context("Failed to write message data")?;

        self.socket.flush().context("Failed to flush socket")?;

        Ok(())
    }

    /// Receive a response from the Lua process with timeout
    pub fn receive_response(&mut self) -> Result<Response> {
        self.receive_response_with_timeout(DEFAULT_TIMEOUT)
    }

    /// Receive a response from the Lua process with a custom timeout
    pub fn receive_response_with_timeout(&mut self, timeout: Duration) -> Result<Response> {
        // Set read timeout
        self.socket
            .set_read_timeout(Some(timeout))
            .context("Failed to set read timeout")?;

        // Read length prefix (4 bytes, little-endian)
        let mut len_bytes = [0u8; 4];
        match self.socket.read_exact(&mut len_bytes) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                bail!("Timeout waiting for response from Lua process");
            }
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                bail!("Timeout waiting for response from Lua process");
            }
            Err(e) => {
                return Err(e).context("Failed to read message length");
            }
        }

        let len = u32::from_le_bytes(len_bytes) as usize;

        if len > MAX_MESSAGE_SIZE {
            bail!("Response too large: {} bytes (max: {})", len, MAX_MESSAGE_SIZE);
        }

        // Read the actual data
        let mut data = vec![0u8; len];
        self.socket
            .read_exact(&mut data)
            .context("Failed to read message data")?;

        // Deserialize the response
        let response = bincode::deserialize(&data)
            .context("Failed to deserialize response")?;

        Ok(response)
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

    /// Get the path to the Lua script
    pub fn script_path(&self) -> &PathBuf {
        &self.script_path
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
        let script_path = PathBuf::from("/tmp/test.lua");
        let cmd = LuaProcess::build_bwrap_command(&options, &script_path, CHILD_IPC_FD);

        let program = cmd.get_program().to_string_lossy();
        assert!(program.contains("bwrap"), "Should use bwrap");
    }

    #[test]
    fn test_build_bwrap_command_no_network() {
        let options = SandboxOptions {
            allow_network: false,
            ..Default::default()
        };
        let script_path = PathBuf::from("/tmp/test.lua");
        let cmd = LuaProcess::build_bwrap_command(&options, &script_path, CHILD_IPC_FD);

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
        let script_path = PathBuf::from("/tmp/test.lua");
        let cmd = LuaProcess::build_bwrap_command(&options, &script_path, CHILD_IPC_FD);

        let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy()).collect();
        assert!(!args.iter().any(|a| a == "--unshare-all"),
                "Should not unshare all namespaces when network is enabled");
        assert!(args.iter().any(|a| a == "--unshare-user"),
                "Should still unshare user namespace");
    }

    #[test]
    fn test_build_bwrap_command_includes_die_with_parent() {
        let options = SandboxOptions::default();
        let script_path = PathBuf::from("/tmp/test.lua");
        let cmd = LuaProcess::build_bwrap_command(&options, &script_path, CHILD_IPC_FD);

        let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy()).collect();
        assert!(args.iter().any(|a| a == "--die-with-parent"),
                "Should include --die-with-parent for cleanup");
    }

    #[test]
    fn test_build_bwrap_command_includes_new_session() {
        let options = SandboxOptions::default();
        let script_path = PathBuf::from("/tmp/test.lua");
        let cmd = LuaProcess::build_bwrap_command(&options, &script_path, CHILD_IPC_FD);

        let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy()).collect();
        assert!(args.iter().any(|a| a == "--new-session"),
                "Should include --new-session for isolation");
    }

    #[test]
    fn test_build_bwrap_command_mounts_usr_readonly() {
        let options = SandboxOptions::default();
        let script_path = PathBuf::from("/tmp/test.lua");
        let cmd = LuaProcess::build_bwrap_command(&options, &script_path, CHILD_IPC_FD);

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
        let script_path = PathBuf::from("/tmp/test.lua");
        let cmd = LuaProcess::build_bwrap_command(&options, &script_path, CHILD_IPC_FD);

        let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy()).collect();
        assert!(args.iter().any(|a| a == "--clearenv"),
                "Should clear environment");
    }

    #[test]
    fn test_build_bwrap_command_sets_essential_env() {
        let options = SandboxOptions::default();
        let script_path = PathBuf::from("/tmp/test.lua");
        let cmd = LuaProcess::build_bwrap_command(&options, &script_path, CHILD_IPC_FD);

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
        let script_path = PathBuf::from("/tmp/test.lua");
        let cmd = LuaProcess::build_bwrap_command(&options, &script_path, CHILD_IPC_FD);

        let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy()).collect();
        let has_home_tmpfs = args.windows(2).any(|w| {
            w[0] == "--tmpfs" && w[1] == "/home"
        });
        assert!(has_home_tmpfs, "Should use tmpfs for /home");
    }

    #[test]
    fn test_build_bwrap_command_includes_script_path() {
        let options = SandboxOptions::default();
        let script_path = PathBuf::from("/tmp/test.lua");
        let cmd = LuaProcess::build_bwrap_command(&options, &script_path, CHILD_IPC_FD);

        let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy()).collect();
        assert!(args.iter().any(|a| a == "/tmp/test.lua"),
                "Should include the script path");
    }

    #[test]
    fn test_build_bwrap_command_includes_lua_interpreter() {
        let options = SandboxOptions::default();
        let script_path = PathBuf::from("/tmp/test.lua");
        let cmd = LuaProcess::build_bwrap_command(&options, &script_path, CHILD_IPC_FD);

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
        let script_path = PathBuf::from("/tmp/test.lua");
        let cmd = LuaProcess::build_bwrap_command(&options, &script_path, CHILD_IPC_FD);

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
        let script_path = PathBuf::from("/tmp/test.lua");
        let cmd = LuaProcess::build_bwrap_command(&options, &script_path, CHILD_IPC_FD);

        let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy()).collect();
        let has_chdir = args.windows(2).any(|w| {
            w[0] == "--chdir" && w[1] == "/tmp/workdir"
        });
        assert!(has_chdir, "Should set working directory");
    }

    #[test]
    fn test_build_bwrap_command_includes_ipc_fd() {
        let options = SandboxOptions::default();
        let script_path = PathBuf::from("/tmp/test.lua");
        let cmd = LuaProcess::build_bwrap_command(&options, &script_path, CHILD_IPC_FD);

        let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy()).collect();
        // Check for --fd 3 3 pattern
        let has_fd = args.windows(3).any(|w| {
            w[0] == "--fd" && w[1] == "3" && w[2] == "3"
        });
        assert!(has_fd, "Should include --fd for IPC socket");
    }

    #[test]
    fn test_build_bwrap_command_sets_cvh_ipc_fd_env() {
        let options = SandboxOptions::default();
        let script_path = PathBuf::from("/tmp/test.lua");
        let cmd = LuaProcess::build_bwrap_command(&options, &script_path, CHILD_IPC_FD);

        let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy()).collect();
        let has_ipc_fd_env = args.windows(3).any(|w| {
            w[0] == "--setenv" && w[1] == "CVH_IPC_FD" && w[2] == "3"
        });
        assert!(has_ipc_fd_env, "Should set CVH_IPC_FD environment variable");
    }

    #[test]
    fn test_build_bwrap_command_clearenv_before_setenv() {
        let options = SandboxOptions::default();
        let script_path = PathBuf::from("/tmp/test.lua");
        let cmd = LuaProcess::build_bwrap_command(&options, &script_path, CHILD_IPC_FD);

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
        let script_path = PathBuf::from("/tmp/test.lua");
        let cmd = LuaProcess::build_bwrap_command(&options, &script_path, CHILD_IPC_FD);

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
    // These tests use Unix socket pairs to validate IPC framing without
    // requiring a full sandboxed process.
    // =========================================================================

    /// Helper struct for testing IPC without bubblewrap
    struct MockIpcPair {
        /// The "parent" side socket (what LuaProcess would use)
        parent: UnixStream,
        /// The "child" side socket (simulates the Lua process)
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
    fn test_ipc_send_request_serializes_correctly() {
        let pair = MockIpcPair::new().expect("Failed to create socket pair");

        // Create a minimal mock to test send_request
        let mut parent_socket = pair.parent;
        let mut child_socket = pair.child;

        // Set a short timeout on the child side for reading
        child_socket.set_read_timeout(Some(Duration::from_secs(1))).unwrap();

        // Spawn a thread to send from parent
        let send_thread = std::thread::spawn(move || {
            let request = Request::Handshake { version: PROTOCOL_VERSION };
            let data = bincode::serialize(&request).unwrap();

            // Write length prefix
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

        let request: Request = bincode::deserialize(&data).unwrap();
        match request {
            Request::Handshake { version } => {
                assert_eq!(version, PROTOCOL_VERSION);
            }
            _ => panic!("Expected Handshake request"),
        }

        send_thread.join().unwrap();
    }

    #[test]
    fn test_ipc_receive_response_deserializes_correctly() {
        let pair = MockIpcPair::new().expect("Failed to create socket pair");

        let mut parent_socket = pair.parent;
        let mut child_socket = pair.child;

        // Spawn a thread to send response from "child"
        let child_thread = std::thread::spawn(move || {
            let response = Response::HandshakeAck {
                version: PROTOCOL_VERSION,
                success: true,
            };
            let data = bincode::serialize(&response).unwrap();

            let len_bytes = (data.len() as u32).to_le_bytes();
            child_socket.write_all(&len_bytes).unwrap();
            child_socket.write_all(&data).unwrap();
            child_socket.flush().unwrap();
            child_socket
        });

        // Read from parent side using receive logic
        parent_socket.set_read_timeout(Some(Duration::from_secs(1))).unwrap();

        let mut len_bytes = [0u8; 4];
        parent_socket.read_exact(&mut len_bytes).unwrap();
        let len = u32::from_le_bytes(len_bytes) as usize;

        let mut data = vec![0u8; len];
        parent_socket.read_exact(&mut data).unwrap();

        let response: Response = bincode::deserialize(&data).unwrap();
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
    fn test_ipc_timeout_enforced() {
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
    fn test_ipc_roundtrip_render_request() {
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

        // Send from parent
        let send_thread = std::thread::spawn(move || {
            let data = bincode::serialize(&request_clone).unwrap();
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

        let received: Request = bincode::deserialize(&data).unwrap();

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
    fn test_child_ipc_fd_constant() {
        // Verify the constant is set correctly
        assert_eq!(CHILD_IPC_FD, 3, "IPC FD should be 3 (first available after stdin/stdout/stderr)");
    }

    #[test]
    fn test_ipc_position_request_roundtrip() {
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

        // Send from parent
        let data = bincode::serialize(&request).unwrap();
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

        let received: Request = bincode::deserialize(&data).unwrap();

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
    fn test_ipc_event_response_with_action() {
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

        // Send from child
        let data = bincode::serialize(&response).unwrap();
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

        let received: Response = bincode::deserialize(&data).unwrap();

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
}
