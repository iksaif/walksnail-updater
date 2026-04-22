# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-04-21

### Added
- Collapsible flashing-procedure card at the top of each device's history view.
- `Avatar_info.txt` recognised as a generic Walksnail-SD marker.
- Manual "Pick SD card…" picker unlocks staging for any hardware even when
  the SD isn't auto-detected.
- Blocking loading dialog during the first `walksnail.app` index fetch so
  the home grid doesn't flash empty.
- Hardware silhouettes redrawn per device (round Moonlight puck, rectangular
  Goggles X lens windows, VRX with screen + keypad, antenna-tower Relay, etc).
- Nav bar with Home / Settings / About icons; folder-pick icon is coloured.
- Downloadable directory is user-configurable from Settings.
- Smoother progress bar with `requestAnimationFrame` tween, indeterminate
  shimmer when the source omits `Content-Length`, and live transfer rate.
- "Downloaded" badge replaces the in-flight Download button for cached
  firmware; a coloured Reveal button opens the file manager.
- Dependency refresh: `dirs 6`, `sha1 0.11`, `sysinfo 0.38`, `thiserror 2`,
  `pdf-extract 0.10`, GitHub Actions `checkout@v6` / `setup-node@v6` /
  `create-pull-request@v8`.

### Changed
- `instructions.json` is now hand-curated with corrected per-device steps
  (Link-button hold times, USB mass-storage flashing for the Relay, etc)
  instead of parsed from PDFs. `fetched_at` / `manual_sha256` dropped.
- `firmware-index` primary source is `walksnail.app` (covers 2024+ releases
  including Goggles X / Goggles L / Moonlight); D3VL JSON + CADDXFPV
  scraper stay as fallbacks.
- Release workflow: Apple / Windows signing env only exported when the
  secret is non-empty; unsigned builds no longer trip `security import`.
- Dark-theme readability: muted text lightened from slate-600 to slate-400,
  new `brand.dim` for secondary body text on dark panels.
- Licence switched to MIT; commit author email normalised.

### Removed
- `crates/manuals`, the `refresh-manuals.yml` workflow, and cached PDF
  manuals. The Tauri app never parses PDFs.
- Deprecated `macos-13` runner entry from the CI matrix.

## [0.1.0] - 2026-04-21

### Added
- Initial implementation: cross-platform Tauri 2 desktop app (macOS, Windows, Linux).
- Firmware index fetch from D3VL mirror with CADDXFPV scraper fallback.
- Removable-drive watcher + multi-signal SD card scan.
- Firmware staging pipeline with SHA1 verification and safety gates.
- Per-device firmware-upgrade instructions extracted from upstream PDFs.
- Past-version history with collapsible changelogs.
- First-run onboarding with "not affiliated" acknowledgement.
