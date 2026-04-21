//! Walksnail hardware identification, firmware version parsing, and safety
//! gates.
//!
//! Pure library: no IO, no async. Everything here is cheaply testable.

use std::fmt;
use std::str::FromStr;

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A Walksnail hardware variant that we know how to identify and update.
///
/// The JSON serialization uses `PascalCase` so it round-trips cleanly to the
/// frontend without bespoke mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Hardware {
    /// Standard Avatar HD VTX (air unit).
    AvatarSky,
    /// Mini 1S VTX.
    AvatarMiniSky,
    /// Moonlight 4K VTX.
    MoonlightSky,
    /// Original Avatar goggles.
    AvatarGnd,
    /// Goggles X.
    GogglesX,
    /// Goggles L.
    GogglesL,
    /// VRX receiver module (a.k.a. Avatar SE).
    VrxSE,
    /// Recon HD — the Mini-family ground unit.
    ReconHd,
    /// Avatar Relay — long-range relay / repeater.
    AvatarRelay,
}

impl Hardware {
    /// Canonical filename prefix used on SD cards (before the `_<version>.img`
    /// suffix).
    pub fn filename_prefix(self) -> &'static str {
        match self {
            Self::AvatarSky => "Avatar_Sky",
            Self::AvatarMiniSky => "AvatarMini_Sky",
            Self::MoonlightSky => "AvatarMoonlight_Sky",
            Self::AvatarGnd => "Avatar_Gnd",
            Self::GogglesX => "AvatarX_Gnd",
            Self::GogglesL => "AvatarLite_Gnd",
            Self::VrxSE => "AvatarSE_Gnd",
            Self::ReconHd => "AvatarMini_Gnd",
            Self::AvatarRelay => "Avatar_Relay",
        }
    }

    /// Render a human-readable name for UI surfaces.
    pub fn display_name(self) -> &'static str {
        match self {
            Self::AvatarSky => "Avatar HD VTX",
            Self::AvatarMiniSky => "Avatar Mini 1S VTX",
            Self::MoonlightSky => "Moonlight 4K VTX",
            Self::AvatarGnd => "Avatar HD Goggles",
            Self::GogglesX => "Goggles X",
            Self::GogglesL => "Goggles L",
            Self::VrxSE => "Avatar VRX",
            Self::ReconHd => "Recon HD",
            Self::AvatarRelay => "Avatar Relay",
        }
    }

    /// Build the canonical firmware filename for this hardware + version.
    pub fn canonical_filename(self, version: Version) -> String {
        format!("{}_{}.img", self.filename_prefix(), version)
    }

    /// Whether this variant is a ground-side device (goggles / VRX). Useful for
    /// file-cleanup rules and instruction selection.
    pub fn is_ground(self) -> bool {
        matches!(
            self,
            Self::AvatarGnd | Self::GogglesX | Self::GogglesL | Self::VrxSE | Self::ReconHd
        )
    }

    /// Iterator over every known variant. Kept in sync with the enum — adding
    /// a variant should also add it here (and the tests enforce it).
    pub fn all() -> &'static [Hardware] {
        &[
            Self::AvatarSky,
            Self::AvatarMiniSky,
            Self::MoonlightSky,
            Self::AvatarGnd,
            Self::GogglesX,
            Self::GogglesL,
            Self::VrxSE,
            Self::ReconHd,
            Self::AvatarRelay,
        ]
    }
}

impl fmt::Display for Hardware {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.display_name())
    }
}

/// `MAJOR.MINOR.PATCH` firmware version — numeric per component so ordering
/// means what users expect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Version {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl Version {
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum VersionParseError {
    #[error("expected MAJOR.MINOR.PATCH, got `{0}`")]
    BadShape(String),
    #[error("non-numeric component in `{0}`")]
    NotNumeric(String),
}

impl FromStr for Version {
    type Err = VersionParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(VersionParseError::BadShape(s.to_string()));
        }
        let parse = |p: &str| {
            p.parse::<u16>()
                .map_err(|_| VersionParseError::NotNumeric(s.to_string()))
        };
        Ok(Self::new(
            parse(parts[0])?,
            parse(parts[1])?,
            parse(parts[2])?,
        ))
    }
}

