//! SD card detection, content classification, and firmware staging.
//!
//! Three responsibilities, kept in separate modules so they are independently
//! testable:
//!
//! * [`watcher`] — emits `Mounted` / `Removed` events for removable drives.
//! * [`scan`] — classifies an already-mounted drive (Walksnail? which
//!   hardware? staged vs. running version?).
//! * [`stage`] — downloads + verifies + copies a `.img` onto an SD card.

pub mod scan;
pub mod stage;
pub mod watcher;

pub use scan::{scan, SdContents, Signal};
pub use stage::{stage_firmware, StageOutcome, StageProgress};
pub use watcher::{looks_like_removable_mount, run_watcher, spawn_watcher, SdEvent};
