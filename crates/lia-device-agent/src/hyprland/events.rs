//! Hyprland `.socket2.sock` event stream parsing and cache invalidation.
//!
//! Architectural Invariants:
//! 1. "Treat event stream lines as hints that trigger re-discovery, not as authoritative resource state."
//! 2. Privacy: Window titles in event strings (e.g. `activewindow>>[title],[class]`) are explicitly discarded.
//! 3. Compositor disconnect or error invalidates freshness until rediscovery completes.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Parsed event emitted by Hyprland's asynchronous event socket.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HyprlandEvent {
    /// Active window changed. Note: raw title is intentionally redacted.
    ActiveWindowChanged {
        address: Option<String>,
        window_title_redacted: bool,
    },
    /// A window was closed.
    WindowClosed { address: String },
    /// A window was moved across coordinates or workspace.
    WindowMoved { address: String },
    /// Monitor output added.
    MonitorAdded { name: String },
    /// Monitor output removed.
    MonitorRemoved { name: String },
    /// Active monitor focus changed.
    FocusedMonitorChanged { name: String },
    /// Active workspace changed.
    WorkspaceChanged { id: i64 },
    /// Generic or unrecognized event line.
    Other(String),
}

/// Parses a single raw event line from Hyprland's `.socket2.sock`.
/// Lines follow the format: `[event_name]>>[event_data]`.
pub fn parse_event_line(line: &str) -> Option<HyprlandEvent> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let (event_name, data) = match trimmed.split_once(">>") {
        Some((name, data)) => (name, data),
        None => return Some(HyprlandEvent::Other(trimmed.to_string())),
    };

    match event_name {
        "activewindow" => {
            // Data format: [title],[class]
            // PRIVACY INVARIANT: Discard title; never expose it.
            Some(HyprlandEvent::ActiveWindowChanged {
                address: None,
                window_title_redacted: true,
            })
        }
        "activewindowv2" => {
            // Data format: [window_address]
            Some(HyprlandEvent::ActiveWindowChanged {
                address: Some(data.trim().to_string()),
                window_title_redacted: true,
            })
        }
        "closewindow" => Some(HyprlandEvent::WindowClosed {
            address: data.trim().to_string(),
        }),
        "movewindow" | "movewindowv2" => {
            let address = data.split(',').next().unwrap_or(data).trim().to_string();
            Some(HyprlandEvent::WindowMoved { address })
        }
        "monitoradded" | "monitoraddedv2" => Some(HyprlandEvent::MonitorAdded {
            name: data.trim().to_string(),
        }),
        "monitorremoved" => Some(HyprlandEvent::MonitorRemoved {
            name: data.trim().to_string(),
        }),
        "focusedmon" => {
            let name = data.split(',').next().unwrap_or(data).trim().to_string();
            Some(HyprlandEvent::FocusedMonitorChanged { name })
        }
        "workspace" | "workspacev2" => {
            let id = data
                .split(',')
                .next()
                .unwrap_or(data)
                .trim()
                .parse::<i64>()
                .unwrap_or(0);
            Some(HyprlandEvent::WorkspaceChanged { id })
        }
        _ => Some(HyprlandEvent::Other(trimmed.to_string())),
    }
}

/// Thread-safe invalidation tracker holding an epoch counter.
/// Any relevant compositor event increments the epoch, invalidating prior snapshots.
#[derive(Clone, Debug, Default)]
pub struct HyprlandCacheInvalidator {
    epoch: Arc<AtomicU64>,
}

impl HyprlandCacheInvalidator {
    pub fn new() -> Self {
        Self {
            epoch: Arc::new(AtomicU64::new(1)),
        }
    }

    /// Read the current cache generation epoch.
    pub fn current_epoch(&self) -> u64 {
        self.epoch.load(Ordering::SeqCst)
    }

    /// Explicitly trigger a cache invalidation.
    pub fn invalidate(&self) -> u64 {
        self.epoch.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Check whether a previously recorded epoch is still valid.
    pub fn is_valid(&self, recorded_epoch: u64) -> bool {
        self.current_epoch() == recorded_epoch
    }

    /// Processes an incoming event. If the event modifies window/monitor state,
    /// increments the epoch and returns true.
    pub fn handle_event(&self, event: &HyprlandEvent) -> bool {
        match event {
            HyprlandEvent::ActiveWindowChanged { .. }
            | HyprlandEvent::WindowClosed { .. }
            | HyprlandEvent::WindowMoved { .. }
            | HyprlandEvent::MonitorAdded { .. }
            | HyprlandEvent::MonitorRemoved { .. }
            | HyprlandEvent::FocusedMonitorChanged { .. }
            | HyprlandEvent::WorkspaceChanged { .. } => {
                self.invalidate();
                true
            }
            HyprlandEvent::Other(_) => false,
        }
    }
}
