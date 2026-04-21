//! Removable-drive watcher.
//!
//! Uses `sysinfo::Disks` polled every 2 seconds. Native device-change APIs
//! (Win32 `RegisterDeviceNotification`, macOS DiskArbitration, Linux udev)
//! would be faster but are meaningfully more code per platform. Polling is
//! simple, works everywhere, and 2s latency on plug-in is unnoticeable in
//! practice.

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;
use tracing::debug;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SdEvent {
    Mounted { path: PathBuf },
    Removed { path: PathBuf },
}

/// Run the watcher as an async task.
///
/// The caller is responsible for spawning this on whichever runtime it uses
/// (Tauri's `async_runtime::spawn`, `tokio::spawn`, etc.). Decoupling this
/// from a specific runtime keeps the crate free of Tauri/Tokio startup-order
/// assumptions — `spawn_watcher` used to panic with "no reactor running"
/// when called from Tauri's `setup` closure.
///
/// The watcher exits when the receiver end of `tx` is dropped.
pub async fn run_watcher(tx: Sender<SdEvent>) {
    run(tx).await
}

/// Convenience wrapper: spawn the watcher on the current Tokio runtime.
/// Only call this from within an active runtime context.
pub fn spawn_watcher(tx: Sender<SdEvent>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(run_watcher(tx))
}

async fn run(tx: Sender<SdEvent>) {
    let mut known: HashSet<PathBuf> = list_removable();
    // Emit initial set so the UI starts with the current state.
    for p in &known {
        if tx.send(SdEvent::Mounted { path: p.clone() }).await.is_err() {
            return;
        }
    }
    loop {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let now = list_removable();
        for added in now.difference(&known) {
            debug!(path = %added.display(), "sd mounted");
            if tx
                .send(SdEvent::Mounted {
                    path: added.clone(),
                })
                .await
                .is_err()
            {
                return;
            }
        }
        for removed in known.difference(&now) {
            debug!(path = %removed.display(), "sd removed");
            if tx
                .send(SdEvent::Removed {
                    path: removed.clone(),
                })
                .await
                .is_err()
            {
                return;
            }
        }
        known = now;
    }
}

/// Enumerate currently-mounted removable drives, returning each mount point.
///
/// `sysinfo::Disk::is_removable()` is the canonical source, but macOS
/// frequently lies — the built-in SD card reader on some MacBook models
/// reports `is_removable = false`, which caused SDs to never appear in the
/// UI. We therefore also treat mounts under `/Volumes/` (macOS), `/media/`
/// and `/run/media/` (Linux) as removable, which catches real SD cards at
/// the cost of surfacing the odd DMG mount.
pub fn list_removable() -> HashSet<PathBuf> {
    let disks = sysinfo::Disks::new_with_refreshed_list();
    disks
        .iter()
        .filter_map(|d| {
            let mp = d.mount_point();
            if d.is_removable() || looks_like_removable_mount(mp) {
                Some(mp.to_path_buf())
            } else {
                None
            }
        })
        .collect()
}

/// Heuristic fallback for OSes whose is_removable flag is unreliable.
///
/// Exposed for tests and for manual-picker validation in the frontend.
pub fn looks_like_removable_mount(path: &std::path::Path) -> bool {
    if path == std::path::Path::new("/") {
        return false;
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(p) = path.to_str() {
            return p.starts_with("/Volumes/") && p != "/Volumes/Macintosh HD";
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Some(p) = path.to_str() {
            return p.starts_with("/media/") || p.starts_with("/run/media/");
        }
    }
    #[cfg(target_os = "windows")]
    {
        // Windows assigns drive letters like E:\ to removable media; trust
        // sysinfo's is_removable() flag on that platform — this heuristic is
        // not needed.
        let _ = path;
    }
    false
}
