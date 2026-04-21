//! Classifies the contents of an SD card.
//!
//! Combines multiple signals to decide which Walksnail hardware variant is in
//! play and which firmware version is *staged* (the `.img` waiting in root)
//! vs. *running* (extracted from DVR `.srt` sidecars if `debug_info_in_srt`
//! has been enabled on the device).

use std::path::{Path, PathBuf};

use hardware::{parse_filename, parse_srt_version, Hardware, Version};
use serde::{Deserialize, Serialize};
use tracing::trace;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdContents {
    pub root: PathBuf,
    pub is_walksnail: bool,
    pub variant: Option<Hardware>,
    /// Version from the firmware `.img` in the SD root — what will flash on
    /// next boot if the user powers the device on right now.
    pub staged_version: Option<Version>,
    /// Version extracted from the newest DVR `.srt` sidecar — what is
    /// currently *running* on the device.
    pub running_version: Option<Version>,
    pub signals: Vec<Signal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Signal {
    StagedImg { filename: String },
    SrtFirmware { version: Version, path: PathBuf },
    MarkerFile { name: String },
    UserfontDir,
    IndependUpgrade,
}

/// Max `.srt` files we open before giving up — keeps a huge DVR library from
/// blocking the UI on plug-in.
const MAX_SRT_INSPECT: usize = 16;

/// Max bytes we read from a single `.srt` file while looking for `FW:x.y.z`.
/// Version appears near the start of every telemetry line, so 64 KiB is more
/// than enough without slurping multi-megabyte DVR sidecars.
const MAX_SRT_BYTES: usize = 64 * 1024;

/// Classify an already-mounted SD card rooted at `root`.
pub fn scan(root: &Path) -> SdContents {
    let mut contents = SdContents {
        root: root.to_path_buf(),
        is_walksnail: false,
        variant: None,
        staged_version: None,
        running_version: None,
        signals: Vec::new(),
    };

    scan_root_files(root, &mut contents);
    scan_srt_files(root, &mut contents);

    contents
}

fn scan_root_files(root: &Path, contents: &mut SdContents) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let path = entry.path();
        let file_type = entry.file_type().ok();

        if let Some((hw, ver)) = parse_filename(&name) {
            contents.is_walksnail = true;
            contents.variant = Some(hw);
            contents.staged_version = Some(ver);
            contents.signals.push(Signal::StagedImg {
                filename: name.to_string(),
            });
            continue;
        }

        if file_type.is_some_and(|t| t.is_dir()) && name.eq_ignore_ascii_case("userfont") {
            contents.is_walksnail = true;
            contents.signals.push(Signal::UserfontDir);
            continue;
        }

        let lowered = name.to_ascii_lowercase();
        match lowered.as_str() {
            "independ_upgrade.txt" => {
                contents.is_walksnail = true;
                contents.signals.push(Signal::IndependUpgrade);
            }
            // Avatar_info.txt appears on Goggles X (and likely others) with
            // key=value lines such as "wifi=on". Generic across Walksnail
            // devices — confirms the SD belongs to *some* Walksnail but not
            // which variant.
            "debug_info_in_srt.txt" | "avatar_time.txt" | "avatar_info.txt" => {
                contents.is_walksnail = true;
                contents.signals.push(Signal::MarkerFile {
                    name: name.to_string(),
                });
            }
            _ => {}
        }
        trace!(file = %path.display(), "inspected");
    }
}

