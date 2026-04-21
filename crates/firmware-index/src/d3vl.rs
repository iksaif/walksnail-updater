//! D3VL firmware mirror — the community-maintained primary source.
//!
//! Schema (observed April 2026):
//!
//! ```json
//! [
//!   {
//!     "badges": ["official", "beta", "latest"],
//!     "date": "2024-03-15",
//!     "version": "39.44.5",
//!     "notes": "…",
//!     "downloads": [
//!       {
//!         "btn": "Download Sky",
//!         "sha1": "…",
//!         "url":  "/dl/39.44.5/Avatar_Sky_39.44.5.img",
//!         "filename": "Avatar_Sky_39.44.5.img"
//!       }
//!     ]
//!   }
//! ]
//! ```

use anyhow::{Context, Result};
use chrono::NaiveDate;
use serde::Deserialize;

use super::{build_download, Channel, FirmwareRelease, Index, SourceLabel};

/// Public base used to resolve relative `url` fields in the upstream JSON.
pub const D3VL_BASE_URL: &str = "https://avatar-firmware.d3vl.com";

const JSON_URL: &str =
    "https://raw.githubusercontent.com/D3VL/Avatar-Firmware-Updates/main/firmwares.json";

#[derive(Debug, Deserialize)]
struct RawRelease {
    version: String,
    date: Option<String>,
    #[serde(default)]
    badges: Vec<String>,
    #[serde(default)]
    notes: String,
    #[serde(default)]
    downloads: Vec<RawDownload>,
}

#[derive(Debug, Deserialize)]
struct RawDownload {
    filename: String,
    url: String,
    #[serde(default)]
    sha1: String,
}

pub async fn fetch(client: &reqwest::Client) -> Result<Index> {
    let resp = client
        .get(JSON_URL)
        .send()
        .await
        .context("GET D3VL firmwares.json")?
        .error_for_status()
        .context("D3VL firmwares.json returned non-2xx")?;
    let raw: Vec<RawRelease> = resp.json().await.context("parsing D3VL JSON")?;
    let releases = raw.into_iter().filter_map(normalize_release).collect();
    Ok(Index {
        releases,
        source: SourceLabel::D3vl,
        fetched_at: chrono::Utc::now(),
    })
}

fn normalize_release(raw: RawRelease) -> Option<FirmwareRelease> {
    let version = raw.version.parse().ok()?;
    let date = raw
        .date
        .as_deref()
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
    let channel = classify_channel(&raw.badges);
    let downloads = raw
        .downloads
        .into_iter()
        .filter_map(|d| {
            let url = if d.url.starts_with("http") {
                d.url
            } else {
                format!("{D3VL_BASE_URL}{}", d.url)
            };
            build_download(d.filename, url, d.sha1.to_lowercase())
        })
        .collect::<Vec<_>>();
    if downloads.is_empty() {
        // Release has no flashable assets (e.g., only PDFs) — skip.
        return None;
    }
    Some(FirmwareRelease {
        version,
        date,
        channel,
        notes: raw.notes,
        downloads,
    })
}

fn classify_channel(badges: &[String]) -> Channel {
    let lowered: Vec<String> = badges.iter().map(|b| b.to_ascii_lowercase()).collect();
    if lowered.iter().any(|b| b == "beta") {
        Channel::Beta
    } else if lowered.iter().any(|b| b == "official" || b == "latest") {
        Channel::Stable
    } else {
        Channel::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_picks_channel_from_badges() {
        let r = RawRelease {
            version: "38.44.13".into(),
            date: Some("2024-01-15".into()),
            badges: vec!["official".into(), "beta".into()],
            notes: "notes".into(),
            downloads: vec![RawDownload {
                filename: "AvatarX_Gnd_38.44.13.img".into(),
                url: "/dl/38.44.13/AvatarX_Gnd_38.44.13.img".into(),
                sha1: "CAFE".into(),
            }],
        };
        let rel = normalize_release(r).unwrap();
        assert_eq!(rel.channel, Channel::Beta);
        assert_eq!(rel.downloads.len(), 1);
        assert!(rel.downloads[0].url.starts_with(D3VL_BASE_URL));
        assert_eq!(rel.downloads[0].sha1, "cafe");
    }

    #[test]
    fn normalize_skips_non_firmware_downloads() {
        let r = RawRelease {
            version: "34.40.15".into(),
            date: Some("2023-08-24".into()),
            badges: vec!["official".into()],
            notes: "x".into(),
            downloads: vec![
                RawDownload {
                    filename: "release_notes.pdf".into(),
                    url: "/dl/34.40.15/release_notes.pdf".into(),
                    sha1: String::new(),
                },
                RawDownload {
                    filename: "Avatar_Sky_34.40.15.img".into(),
                    url: "/dl/34.40.15/Avatar_Sky_34.40.15.img".into(),
                    sha1: String::new(),
                },
            ],
        };
        let rel = normalize_release(r).unwrap();
        assert_eq!(rel.downloads.len(), 1);
        assert_eq!(rel.downloads[0].filename, "Avatar_Sky_34.40.15.img");
    }

    #[test]
    fn normalize_drops_release_with_no_flashable_downloads() {
        let r = RawRelease {
            version: "1.2.3".into(),
            date: None,
            badges: vec!["official".into()],
            notes: String::new(),
            downloads: vec![RawDownload {
                filename: "release_notes.pdf".into(),
                url: "/x".into(),
                sha1: String::new(),
            }],
        };
        assert!(normalize_release(r).is_none());
    }
}
