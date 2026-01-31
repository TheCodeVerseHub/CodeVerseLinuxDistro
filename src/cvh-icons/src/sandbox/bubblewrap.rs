//! Bubblewrap sandbox implementation
//!
//! Uses bubblewrap (bwrap) for container-like isolation of Lua scripts.

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

use super::SandboxOptions;

/// Bubblewrap sandbox wrapper
#[allow(dead_code)]
pub struct BubblewrapSandbox {
    options: SandboxOptions,
}

#[allow(dead_code)]
impl BubblewrapSandbox {
    /// Create a new bubblewrap sandbox
    pub fn new(options: SandboxOptions) -> Self {
        Self { options }
    }

    /// Build the bwrap command with appropriate arguments
    pub fn build_command(&self, program: &str, args: &[&str]) -> Command {
        let mut cmd = Command::new("bwrap");

        // Die when parent dies
        cmd.arg("--die-with-parent");

        // New session for isolation
        cmd.arg("--new-session");

        // Unshare namespaces
        if self.options.allow_network {
            // Keep network, unshare others
            cmd.args(["--unshare-user", "--unshare-pid", "--unshare-uts", "--unshare-cgroup"]);
        } else {
            // Unshare everything including network
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
        for path in &self.options.read_only_paths {
            if path.exists() {
                let path_str = path.to_string_lossy();
                cmd.args(["--ro-bind", &path_str, &path_str]);
            }
        }

        // Add configured read-write paths
        for path in &self.options.read_write_paths {
            if path.exists() {
                let path_str = path.to_string_lossy();
                cmd.args(["--bind", &path_str, &path_str]);
            }
        }

        // Set working directory
        if let Some(ref work_dir) = self.options.work_dir {
            cmd.args(["--chdir", &work_dir.to_string_lossy()]);
        }

        // Pass environment variables
        for (key, value) in &self.options.env_vars {
            cmd.args(["--setenv", key, value]);
        }

        // Clear most environment
        cmd.args(["--clearenv"]);

        // Set essential environment
        cmd.args(["--setenv", "PATH", "/usr/bin:/bin"]);
        cmd.args(["--setenv", "HOME", "/tmp"]);
        cmd.args(["--setenv", "LANG", "C.UTF-8"]);

        // Add the actual program and arguments
        cmd.arg("--");
        cmd.arg(program);
        cmd.args(args);

        cmd
    }

    /// Run a command in the sandbox
    pub fn run(&self, program: &str, args: &[&str]) -> Result<std::process::Output> {
        let mut cmd = self.build_command(program, args);

        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .context("Failed to execute sandboxed command")
    }

    /// Spawn a command in the sandbox (non-blocking)
    pub fn spawn(&self, program: &str, args: &[&str]) -> Result<Child> {
        let mut cmd = self.build_command(program, args);

        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn sandboxed command")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_creation() {
        let options = SandboxOptions::default();
        let sandbox = BubblewrapSandbox::new(options);

        // Just verify it builds without panicking
        let cmd = sandbox.build_command("echo", &["hello"]);
        assert!(cmd.get_program().to_string_lossy().contains("bwrap"));
    }
}