fn scan_srt_files(root: &Path, contents: &mut SdContents) {
    let mut best: Option<(std::time::SystemTime, Version, PathBuf)> = None;
    let mut inspected = 0usize;
    for entry in WalkDir::new(root)
        .max_depth(3)
        .into_iter()
        .filter_map(Result::ok)
    {
        if inspected >= MAX_SRT_INSPECT {
            break;
        }
        let path = entry.path();
        if path
            .extension()
            .is_none_or(|e| !e.eq_ignore_ascii_case("srt"))
        {
            continue;
        }
        inspected += 1;
        let Ok(bytes) = read_prefix(path, MAX_SRT_BYTES) else {
            continue;
        };
        let text = String::from_utf8_lossy(&bytes);
        let Some(ver) = parse_srt_version(&text) else {
            continue;
        };
        let mtime = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        if best.as_ref().is_none_or(|b| mtime > b.0) {
            best = Some((mtime, ver, path.to_path_buf()));
        }
    }
    if let Some((_, ver, path)) = best {
        contents.is_walksnail = true;
        contents.running_version = Some(ver);
        contents
            .signals
            .push(Signal::SrtFirmware { version: ver, path });
    }
}

fn read_prefix(path: &Path, max: usize) -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    let file = std::fs::File::open(path)?;
    let mut buf = Vec::with_capacity(max.min(8 * 1024));
    file.take(max as u64).read_to_end(&mut buf)?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    fn setup() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn detects_staged_img() {
        let dir = setup();
        fs::write(dir.path().join("AvatarX_Gnd_37.42.4.img"), b"firmware").unwrap();
        let c = scan(dir.path());
        assert!(c.is_walksnail);
        assert_eq!(c.variant, Some(Hardware::GogglesX));
        assert_eq!(c.staged_version, Some(Version::new(37, 42, 4)));
    }

    #[test]
    fn detects_userfont_dir() {
        let dir = setup();
        fs::create_dir(dir.path().join("userfont")).unwrap();
        let c = scan(dir.path());
        assert!(c.is_walksnail);
        assert!(c.signals.iter().any(|s| matches!(s, Signal::UserfontDir)));
    }

    #[test]
    fn detects_independ_upgrade_file() {
        let dir = setup();
        fs::write(dir.path().join("independ_upgrade.txt"), b"").unwrap();
        let c = scan(dir.path());
        assert!(c.is_walksnail);
    }

    #[test]
    fn detects_avatar_info_marker() {
        let dir = setup();
        fs::write(dir.path().join("Avatar_info.txt"), b"wifi=on\n").unwrap();
        let c = scan(dir.path());
        assert!(c.is_walksnail);
        assert!(c.signals.iter().any(
            |s| matches!(s, Signal::MarkerFile { name } if name.eq_ignore_ascii_case("Avatar_info.txt"))
        ));
    }

    #[test]
    fn extracts_running_version_from_srt() {
        let dir = setup();
        let dvr = dir.path().join("Movies");
        fs::create_dir(&dvr).unwrap();
        let mut f = fs::File::create(dvr.join("flight_001.srt")).unwrap();
        writeln!(
            f,
            "1\n00:00:00,000 --> 00:00:01,000\nSignal:99 FW:38.44.13 Volt:16.4V\n"
        )
        .unwrap();
        let c = scan(dir.path());
        assert_eq!(c.running_version, Some(Version::new(38, 44, 13)));
    }

    #[test]
    fn mixed_signals_track_independently() {
        let dir = setup();
        fs::write(dir.path().join("AvatarX_Gnd_37.42.4.img"), b"").unwrap();
        let dvr = dir.path().join("Movies");
        fs::create_dir(&dvr).unwrap();
        let mut f = fs::File::create(dvr.join("flight_001.srt")).unwrap();
        writeln!(
            f,
            "1\n00:00:00,000 --> 00:00:01,000\nFW:39.44.5 Volt:16.4V\n"
        )
        .unwrap();
        let c = scan(dir.path());
        assert_eq!(c.staged_version, Some(Version::new(37, 42, 4)));
        assert_eq!(c.running_version, Some(Version::new(39, 44, 5)));
        assert_eq!(c.variant, Some(Hardware::GogglesX));
    }

    #[test]
    fn non_walksnail_sd_returns_false() {
        let dir = setup();
        fs::write(dir.path().join("random.txt"), b"x").unwrap();
        let c = scan(dir.path());
        assert!(!c.is_walksnail);
        assert!(c.variant.is_none());
    }
}