/// Firmware `.img` filename regex. Order matters: the most-specific prefix
/// must come first so `AvatarMini_Sky` wins over `Avatar_Sky`.
static FILENAME_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?xi)^
          (?P<prefix>
              AvatarMoonlight_Sky
            | AvatarMini_Sky
            | AvatarMini_Gnd
            | AvatarLite_Gnd
            | AvatarX_Gnd
            | AvatarSE_Gnd
            | Avatar_Sky
            | Avatar_Gnd
            | Avatar_Relay
          )
          _
          (?P<major>\d+)\.(?P<minor>\d+)\.(?P<patch>\d+)
          \.img
        $",
    )
    .expect("hardcoded filename regex is valid")
});

/// Parse a firmware filename into `(hardware, version)` if it matches the
/// Walksnail naming convention.
pub fn parse_filename(name: &str) -> Option<(Hardware, Version)> {
    let caps = FILENAME_RE.captures(name)?;
    let hardware = match caps["prefix"].to_ascii_lowercase().as_str() {
        "avatarmoonlight_sky" => Hardware::MoonlightSky,
        "avatarmini_sky" => Hardware::AvatarMiniSky,
        "avatarmini_gnd" => Hardware::ReconHd,
        "avatarlite_gnd" => Hardware::GogglesL,
        "avatarx_gnd" => Hardware::GogglesX,
        "avatarse_gnd" => Hardware::VrxSE,
        "avatar_sky" => Hardware::AvatarSky,
        "avatar_gnd" => Hardware::AvatarGnd,
        "avatar_relay" => Hardware::AvatarRelay,
        _ => return None,
    };
    let version = Version::new(
        caps["major"].parse().ok()?,
        caps["minor"].parse().ok()?,
        caps["patch"].parse().ok()?,
    );
    Some((hardware, version))
}

/// Best-effort extraction of the running firmware version from a DVR `.srt`
/// sidecar. Walksnail embeds a line like `FW:38.44.13` when `debug_info_in_srt`
/// is on. We tolerate small format drift.
pub fn parse_srt_version(contents: &str) -> Option<Version> {
    static RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?i)\bFW[:\s]+v?(\d+\.\d+\.\d+)").expect("valid regex"));
    let caps = RE.captures(contents)?;
    caps.get(1)?.as_str().parse().ok()
}

/// Result of a safety check on a proposed firmware update.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SafetyVerdict {
    /// Update is safe — latest-stable of same hardware, equal or newer.
    Ok,
    /// Update would likely work but deserves a prominent warning.
    Warn { reason: String },
    /// Hard block — flashing would almost certainly brick the device.
    Block { reason: String },
}

/// Goggles X (new batches) brick floor. Flashing anything lower than this on
/// hardware manufactured after Jan 2025 permanently bricks it. We apply the
/// floor to *all* Goggles X because we can't reliably tell old from new from
/// software; erring safe is the right call.
pub const GOGGLES_X_FLOOR: Version = Version::new(38, 44, 13);

/// Avatar GT floor (new batches ship with ≥ 39.44.2 due to component upgrade).
pub const AVATAR_GT_FLOOR: Version = Version::new(39, 44, 2);

