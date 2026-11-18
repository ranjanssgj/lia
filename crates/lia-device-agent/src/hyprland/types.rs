//! Typed contracts, resource models, and error taxonomy for the Hyprland v0.57.2 adapter.
//!
//! Architectural Invariants:
//! 1. Window references are ephemeral and generation-bound; stale identities fail closed.
//! 2. Active-window observation strictly redacts raw window titles and window content.
//! 3. Only same-display moves preserving original width, height, and workspace are permitted.
//! 4. Operation outcomes distinguish [`MoveOperationResult::VerifiedSuccess`],
//!    [`MoveOperationResult::VerifiedFailure`], and [`MoveOperationResult::UnknownOutcome`].

use serde::{Deserialize, Serialize};
use std::fmt;

/// Ephemeral identity binding a window resource to a specific observation,
/// display output, workspace, and freshness window.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowIdentity {
    /// Opaque hex address formatted by Hyprland (e.g. "0x55a8f2c3b1a0").
    pub address: String,
    /// Ephemeral generation token issued by the device agent.
    pub generation_token: String,
    /// Workspace identifier where the window was observed.
    pub workspace_id: i64,
    /// Display monitor identifier hosting the window.
    pub monitor_id: i64,
    /// Observation timestamp in integer microseconds.
    pub observed_at_us: i64,
    /// Expiration deadline in integer microseconds (short-lived, e.g. 10s).
    pub fresh_until_us: i64,
}

impl WindowIdentity {
    /// Checks whether the window resource reference is currently fresh.
    pub fn is_fresh(&self, now_us: i64) -> bool {
        now_us <= self.fresh_until_us && now_us >= self.observed_at_us
    }
}

/// 2D bounding geometry of a window in compositor desktop coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowGeometry {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl WindowGeometry {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// Usable display work area bounded by display dimensions and reserved panel margins.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisplayWorkArea {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub monitor_id: i64,
}

impl DisplayWorkArea {
    /// Validates whether a target window rectangle fits fully within this display work area.
    pub fn contains_rect(&self, x: i32, y: i32, width: i32, height: i32) -> bool {
        if width <= 0 || height <= 0 || self.width <= 0 || self.height <= 0 {
            return false;
        }
        let right = match x.checked_add(width) {
            Some(r) => r,
            None => return false,
        };
        let bottom = match y.checked_add(height) {
            Some(b) => b,
            None => return false,
        };
        let area_right = match self.x.checked_add(self.width) {
            Some(r) => r,
            None => return false,
        };
        let area_bottom = match self.y.checked_add(self.height) {
            Some(b) => b,
            None => return false,
        };

        x >= self.x && right <= area_right && y >= self.y && bottom <= area_bottom
    }
}

/// Redacted snapshot of an observed active window.
/// Conforms to Privacy Invariant: raw window titles and user text are NEVER included.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveWindowSnapshot {
    pub identity: WindowIdentity,
    pub geometry: WindowGeometry,
    /// Anonymized or application class token (e.g. "kitty", "foot", "code-oss").
    pub app_class: String,
    /// Explicit verification that the title has been redacted.
    pub title_redacted: bool,
    /// Whether the window is floating (tiled windows cannot be repositioned via exact pixel coords).
    pub floating: bool,
}

/// Authorized command to move a floating window within the same display.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SameDisplayMoveCommand {
    pub window: WindowIdentity,
    pub target_x: i32,
    pub target_y: i32,
    pub preserved_width: i32,
    pub preserved_height: i32,
    pub preserved_workspace: i64,
    pub preserved_monitor: i64,
}

/// Tri-state outcome of an adapter operation in accordance with Invariant 7.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoveOperationResult {
    /// The move request succeeded AND independent read-back proved target geometry.
    VerifiedSuccess {
        before: WindowGeometry,
        after: WindowGeometry,
        attempt_id: String,
    },
    /// The operation failed deterministically without effect or was rejected before dispatch.
    VerifiedFailure { reason: String, attempt_id: String },
    /// The request may have reached the compositor, but outcome could not be verified.
    /// Invariant 8: Acknowledgement != execution; blind retry forbidden.
    UnknownOutcome {
        attempt_id: String,
        reconciliation_required: bool,
    },
}

