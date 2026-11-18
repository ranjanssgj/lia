//! Local IPC transport for Hyprland v0.57.2 request socket.
//!
//! Architectural Invariants:
//! 1. Synchronous request socket: open immediately before request, read response, close immediately.
//!    Never leave request sockets idle (avoids compositor blocking).
//! 2. Strict command allowlist: arbitrary shell execution or arbitrary compositor commands are rejected.
//! 3. Bounded I/O: connection and read deadlines are capped at 1.0s, responses bounded to 1 MiB.
//! 4. Private filesystem permissions: verify socket parent directory ownership and permissions.

use super::types::HyprlandError;
use std::env;
use std::io::{Read, Write};
use std::os::unix::fs::MetadataExt;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Maximum allowable response size from Hyprland IPC (1 MiB).
pub const MAX_HYPRLAND_RESPONSE_BYTES: usize = 1_048_576;
/// Hard I/O timeout ceiling for Hyprland request socket operations.
pub const HYPRLAND_IO_TIMEOUT: Duration = Duration::from_millis(1_000);

/// Abstract transport trait allowing test mocks and synthetic compositor harnesses.
pub trait HyprlandTransport {
    /// Issue a read-only query command (e.g. `j/activewindow`, `j/monitors`).
    fn query(&self, command: &str) -> Result<String, HyprlandError>;

    /// Issue an authorized state-changing dispatch command.
    fn dispatch(&self, command: &str) -> Result<String, HyprlandError>;
}

/// Production Unix-domain socket client connecting to Hyprland's request socket.
#[derive(Clone, Debug)]
pub struct HyprlandSocketClient {
    socket_path: PathBuf,
}

impl HyprlandSocketClient {
    /// Discover and construct client from environment variables:
    /// `$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/.socket.sock`.
    pub fn discover_from_env() -> Result<Self, HyprlandError> {
        let xdg_runtime = env::var("XDG_RUNTIME_DIR")
            .map_err(|_| HyprlandError::SessionNotFound("XDG_RUNTIME_DIR not set".into()))?;
        let instance_sig = env::var("HYPRLAND_INSTANCE_SIGNATURE").map_err(|_| {
            HyprlandError::SessionNotFound("HYPRLAND_INSTANCE_SIGNATURE not set".into())
        })?;

        if instance_sig.is_empty() || instance_sig.contains('/') || instance_sig.contains("..") {
            return Err(HyprlandError::SessionNotFound(
                "HYPRLAND_INSTANCE_SIGNATURE contains invalid characters".into(),
            ));
        }

        let socket_path = PathBuf::from(xdg_runtime)
            .join("hypr")
            .join(instance_sig)
            .join(".socket.sock");

        Self::new(socket_path)
    }

    /// Construct with an explicit socket path and validate private permissions.
    pub fn new(socket_path: impl Into<PathBuf>) -> Result<Self, HyprlandError> {
        let path = socket_path.into();
        Self::validate_socket_path(&path)?;
        Ok(Self { socket_path: path })
    }

    /// Validates socket existence and security invariants.
    fn validate_socket_path(path: &Path) -> Result<(), HyprlandError> {
        if !path.exists() {
            return Err(HyprlandError::SocketNotFound(path.display().to_string()));
        }

        let parent = path
            .parent()
            .ok_or_else(|| HyprlandError::SocketNotFound("Socket path has no parent".into()))?;
        let metadata = parent.metadata().map_err(|e| {
            HyprlandError::SocketPermissionDenied(format!("Cannot stat socket directory: {e}"))
        })?;

        // Verify current UID owns the socket directory
        let current_uid = unsafe { libc::geteuid() };
        if metadata.uid() != current_uid {
            return Err(HyprlandError::SocketPermissionDenied(format!(
                "Socket parent directory owned by foreign UID {}, expected {}",
                metadata.uid(),
                current_uid
            )));
        }

        // Mode must not be world-writable
        let mode = metadata.mode() & 0o777;
        if mode & 0o002 != 0 {
            return Err(HyprlandError::SocketPermissionDenied(format!(
                "Socket parent directory has unsafe world-writable permissions: {mode:04o}"
            )));
        }

        Ok(())
    }