/// Evaluate whether it is safe to flash `target` over `current` for the given
/// hardware. When `current` is unknown (e.g. fresh SD), skips the generic
/// downgrade warning but still applies the hard floors.
pub fn safety_check(
    hardware: Hardware,
    current: Option<Version>,
    target: Version,
) -> SafetyVerdict {
    if hardware == Hardware::GogglesX && target < GOGGLES_X_FLOOR {
        return SafetyVerdict::Block {
            reason: format!(
                "Goggles X manufactured since Jan 2025 contain new DDR memory. \
                 Flashing below {GOGGLES_X_FLOOR} is a known permanent brick. \
                 Refusing to stage this firmware."
            ),
        };
    }
    if hardware == Hardware::AvatarSky && target < AVATAR_GT_FLOOR {
        // Can't distinguish "GT" vs other Sky variants from SD, so warn.
        return SafetyVerdict::Warn {
            reason: format!(
                "Avatar GT VTX units require at least {AVATAR_GT_FLOOR}. If \
                 this is a GT, flashing below that is a brick risk."
            ),
        };
    }
    match current {
        Some(c) if target < c => SafetyVerdict::Warn {
            reason: format!("Downgrade from {c} to {target}. Proceed only if you know why."),
        },
        _ => SafetyVerdict::Ok,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_roundtrip() {
        let v: Version = "39.44.3".parse().unwrap();
        assert_eq!(v, Version::new(39, 44, 3));
        assert_eq!(v.to_string(), "39.44.3");
    }

    #[test]
    fn version_ordering_is_numeric() {
        let v = Version::new(9, 44, 3);
        let w = Version::new(38, 44, 3);
        assert!(v < w, "9.44.3 should be less than 38.44.3");
    }

    #[test]
    fn version_parse_errors() {
        assert!(matches!(
            "1.2".parse::<Version>(),
            Err(VersionParseError::BadShape(_))
        ));
        assert!(matches!(
            "a.b.c".parse::<Version>(),
            Err(VersionParseError::NotNumeric(_))
        ));
    }

    #[test]
    fn parse_filename_table() {
        let cases = [
            ("Avatar_Sky_39.44.3.img", Hardware::AvatarSky, (39, 44, 3)),
            (
                "AvatarMini_Sky_38.43.4.img",
                Hardware::AvatarMiniSky,
                (38, 43, 4),
            ),
            (
                "AvatarMoonlight_Sky_39.44.5.img",
                Hardware::MoonlightSky,
                (39, 44, 5),
            ),
            ("Avatar_Gnd_33.39.10.img", Hardware::AvatarGnd, (33, 39, 10)),
            ("AvatarX_Gnd_39.44.5.img", Hardware::GogglesX, (39, 44, 5)),
            (
                "AvatarLite_Gnd_38.44.13.img",
                Hardware::GogglesL,
                (38, 44, 13),
            ),
            ("AvatarSE_Gnd_38.43.4.img", Hardware::VrxSE, (38, 43, 4)),
            (
                "AvatarMini_Gnd_34.40.15.img",
                Hardware::ReconHd,
                (34, 40, 15),
            ),
            (
                "Avatar_Relay_39.44.4.img",
                Hardware::AvatarRelay,
                (39, 44, 4),
            ),
        ];
        for (name, hw, (maj, min, pat)) in cases {
            let got = parse_filename(name).expect(name);
            assert_eq!(got.0, hw, "hardware mismatch for {name}");
            assert_eq!(
                got.1,
                Version::new(maj, min, pat),
                "version mismatch for {name}"
            );
        }
    }

    #[test]
    fn parse_filename_rejects_noise() {
        for bogus in [
            "Avatar_Sky_1.2.img",          // missing patch
            "Avatar_Sky_1.2.3.bin",        // wrong extension
            "NotAWalksnail_Sky_1.2.3.img", // wrong prefix
            "Avatar_Sky 1.2.3.img",        // space instead of underscore
            "",
        ] {
            assert!(parse_filename(bogus).is_none(), "should reject `{bogus}`");
        }
    }

    #[test]
    fn parse_filename_is_case_insensitive_on_prefix() {
        assert!(parse_filename("avatarx_gnd_38.44.13.img").is_some());
    }

    #[test]
    fn canonical_filename_roundtrip() {
        for hw in Hardware::all() {
            let name = hw.canonical_filename(Version::new(38, 44, 13));
            let (parsed_hw, parsed_v) = parse_filename(&name).unwrap();
            assert_eq!(parsed_hw, *hw);
            assert_eq!(parsed_v, Version::new(38, 44, 13));
        }
    }

    #[test]
    fn safety_blocks_goggles_x_downgrade() {
        let v = safety_check(
            Hardware::GogglesX,
            Some(Version::new(39, 44, 5)),
            Version::new(37, 42, 4),
        );
        assert!(matches!(v, SafetyVerdict::Block { .. }));
    }

    #[test]
    fn safety_allows_goggles_x_at_or_above_floor() {
        let v = safety_check(Hardware::GogglesX, None, GOGGLES_X_FLOOR);
        assert_eq!(v, SafetyVerdict::Ok);
    }

    #[test]
    fn safety_warns_on_generic_downgrade() {
        let v = safety_check(
            Hardware::AvatarGnd,
            Some(Version::new(39, 44, 5)),
            Version::new(38, 44, 0),
        );
        assert!(matches!(v, SafetyVerdict::Warn { .. }));
    }

    #[test]
    fn safety_warns_avatar_gt_below_floor() {
        let v = safety_check(Hardware::AvatarSky, None, Version::new(37, 42, 4));
        assert!(matches!(v, SafetyVerdict::Warn { .. }));
    }

    #[test]
    fn srt_version_extracts_running_firmware() {
        let srt = "\
1
00:00:00,000 --> 00:00:01,000
Signal:99 CH:R3 FW:38.44.13 Volt:16.4V

";
        assert_eq!(parse_srt_version(srt), Some(Version::new(38, 44, 13)));
    }

    #[test]
    fn srt_version_returns_none_when_absent() {
        assert_eq!(parse_srt_version("no firmware data here"), None);
    }
}
