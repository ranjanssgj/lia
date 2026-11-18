//! Window and monitor discovery for Hyprland v0.57.2.
//!
//! Architectural Invariants:
//! 1. Privacy: Raw window titles and window content are NEVER extracted, stored, or logged.
//! 2. Short-lived ephemeral references: Freshness deadline is capped at 10 seconds.
//! 3. Compositor validation: Active window must be mapped and floating to qualify for same-display move.

use super::client::HyprlandTransport;
use super::types::{
    ActiveWindowSnapshot, DisplayWorkArea, HyprlandError, WindowGeometry, WindowIdentity,
};
use crate::TrustedClock;
use serde::Deserialize;

/// Internal raw DTO for parsing Hyprland's `j/activewindow` response.
/// Deliberately omits `title` and `initialTitle` to enforce compile-time privacy guarantees.
#[derive(Debug, Deserialize)]
struct RawActiveWindow {
    #[serde(default)]
    address: String,
    #[serde(default)]
    mapped: bool,
    #[serde(default)]
    hidden: bool,
    #[serde(default)]
    at: Vec<i32>,
    #[serde(default)]
    size: Vec<i32>,
    #[serde(default)]
    workspace: RawWorkspace,
    #[serde(default)]
    floating: bool,
    #[serde(default)]
    monitor: i64,
    #[serde(default)]
    class: String,
}

#[derive(Debug, Default, Deserialize)]
struct RawWorkspace {
    #[serde(default)]
    id: i64,
}

/// Internal raw DTO for parsing Hyprland's `j/monitors` response.
#[derive(Debug, Deserialize)]
struct RawMonitor {
    id: i64,
    #[serde(default)]
    x: i32,
    #[serde(default)]
    y: i32,
    #[serde(default)]
    width: i32,
    #[serde(default)]
    height: i32,
    #[serde(default)]
    reserved: Vec<i32>,
    #[serde(default)]
    focused: bool,
}

/// Discovers the active window from Hyprland and constructs a privacy-redacted snapshot.
pub fn inspect_active_window<T, C>(
    transport: &T,
    clock: &C,
    generation_token: &str,
) -> Result<ActiveWindowSnapshot, HyprlandError>
where
    T: HyprlandTransport,
    C: TrustedClock,
{
    let response = transport.query("j/activewindow")?;
    let raw: RawActiveWindow = serde_json::from_str(&response).map_err(|e| {
        HyprlandError::MalformedResponse(format!("Failed to parse active window JSON: {e}"))
    })?;

    if raw.address.is_empty() || raw.address == "0x0" || !raw.mapped || raw.hidden {
        return Err(HyprlandError::ActiveWindowAbsent);
    }

    if raw.at.len() < 2 || raw.size.len() < 2 {
        return Err(HyprlandError::MalformedResponse(
            "Active window geometry fields missing or malformed".into(),
        ));
    }

    let now_us = clock
        .now_us()
        .map_err(|e| HyprlandError::MalformedResponse(format!("Trusted clock failure: {e}")))?;
    // 10-second reference freshness ceiling
    let fresh_until_us = now_us.saturating_add(10_000_000);

    let identity = WindowIdentity {
        address: raw.address,
        generation_token: generation_token.to_string(),
        workspace_id: raw.workspace.id,
        monitor_id: raw.monitor,
        observed_at_us: now_us,
        fresh_until_us,
    };

    let geometry = WindowGeometry::new(raw.at[0], raw.at[1], raw.size[0], raw.size[1]);

    // Sanitize app class: trim whitespace, default to "unknown" if empty
    let app_class = if raw.class.trim().is_empty() {
        "unknown".to_string()
    } else {
        raw.class.trim().to_string()
    };

    Ok(ActiveWindowSnapshot {
        identity,
        geometry,
        app_class,
        title_redacted: true,
        floating: raw.floating,
    })
}

/// Discovers the monitor work area for the specified monitor ID.
pub fn inspect_work_area<T>(
    transport: &T,
    monitor_id: i64,
) -> Result<DisplayWorkArea, HyprlandError>
where
    T: HyprlandTransport,
{
    let response = transport.query("j/monitors")?;
    let monitors: Vec<RawMonitor> = serde_json::from_str(&response).map_err(|e| {
        HyprlandError::MalformedResponse(format!("Failed to parse monitors JSON: {e}"))
    })?;

    let mon = monitors
        .into_iter()
        .find(|m| m.id == monitor_id || (monitor_id < 0 && m.focused))
        .ok_or_else(|| {
            HyprlandError::CompositorError(format!("Monitor ID {monitor_id} not found"))
        })?;

    // reserved is [top, bottom, left, right]
    let (top, bottom, left, right) = if mon.reserved.len() >= 4 {
        (
            mon.reserved[0],
            mon.reserved[1],
            mon.reserved[2],
            mon.reserved[3],
        )
    } else {
        (0, 0, 0, 0)
    };

    let usable_x = mon.x.saturating_add(left);
    let usable_y = mon.y.saturating_add(top);
    let usable_width = mon.width.saturating_sub(left).saturating_sub(right);
    let usable_height = mon.height.saturating_sub(top).saturating_sub(bottom);

    Ok(DisplayWorkArea {
        x: usable_x,
        y: usable_y,
        width: usable_width,
        height: usable_height,
        monitor_id: mon.id,
    })
}