/// Comprehensive error taxonomy for Hyprland adapter operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HyprlandError {
    SessionNotFound(String),
    SocketNotFound(String),
    SocketPermissionDenied(String),
    SocketIoError(String),
    MalformedResponse(String),
    ActiveWindowAbsent,
    WindowNotFloating,
    StaleIdentity {
        expected_address: String,
        actual_address: String,
    },
    ExpiredResource {
        observed_at_us: i64,
        fresh_until_us: i64,
        now_us: i64,
    },
    DimensionMismatch {
        expected_width: i32,
        actual_width: i32,
        expected_height: i32,
        actual_height: i32,
    },
    WorkspaceMismatch {
        expected: i64,
        actual: i64,
    },
    MonitorMismatch {
        expected: i64,
        actual: i64,
    },
    OutOfBounds {
        target_x: i32,
        target_y: i32,
        width: i32,
        height: i32,
        work_area: DisplayWorkArea,
    },
    InvalidAddressFormat(String),
    Timeout,
    CompositorError(String),
    PermissionDenied(String),
}

impl fmt::Display for HyprlandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SessionNotFound(msg) => write!(f, "Hyprland session not found: {msg}"),
            Self::SocketNotFound(path) => write!(f, "Hyprland socket path not found: {path}"),
            Self::SocketPermissionDenied(msg) => {
                write!(f, "Hyprland socket permission denied: {msg}")
            }
            Self::SocketIoError(msg) => write!(f, "Hyprland socket I/O error: {msg}"),
            Self::MalformedResponse(msg) => {
                write!(f, "Malformed response from Hyprland IPC: {msg}")
            }
            Self::ActiveWindowAbsent => {
                write!(f, "No active window focused or mapped in compositor")
            }
            Self::WindowNotFloating => write!(
                f,
                "Window is tiled; same-display pixel move requires floating window"
            ),
            Self::StaleIdentity {
                expected_address,
                actual_address,
            } => {
                write!(
                    f,
                    "Stale window identity: expected {expected_address}, observed {actual_address}"
                )
            }
            Self::ExpiredResource {
                observed_at_us,
                fresh_until_us,
                now_us,
            } => {
                write!(
                    f,
                    "Window reference expired (observed {observed_at_us}, deadline {fresh_until_us}, now {now_us})"
                )
            }
            Self::DimensionMismatch {
                expected_width,
                actual_width,
                expected_height,
                actual_height,
            } => {
                write!(
                    f,
                    "Window dimensions changed: expected {expected_width}x{expected_height}, observed {actual_width}x{actual_height}"
                )
            }
            Self::WorkspaceMismatch { expected, actual } => {
                write!(
                    f,
                    "Workspace changed: expected {expected}, observed {actual}"
                )
            }
            Self::MonitorMismatch { expected, actual } => {
                write!(f, "Monitor changed: expected {expected}, observed {actual}")
            }
            Self::OutOfBounds {
                target_x,
                target_y,
                width,
                height,
                work_area,
            } => {
                write!(
                    f,
                    "Target window rect ({target_x},{target_y} {width}x{height}) exceeds display work area ({work_area:?})"
                )
            }
            Self::InvalidAddressFormat(addr) => {
                write!(f, "Invalid hex window address format: {addr}")
            }
            Self::Timeout => write!(f, "Hyprland IPC request timed out"),
            Self::CompositorError(msg) => write!(f, "Hyprland compositor rejected command: {msg}"),
            Self::PermissionDenied(msg) => write!(f, "Operation denied by policy: {msg}"),
        }
    }
}

impl std::error::Error for HyprlandError {}