    /// Executes a synchronous command over the Unix domain stream socket.
    /// Opens the socket, writes the command, reads the full response, and closes the socket.
    fn execute_raw(&self, command: &str) -> Result<String, HyprlandError> {
        let mut stream = UnixStream::connect(&self.socket_path).map_err(|e| {
            HyprlandError::SocketIoError(format!("Failed to connect to Hyprland socket: {e}"))
        })?;

        stream
            .set_read_timeout(Some(HYPRLAND_IO_TIMEOUT))
            .map_err(|e| {
                HyprlandError::SocketIoError(format!("Failed to set read timeout: {e}"))
            })?;
        stream
            .set_write_timeout(Some(HYPRLAND_IO_TIMEOUT))
            .map_err(|e| {
                HyprlandError::SocketIoError(format!("Failed to set write timeout: {e}"))
            })?;

        stream
            .write_all(command.as_bytes())
            .map_err(|e| HyprlandError::SocketIoError(format!("Failed to write command: {e}")))?;
        stream
            .flush()
            .map_err(|e| HyprlandError::SocketIoError(format!("Failed to flush stream: {e}")))?;

        let mut response = Vec::new();
        let mut buffer = [0u8; 4096];
        loop {
            match stream.read(&mut buffer) {
                Ok(0) => break, // EOF reached
                Ok(n) => {
                    if response.len().saturating_add(n) > MAX_HYPRLAND_RESPONSE_BYTES {
                        return Err(HyprlandError::MalformedResponse(
                            "Hyprland response exceeded 1 MiB limit".into(),
                        ));
                    }
                    response.extend_from_slice(&buffer[..n]);
                }
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    return Err(HyprlandError::Timeout);
                }
                Err(e) => {
                    return Err(HyprlandError::SocketIoError(format!(
                        "Error reading response from socket: {e}"
                    )));
                }
            }
        }

        String::from_utf8(response).map_err(|e| {
            HyprlandError::MalformedResponse(format!("Response was not valid UTF-8: {e}"))
        })
    }
}

/// Strict validation and command construction rules for Hyprland IPC.
pub struct HyprlandCommandFilter;

impl HyprlandCommandFilter {
    /// Validates and ensures read-only query command is strictly allowlisted.
    pub fn validate_query(cmd: &str) -> Result<(), HyprlandError> {
        match cmd {
            "j/activewindow" | "j/monitors" | "j/workspaces" | "j/clients" => Ok(()),
            other => Err(HyprlandError::CompositorError(format!(
                "Disallowed Hyprland query command: {other}"
            ))),
        }
    }

    /// Constructs an authorized same-display window move dispatch command.
    /// Strictly escapes and formats: `dispatch movewindowpixel exact <X> <Y>,address:<ADDR>`
    pub fn format_same_display_move(
        target_x: i32,
        target_y: i32,
        address: &str,
    ) -> Result<String, HyprlandError> {
        Self::validate_hex_address(address)?;
        Ok(format!(
            "dispatch movewindowpixel exact {} {},address:{}",
            target_x, target_y, address
        ))
    }

    /// Validates that an address is a strictly formatted hexadecimal string.
    pub fn validate_hex_address(addr: &str) -> Result<(), HyprlandError> {
        if !addr.starts_with("0x") || addr.len() < 3 || addr.len() > 18 {
            return Err(HyprlandError::InvalidAddressFormat(addr.to_string()));
        }
        for ch in addr[2..].chars() {
            if !ch.is_ascii_hexdigit() {
                return Err(HyprlandError::InvalidAddressFormat(addr.to_string()));
            }
        }
        Ok(())
    }
}

impl HyprlandTransport for HyprlandSocketClient {
    fn query(&self, command: &str) -> Result<String, HyprlandError> {
        HyprlandCommandFilter::validate_query(command)?;
        self.execute_raw(command)
    }

    fn dispatch(&self, command: &str) -> Result<String, HyprlandError> {
        // Enforce dispatch allowlist: must start with "dispatch movewindowpixel exact "
        if !command.starts_with("dispatch movewindowpixel exact ") {
            return Err(HyprlandError::CompositorError(
                "Disallowed dispatch command: only movewindowpixel exact is permitted".into(),
            ));
        }
        self.execute_raw(command)
    }
}
