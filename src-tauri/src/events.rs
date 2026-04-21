//! Named event channels emitted on the Tauri bus.
//!
//! Centralizing the names here prevents string-typo drift between the Rust
//! emitter and the TypeScript `listen()` call sites.

pub const SD_MOUNTED: &str = "sd:mounted";
pub const SD_REMOVED: &str = "sd:removed";
pub const STAGE_PROGRESS: &str = "stage:progress";
pub const DOWNLOAD_PROGRESS: &str = "download:progress";
