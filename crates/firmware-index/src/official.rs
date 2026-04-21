//! Fallback scraper for the official CADDXFPV download center.
//!
//! The official page has no JSON API and the DOM structure is not published,
//! so scraping is inherently brittle. We keep the extraction narrow: each
//! candidate `<a>` link with an `href` ending in `.img` is inspected, the
//! filename parsed by `hardware::parse_filename`, and if that succeeds we
//! build a [`FirmwareRelease`] from it.
//!
//! Because the page does not surface SHA1 hashes for each build, downloads
//! sourced here have an empty `sha1` — callers should treat that as "no
//! integrity check available" and warn the user accordingly.

use anyhow::{Context, Result};
use chrono::NaiveDate;
use hardware::parse_filename;
use once_cell::sync::Lazy;
use regex::Regex;
use tracing::debug;

use super::{Channel, Download, FirmwareRelease, Index, SourceLabel};

const OFFICIAL_URL: &str = "https://www.caddxfpv.com/pages/download-center";

pub async fn fetch(client: &reqwest::Client) -> Result<Index> {
    let html = client
        .get(OFFICIAL_URL)
        .send()
        .await
        .context("GET CADDXFPV download center")?
        .error_for_status()
        .context("CADDXFPV returned non-2xx")?
        .text()
        .await
        .context("reading CADDXFPV body")?;
    let releases = parse_links(&html);
    Ok(Index {
        releases,
        source: SourceLabel::Official,
        fetched_at: chrono::Utc::now(),
    })
}

/// Extract every `<a href="…*.img">` link and group them by firmware version.
///
/// We intentionally don't bring in a full HTML parser — the page is large and
/// we only need hrefs. A narrow regex is more predictable across layout
/// tweaks than querying a shifting DOM.
fn parse_links(html: &str) -> Vec<FirmwareRelease> {
    static HREF_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r#"href=["']([^"']+\.img)["']"#).expect("valid regex"));
    let mut by_version: std::collections::BTreeMap<String, Vec<Download>> = Default::default();
    for caps in HREF_RE.captures_iter(html) {
        let href = &caps[1];
        let filename = href.rsplit('/').next().unwrap_or(href).to_string();
        let Some((hardware, version)) = parse_filename(&filename) else {
            continue;
        };
        let url = absolutize(href);
        by_version
            .entry(version.to_string())
            .or_default()
            .push(Download {
                hardware,
                filename,
                url,
                sha1: String::new(),
            });
    }
    debug!("official scrape yielded {} versions", by_version.len());
    by_version
        .into_iter()
        .filter_map(|(v, downloads)| {
            let version = v.parse().ok()?;
            Some(FirmwareRelease {
                version,
                date: None,
                channel: Channel::Stable,
                notes: "(sourced from CADDXFPV download center)".into(),
                downloads,
            })
        })
        .collect()
}

fn absolutize(href: &str) -> String {
    if href.starts_with("http") {
        href.to_string()
    } else if let Some(stripped) = href.strip_prefix("//") {
        format!("https://{stripped}")
    } else if href.starts_with('/') {
        format!("https://www.caddxfpv.com{href}")
    } else {
        href.to_string()
    }
}

// Rustc doesn't warn on unused items in `pub(crate)` paths with cfg, so silence:
#[allow(dead_code)]
fn _silence_unused(_: NaiveDate) {}

#[cfg(test)]
mod tests {
    use super::*;
    use hardware::Hardware;

    #[test]
    fn parse_links_groups_by_version() {
        let html = r#"
            <a href="https://cdn.example/Avatar_Sky_39.44.5.img">Sky</a>
            <a href='/dl/AvatarX_Gnd_39.44.5.img'>Goggles X</a>
            <a href="//cdn.example/AvatarLite_Gnd_38.44.13.img">Goggles L</a>
            <a href="/dl/release_notes.pdf">Notes</a>
        "#;
        let rels = parse_links(html);
        assert_eq!(rels.len(), 2);
        let r = rels
            .iter()
            .find(|r| r.version.to_string() == "39.44.5")
            .unwrap();
        assert_eq!(r.downloads.len(), 2);
        assert!(r
            .downloads
            .iter()
            .any(|d| d.hardware == Hardware::AvatarSky));
        assert!(r.downloads.iter().any(|d| d.hardware == Hardware::GogglesX));
        assert!(r.downloads.iter().all(|d| d.url.starts_with("http")));
    }

    #[test]
    fn parse_links_ignores_non_img_hrefs() {
        let html = r#"<a href="notes.pdf">x</a><a href="foo.txt">y</a>"#;
        assert!(parse_links(html).is_empty());
    }
}
