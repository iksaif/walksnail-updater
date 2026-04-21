//! Fetches, normalizes, and caches the Walksnail firmware index.
//!
//! Primary source: the community-maintained JSON index at
//! <https://github.com/D3VL/Avatar-Firmware-Updates>. Secondary: the CADDXFPV
//! download center — scraping-based and brittle, so we only use it as a
//! fallback when the primary source is unreachable or stale.
//!
//! The [`fetch`] entry point returns a fully normalized [`Index`] ready for
//! consumption by the UI, without exposing the upstream idiosyncrasies.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::NaiveDate;
use hardware::{parse_filename, Hardware, Version};
use serde::{Deserialize, Serialize};
use tracing::warn;

mod d3vl;
mod official;
mod walksnail_app;

pub use d3vl::D3VL_BASE_URL;

/// Our normalized view of one firmware release.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareRelease {
    pub version: Version,
    /// Release date from upstream (`YYYY-MM-DD`) when known.
    pub date: Option<NaiveDate>,
    pub channel: Channel,
    pub notes: String,
    /// One entry per supported hardware variant. Non-firmware assets (release
    /// notes PDFs, font zips) are filtered out upstream so every entry here is
    /// a flashable `.img`.
    pub downloads: Vec<Download>,
}

/// Release-notes / font zips / "beta" / "stable" channels come from upstream
/// badges. We collapse them to a simple enum for the UI to filter on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    Stable,
    Beta,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Download {
    pub hardware: Hardware,
    pub filename: String,
    pub url: String,
    /// Upstream publishes SHA1; we verify downloads against it.
    pub sha1: String,
}

/// Normalized index — the full history, newest first within each hardware.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Index {
    pub releases: Vec<FirmwareRelease>,
    /// Where the index came from — shown in the UI for transparency.
    pub source: SourceLabel,
    /// When we fetched it (UTC). Used for cache staleness + display.
    pub fetched_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceLabel {
    WalksnailApp,
    D3vl,
    Official,
    Cache,
}

impl Index {
    /// Latest stable release that supports `hardware`, if any.
    pub fn latest_stable_for(&self, hardware: Hardware) -> Option<&FirmwareRelease> {
        self.releases_for(hardware)
            .filter(|r| r.channel == Channel::Stable)
            .max_by_key(|r| r.version)
    }

    /// Every release that has a download for `hardware`, newest first.
    pub fn releases_for(&self, hardware: Hardware) -> impl Iterator<Item = &FirmwareRelease> {
        let mut v: Vec<&FirmwareRelease> = self
            .releases
            .iter()
            .filter(move |r| r.downloads.iter().any(|d| d.hardware == hardware))
            .collect();
        v.sort_by_key(|r| std::cmp::Reverse(r.version));
        v.into_iter()
    }
}

/// User-facing source preference. The default (`Auto`) tries walksnail.app
/// (most complete, current), then falls back to D3VL's checked-in JSON
/// (stale but offline-friendly), then the official CADDXFPV scraper.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    #[default]
    Auto,
    WalksnailAppOnly,
    D3vlOnly,
    OfficialOnly,
}

/// Fetch the normalized index from the network. On total failure the caller
/// can fall back to a cached [`Index`] via [`load_cache`].
pub async fn fetch(client: &reqwest::Client, source: Source) -> Result<Index> {
    match source {
        Source::WalksnailAppOnly => walksnail_app::fetch(client).await,
        Source::D3vlOnly => d3vl::fetch(client).await,
        Source::OfficialOnly => official::fetch(client).await,
        Source::Auto => match walksnail_app::fetch(client).await {
            Ok(i) if !i.releases.is_empty() => Ok(i),
            Ok(_) => {
                warn!("walksnail.app returned no releases, trying D3VL JSON");
                try_chain(client).await
            }
            Err(e) => {
                warn!(error = %e, "walksnail.app fetch failed, trying D3VL JSON");
                try_chain(client).await
            }
        },
    }
}

async fn try_chain(client: &reqwest::Client) -> Result<Index> {
    match d3vl::fetch(client).await {
        Ok(i) if !i.releases.is_empty() => Ok(i),
        Ok(_) => {
            warn!("D3VL JSON empty, falling back to CADDXFPV scraper");
            official::fetch(client).await
        }
        Err(e) => {
            warn!(error = %e, "D3VL fetch failed, falling back to CADDXFPV scraper");
            official::fetch(client).await
        }
    }
}

