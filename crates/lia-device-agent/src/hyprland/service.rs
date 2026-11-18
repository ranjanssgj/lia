//! High-level Hyprland adapter service coordinating discovery, caching,
//! background event listening, and verified operations.
//!
//! Architectural Invariants:
//! 1. Event socket stream lines are hints that trigger re-discovery, never authoritative state.
//! 2. Cache invalidation on every focus/window/monitor/workspace event ensures freshness.
//! 3. All operations require an authoritative `EffectPermit` from `AgentStore`.
//! 4. Independent read-back postcondition verification is strictly enforced before committing success.

use super::client::HyprlandTransport;
use super::discovery::{inspect_active_window, inspect_work_area};
use super::events::{HyprlandCacheInvalidator, parse_event_line};
use super::manifest::{HyprlandCapabilityManifest, generate_hyprland_manifest};
use super::operations::execute_and_record_move;
use super::types::{
    ActiveWindowSnapshot, DisplayWorkArea, HyprlandError, MoveOperationResult,
    SameDisplayMoveCommand, WindowGeometry,
};
use crate::TrustedClock;
use crate::store::{AgentStore, EffectPermit};
use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{JoinHandle, sleep, spawn};
use std::time::Duration;

/// High-level Hyprland adapter coordinator managing discovery, caching, and operations.
pub struct HyprlandAdapterService<T: HyprlandTransport> {
    device_id: String,
    account_id: String,
    transport: T,
    invalidator: HyprlandCacheInvalidator,
    cached_snapshot: Option<ActiveWindowSnapshot>,
    cached_snapshot_epoch: u64,
    cached_work_area: Option<DisplayWorkArea>,
    cached_work_area_epoch: u64,
}

impl<T: HyprlandTransport> HyprlandAdapterService<T> {
    pub fn new(
        device_id: impl Into<String>,
        account_id: impl Into<String>,
        transport: T,
        invalidator: HyprlandCacheInvalidator,
    ) -> Self {
        Self {
            device_id: device_id.into(),
            account_id: account_id.into(),
            transport,
            invalidator,
            cached_snapshot: None,
            cached_snapshot_epoch: 0,
            cached_work_area: None,
            cached_work_area_epoch: 0,
        }
    }

    /// Access the underlying transport.
    pub fn transport(&self) -> &T {
        &self.transport
    }

    /// Access the cache invalidator.
    pub fn invalidator(&self) -> &HyprlandCacheInvalidator {
        &self.invalidator
    }

    /// Retrieves the current active window snapshot, using cached data if still valid.
    pub fn get_active_window<C: TrustedClock>(
        &mut self,
        clock: &C,
        generation_token: &str,
        force_refresh: bool,
    ) -> Result<ActiveWindowSnapshot, HyprlandError> {
        let current_epoch = self.invalidator.current_epoch();

        if !force_refresh
            && self.cached_snapshot_epoch == current_epoch
            && let Some(snapshot) = &self.cached_snapshot
        {
            let now_us = clock
                .now_us()
                .map_err(|e| HyprlandError::CompositorError(format!("Clock failure: {e}")))?;
            if snapshot.identity.is_fresh(now_us) {
                return Ok(snapshot.clone());
            }
        }

        // Cache invalidated or expired; query fresh snapshot from compositor
        let fresh_snapshot = inspect_active_window(&self.transport, clock, generation_token)?;
        self.cached_snapshot = Some(fresh_snapshot.clone());
        self.cached_snapshot_epoch = current_epoch;

        Ok(fresh_snapshot)
    }

    /// Retrieves the display work area, using cached data if still valid.
    pub fn get_work_area(
        &mut self,
        monitor_id: i64,
        force_refresh: bool,
    ) -> Result<DisplayWorkArea, HyprlandError> {
        let current_epoch = self.invalidator.current_epoch();

        if !force_refresh && self.cached_work_area_epoch == current_epoch {
            match &self.cached_work_area {
                Some(area) if area.monitor_id == monitor_id || monitor_id < 0 => {
                    return Ok(*area);
                }
                _ => {}
            }
        }

        let fresh_area = inspect_work_area(&self.transport, monitor_id)?;
        self.cached_work_area = Some(fresh_area);
        self.cached_work_area_epoch = current_epoch;

        Ok(fresh_area)
    }

    /// Generates the standardized Hyprland capability manifest.
    pub fn get_capability_manifest<C: TrustedClock>(
        &mut self,
        clock: &C,
        generation_token: &str,
        revision: i64,
        expires_in_us: i64,
    ) -> Result<HyprlandCapabilityManifest, HyprlandError> {
        let now_us = clock
            .now_us()
            .map_err(|e| HyprlandError::CompositorError(format!("Clock failure: {e}")))?;
        let observed_at = crate::time::format_rfc3339_us(now_us);
        let expires_at = crate::time::format_rfc3339_us(now_us.saturating_add(expires_in_us));

        // Attempt active window discovery; if none focused, manifest reports empty resources
        let windows = match self.get_active_window(clock, generation_token, false) {
            Ok(snap) => vec![snap],
            Err(HyprlandError::ActiveWindowAbsent) => Vec::new(),
            Err(e) => return Err(e),
        };

        Ok(generate_hyprland_manifest(
            &self.device_id,
            &self.account_id,
            revision,
            &observed_at,
            &expires_at,
            &windows,
        ))
    }

    /// Executes an authorized same-display window move with independent read-back verification.
    pub fn execute_move<C: TrustedClock>(
        &mut self,
        command: &SameDisplayMoveCommand,
        before_geom: WindowGeometry,
        clock: &C,
        store: &mut AgentStore,
        permit: &EffectPermit,
    ) -> Result<MoveOperationResult, HyprlandError> {
        let work_area = self.get_work_area(command.preserved_monitor, false)?;
        let result = execute_and_record_move(
            command,
            before_geom,
            &self.transport,
            clock,
            store,
            permit,
            &work_area,
        )?;

        // Invalidate cache following any move attempt
        self.invalidator.invalidate();
        self.cached_snapshot = None;

        Ok(result)
    }
}

/// Spawns a background listener thread connecting to Hyprland's `.socket2.sock`.
/// Reads events line-by-line and triggers cache invalidations on the invalidator.
pub fn spawn_event_listener(
    socket_path: PathBuf,
    invalidator: HyprlandCacheInvalidator,
    shutdown: Arc<AtomicBool>,
) -> JoinHandle<()> {
    spawn(move || {
        let mut backoff_ms = 50u64;

        while !shutdown.load(Ordering::Relaxed) {
            let stream = match UnixStream::connect(&socket_path) {
                Ok(s) => {
                    // Reset backoff on successful connect
                    backoff_ms = 50;
                    s
                }
                Err(_) => {
                    // Invalidate cache on disconnection/failure
                    invalidator.invalidate();
                    sleep(Duration::from_millis(backoff_ms));
                    backoff_ms = (backoff_ms * 2).min(1000);
                    continue;
                }
            };

            // 1.0s read timeout so the loop checks the shutdown signal regularly
            let _ = stream.set_read_timeout(Some(Duration::from_millis(1000)));
            let reader = BufReader::new(stream);

            for line_res in reader.lines() {
                if shutdown.load(Ordering::Relaxed) {
                    break;
                }

                match line_res {
                    Ok(line) => {
                        if let Some(event) = parse_event_line(&line) {
                            invalidator.handle_event(&event);
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                        // Periodic check for shutdown
                        continue;
                    }
                    Err(_) => {
                        // EOF or stream error
                        invalidator.invalidate();
                        break;
                    }
                }
            }

            sleep(Duration::from_millis(backoff_ms));
        }
    })
}