/// Write the index to disk as JSON.
pub async fn save_cache(path: &Path, index: &Index) -> Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.ok();
    }
    let bytes = serde_json::to_vec_pretty(index).context("serializing index")?;
    tokio::fs::write(path, bytes)
        .await
        .with_context(|| format!("writing index cache to {}", path.display()))?;
    Ok(())
}

/// Read a previously-saved index from disk. Returns an `Err` if the file is
/// absent or corrupted — callers should treat that as "no cache" and keep
/// going.
pub async fn load_cache(path: &Path) -> Result<Index> {
    let bytes = tokio::fs::read(path)
        .await
        .with_context(|| format!("reading cache at {}", path.display()))?;
    let mut index: Index = serde_json::from_slice(&bytes).context("parsing cached index JSON")?;
    index.source = SourceLabel::Cache;
    Ok(index)
}

/// Helper: turn a filename + url into a [`Download`], skipping files that
/// aren't recognized firmware `.img` files (release notes, font packs, etc).
pub(crate) fn build_download(filename: String, url: String, sha1: String) -> Option<Download> {
    let (hardware, _version) = parse_filename(&filename)?;
    Some(Download {
        hardware,
        filename,
        url,
        sha1,
    })
}

/// Default cache file location relative to the caller-provided data dir.
pub fn cache_path(data_dir: &Path) -> PathBuf {
    data_dir.join("index.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_index() -> Index {
        let release = |ver: &str, channel: Channel, hw: Hardware| FirmwareRelease {
            version: ver.parse().unwrap(),
            date: NaiveDate::from_ymd_opt(2024, 1, 1),
            channel,
            notes: format!("Release {ver}"),
            downloads: vec![Download {
                hardware: hw,
                filename: hw.canonical_filename(ver.parse().unwrap()),
                url: format!("https://example/{ver}"),
                sha1: "deadbeef".into(),
            }],
        };
        Index {
            releases: vec![
                release("37.42.4", Channel::Stable, Hardware::GogglesX),
                release("39.44.5", Channel::Stable, Hardware::GogglesX),
                release("40.0.0", Channel::Beta, Hardware::GogglesX),
                release("38.44.13", Channel::Stable, Hardware::AvatarSky),
            ],
            source: SourceLabel::D3vl,
            fetched_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn latest_stable_picks_highest_stable_for_hardware() {
        let idx = sample_index();
        let latest = idx.latest_stable_for(Hardware::GogglesX).unwrap();
        assert_eq!(latest.version, Version::new(39, 44, 5));
    }

    #[test]
    fn latest_stable_ignores_beta() {
        let idx = sample_index();
        let latest = idx.latest_stable_for(Hardware::GogglesX).unwrap();
        assert_ne!(latest.channel, Channel::Beta);
    }

    #[test]
    fn releases_for_returns_newest_first() {
        let idx = sample_index();
        let got: Vec<Version> = idx
            .releases_for(Hardware::GogglesX)
            .map(|r| r.version)
            .collect();
        assert_eq!(
            got,
            vec![
                Version::new(40, 0, 0),
                Version::new(39, 44, 5),
                Version::new(37, 42, 4)
            ]
        );
    }

    #[test]
    fn build_download_rejects_non_firmware() {
        assert!(build_download("release_notes.pdf".into(), String::new(), String::new()).is_none());
        assert!(build_download("userfont.zip".into(), String::new(), String::new()).is_none());
    }

    #[test]
    fn build_download_accepts_canonical_img() {
        let dl = build_download(
            "AvatarX_Gnd_39.44.5.img".into(),
            "https://example/x".into(),
            "abc123".into(),
        )
        .unwrap();
        assert_eq!(dl.hardware, Hardware::GogglesX);
    }

    #[tokio::test]
    async fn cache_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = cache_path(dir.path());
        let idx = sample_index();
        save_cache(&path, &idx).await.unwrap();
        let loaded = load_cache(&path).await.unwrap();
        assert_eq!(loaded.releases.len(), idx.releases.len());
        assert_eq!(loaded.source, SourceLabel::Cache);
    }
}
