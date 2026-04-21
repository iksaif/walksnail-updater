//! Primary firmware source: scrape <https://walksnail.app/firmware>.
//!
//! `walksnail.app` is the D3VL-run community mirror's live web UI. Unlike the
//! stale `firmwares.json` on GitHub (last updated August 2023), this site has
//! every release through today, including hardware variants that didn't exist
//! two years ago (Goggles X, Goggles L, Avatar Relay, etc.).
//!
//! There is no JSON API — the site is Remix-rendered HTML. We scrape it in
//! two passes:
//!
//! 1. `GET /firmware` — lists every release as `/firmware/avatar/<ver>` links
//!    with a visible date next to each.
//! 2. `GET /firmware/avatar/<ver>` — every firmware `.img` URL on
//!    `download.walksnail.app/<uuid>/<filename>?download` plus the release
//!    notes and channel badge.
//!
//! Detail pages are fetched concurrently. We don't publish SHA1 because the
//! page doesn't expose one; the stage pipeline treats missing hashes as "no
//! integrity check" rather than blocking.

use std::collections::HashMap;

use anyhow::{Context, Result};
use chrono::NaiveDate;
use futures_util::{stream, StreamExt};
use hardware::parse_filename;
use once_cell::sync::Lazy;
use regex::Regex;
use tracing::{debug, warn};

use super::{Channel, Download, FirmwareRelease, Index, SourceLabel};

const BASE: &str = "https://walksnail.app";
const INDEX_URL: &str = "https://walksnail.app/firmware";
const MAX_CONCURRENT_DETAIL_FETCHES: usize = 8;

pub async fn fetch(client: &reqwest::Client) -> Result<Index> {
    let listing = client
        .get(INDEX_URL)
        .send()
        .await
        .context("GET walksnail.app/firmware")?
        .error_for_status()
        .context("walksnail.app/firmware non-2xx")?
        .text()
        .await
        .context("reading walksnail.app/firmware body")?;

    let stubs = parse_listing(&listing);
    debug!(count = stubs.len(), "walksnail.app listing parsed");

    let releases: Vec<FirmwareRelease> = stream::iter(stubs.into_iter().map(|stub| {
        let client = client.clone();
        async move { fetch_detail(&client, stub).await }
    }))
    .buffer_unordered(MAX_CONCURRENT_DETAIL_FETCHES)
    .filter_map(|r| async move {
        match r {
            Ok(release) => Some(release),
            Err(e) => {
                warn!(error = %e, "release detail fetch failed");
                None
            }
        }
    })
    .collect()
    .await;

    Ok(Index {
        releases,
        source: SourceLabel::WalksnailApp,
        fetched_at: chrono::Utc::now(),
    })
}

/// Stub row extracted from the listing page — just enough to pivot to the
/// detail page.
#[derive(Debug, Clone)]
struct Stub {
    version: String,
    date: Option<NaiveDate>,
    channel: Channel,
    slug: String, // e.g. "avatar/39.44.5"
}

fn parse_listing(html: &str) -> Vec<Stub> {
    // Each listing row looks roughly like:
    //   <a href="/firmware/avatar/39.44.5" …>…  <span>Official</span> <span>Beta</span>
    //   …<time datetime="2024-09-11">…</time>…</a>
    static LINK_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r#"/firmware/(?P<slug>[a-z0-9]+/[0-9][0-9.]*)"#).expect("valid regex")
    });
    let mut seen = HashMap::<String, Stub>::new();
    for caps in LINK_RE.captures_iter(html) {
        let slug = caps["slug"].to_string();
        let version = slug.rsplit('/').next().unwrap_or_default().to_string();
        if version.is_empty() {
            continue;
        }
        seen.entry(slug.clone()).or_insert_with(|| Stub {
            version,
            date: None,
            channel: Channel::Unknown,
            slug,
        });
    }
    // Annotate with nearest date + badge where we can see them. Best-effort:
    // the listing puts a `<time>` element right before each link in the DOM.
    static TIME_NEAR_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(
            r#"(?is)<time[^>]*datetime="(?P<date>\d{4}-\d{2}-\d{2})"[^>]*>.*?/firmware/(?P<slug>[a-z0-9]+/[0-9][0-9.]*)"#,
        )
        .expect("valid regex")
    });
    for caps in TIME_NEAR_RE.captures_iter(html) {
        let slug = caps["slug"].to_string();
        if let Some(stub) = seen.get_mut(&slug) {
            stub.date = NaiveDate::parse_from_str(&caps["date"], "%Y-%m-%d").ok();
        }
    }
    // Badge sniffing: walk the document once more, and for each
    // `/firmware/<slug>` match inspect the text in a ~400 char window
    // after the link. Rust's default regex engine doesn't support
    // look-around, so we do the windowing manually.
    for caps in LINK_RE.captures_iter(html) {
        let slug = caps["slug"].to_string();
        let start = caps.get(0).map(|m| m.end()).unwrap_or(0);
        let end = (start + 400).min(html.len());
        let window = html[start..end].to_lowercase();
        // Don't let the window spill into the *next* firmware link.
        let trimmed = match window.find("/firmware/") {
            Some(idx) => &window[..idx],
            None => window.as_str(),
        };
        if let Some(stub) = seen.get_mut(&slug) {
            if trimmed.contains("beta") || trimmed.contains("pre-release") {
                stub.channel = Channel::Beta;
            } else if trimmed.contains("official")
                || trimmed.contains("latest")
                || trimmed.contains("stable")
            {
                stub.channel = Channel::Stable;
            }
        }
    }
    // Default: treat unannotated releases as stable. The detail page confirms
    // this via its own badge; if we later read the detail we'll refine.
    for stub in seen.values_mut() {
        if stub.channel == Channel::Unknown {
            stub.channel = Channel::Stable;
        }
    }
    let mut out: Vec<Stub> = seen.into_values().collect();
    // Newest first in the list helps the UI show the latest without an extra
    // sort in the happy path.
    out.sort_by(|a, b| b.version.cmp(&a.version));
    out
}

async fn fetch_detail(client: &reqwest::Client, stub: Stub) -> Result<FirmwareRelease> {
    let url = format!("{BASE}/firmware/{}", stub.slug);
    let html = client
        .get(&url)
        .send()
        .await
        .with_context(|| format!("GET {url}"))?
        .error_for_status()
        .with_context(|| format!("non-2xx from {url}"))?
        .text()
        .await
        .with_context(|| format!("reading body {url}"))?;

    let downloads = parse_download_links(&html);
    let notes = extract_notes(&html).unwrap_or_default();
    // Detail-page header carries channel + date verbatim; prefer those over
    // the listing heuristics when we find them.
    let channel = refine_channel(&html, stub.channel);
    let date = extract_detail_date(&html).or(stub.date);

    let version = stub
        .version
        .parse()
        .with_context(|| format!("parsing version `{}`", stub.version))?;
    Ok(FirmwareRelease {
        version,
        date,
        channel,
        notes,
        downloads,
    })
}

fn parse_download_links(html: &str) -> Vec<Download> {
    static DL_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(
            r#"href="(?P<url>https://download\.walksnail\.app/[^"]+/(?P<filename>[^/"?]+\.img)(?:\?[^"]*)?)""#,
        )
        .expect("valid regex")
    });
    let mut out = Vec::new();
    let mut seen_filenames = std::collections::HashSet::new();
    for caps in DL_RE.captures_iter(html) {
        let filename = caps["filename"].to_string();
        if !seen_filenames.insert(filename.clone()) {
            continue;
        }
        let url = caps["url"].to_string();
        let Some((hardware, _)) = parse_filename(&filename) else {
            continue;
        };
        out.push(Download {
            hardware,
            filename,
            url,
            // walksnail.app does not publish per-file SHA1 hashes on the
            // detail page. Leaving this empty signals the stage pipeline to
            // skip integrity verification, which the UI surfaces as a
            // notice.
            sha1: String::new(),
        });
    }
    out
}

fn extract_notes(html: &str) -> Option<String> {
    // Two-step so we don't have to cram heading matching + body windowing
    // into one regex with nested greedy quantifiers:
    //   1. Walk every <h1..h6>…</h1..h6> match.
    //   2. If its (unescaped, lowercased) title names a changelog-like
    //      section, the body is everything from the end of that heading to
    //      the next heading at the *same or lower* numeric level (so sub-
    //      headings like <h4>Bug Fix</h4> stay inside the block), capped at
    //      6 KB.
    static H_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r#"(?is)<h(?P<level>[1-6])[^>]*>(?P<title>[^<]{0,200})</h[1-6]>"#)
            .expect("valid regex")
    });
    for caps in H_RE.captures_iter(html) {
        let title = html_unescape(caps.name("title")?.as_str()).to_lowercase();
        let matches = (title.contains("what") && title.contains("new"))
            || title.contains("changelog")
            || title.contains("release note");
        if !matches {
            continue;
        }
        let level: u32 = caps["level"].parse().ok()?;
        let whole = caps.get(0)?;
        let body_start = whole.end();
        // Build a "next heading at level ≤ current" boundary regex.
        let max_level = level.max(1);
        let boundary = format!(r#"(?is)<h[1-{max_level}][^>\s]*[^>]*>"#);
        let body_end = Regex::new(&boundary)
            .ok()
            .and_then(|re| re.find(&html[body_start..]).map(|m| body_start + m.start()))
            .unwrap_or_else(|| (body_start + 6000).min(html.len()));
        let body = &html[body_start..body_end];
        let cleaned = html_to_text(body);
        if !cleaned.trim().is_empty() {
            return Some(cleaned);
        }
    }
    None
}

/// Lossy HTML-to-text conversion that **preserves list structure**.
///
/// Walksnail's release notes use `<ul><li>` for bullets; naive tag stripping
/// collapses those into a run-on sentence. We convert the structural tags
/// to newlines + bullet markers before dropping the rest so the rendered
/// changelog looks like a list again.
fn html_to_text(s: &str) -> String {
    static LI_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?is)<li[^>]*>(?P<t>.*?)</li>").expect("valid regex"));
    static BR_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?is)<br\s*/?>").expect("valid regex"));
    static BLOCK_END_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?is)</(?:p|div|h[1-6]|ul|ol|section|article)\s*>").expect("valid regex")
    });
    static TAG_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"<[^>]+>").expect("valid regex"));

    let with_bullets = LI_RE.replace_all(s, "\n• $t\n");
    let with_br = BR_RE.replace_all(&with_bullets, "\n");
    let with_blocks = BLOCK_END_RE.replace_all(&with_br, "\n");
    let no_tags = TAG_RE.replace_all(&with_blocks, "");
    let unescaped = html_unescape(&no_tags);
    // Collapse intra-line whitespace but keep line breaks, then drop
    // consecutive blanks so the output is tight.
    let mut out: Vec<String> = Vec::new();
    let mut last_blank = true;
    for raw in unescaped.lines() {
        let line = raw.split_whitespace().collect::<Vec<_>>().join(" ");
        if line.is_empty() {
            if !last_blank {
                out.push(String::new());
                last_blank = true;
            }
        } else {
            out.push(line);
            last_blank = false;
        }
    }
    while out.last().is_some_and(|l| l.is_empty()) {
        out.pop();
    }
    out.join("\n")
}

/// Walksnail.app's detail header reads e.g. "Released: 8 December 2025".
/// Parse the trailing date into `NaiveDate` when it matches.
fn extract_detail_date(html: &str) -> Option<NaiveDate> {
    static RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r#"(?is)Released:?\s*(?P<date>\d{1,2}\s+\w+\s+\d{4})"#).expect("valid regex")
    });
    let raw = RE.captures(html)?.name("date")?.as_str();
    for fmt in &["%-d %B %Y", "%d %B %Y"] {
        if let Ok(d) = NaiveDate::parse_from_str(raw.trim(), fmt) {
            return Some(d);
        }
    }
    None
}

fn refine_channel(html: &str, fallback: Channel) -> Channel {
    // Header block is the top ~4 KB and reads like:
    //   "Walksnail Avatar system Official Stable Released: 8 December 2025"
    //   "Walksnail Avatar system Official Beta Released: …"
    let head = html
        .get(..html.len().min(4000))
        .unwrap_or("")
        .to_lowercase();
    if head.contains("official beta")
        || head.contains("pre-release")
        || head.contains("beta release")
    {
        Channel::Beta
    } else if head.contains("official stable")
        || head.contains("stable release")
        || head.contains("official release")
    {
        Channel::Stable
    } else {
        fallback
    }
}

fn strip_tags(s: &str) -> String {
    static TAG_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"<[^>]+>").expect("valid regex"));
    let no_tags = TAG_RE.replace_all(s, " ");
    html_unescape(&no_tags)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn html_unescape(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use hardware::Hardware;

    #[test]
    fn listing_parser_extracts_slugs_and_dates() {
        let html = r#"
          <time datetime="2024-09-11">Sep 11 2024</time>
          <a href="/firmware/avatar/39.44.5">39.44.5 Official</a>
          <a href="/firmware/avatar/38.44.13">38.44.13 Beta</a>
        "#;
        let stubs = parse_listing(html);
        assert!(stubs.iter().any(|s| s.version == "39.44.5"));
        let s = stubs.iter().find(|s| s.version == "39.44.5").unwrap();
        assert_eq!(s.date, NaiveDate::from_ymd_opt(2024, 9, 11));
        let b = stubs.iter().find(|s| s.version == "38.44.13").unwrap();
        assert_eq!(b.channel, Channel::Beta);
    }

    #[test]
    fn detail_download_parser_pulls_every_variant() {
        let html = r#"
          <a href="https://download.walksnail.app/abc/Avatar_Sky_39.44.5.img?download">sky</a>
          <a href="https://download.walksnail.app/def/AvatarX_Gnd_39.44.5.img?download">x</a>
          <a href="https://download.walksnail.app/ghi/AvatarLite_Gnd_39.44.5.img?download">l</a>
          <a href="https://download.walksnail.app/jkl/Avatar_Relay_39.44.4.img?download">relay</a>
        "#;
        let dls = parse_download_links(html);
        let hws: Vec<Hardware> = dls.iter().map(|d| d.hardware).collect();
        assert!(hws.contains(&Hardware::AvatarSky));
        assert!(hws.contains(&Hardware::GogglesX));
        assert!(hws.contains(&Hardware::GogglesL));
        assert!(hws.contains(&Hardware::AvatarRelay));
        assert!(dls.iter().all(|d| d.sha1.is_empty()));
    }

    #[test]
    fn strip_tags_normalizes_whitespace() {
        let s = "<p>hello <b>world</b>\n\n foo</p>";
        assert_eq!(strip_tags(s), "hello world foo");
    }

    #[test]
    fn html_to_text_preserves_newlines_between_blocks() {
        let s = "<p>First paragraph.</p><ul><li>one</li><li>two</li></ul><p>End.</p>";
        let t = html_to_text(s);
        assert!(t.contains("First paragraph."));
        assert!(t.contains("• one"));
        assert!(t.contains("• two"));
        assert!(t.contains("End."));
        assert!(t.matches('\n').count() >= 3);
    }

    #[test]
    fn extract_notes_handles_whats_new_heading() {
        let html = r#"
            <h3 class="x">What&#x27;s new?</h3>
            <div>
              <h4>Bug Fix</h4>
              <p>Fixed the issue of color screen distortion on some cameras</p>
            </div>
            <h3>Downloads</h3>
        "#;
        let notes = extract_notes(html).expect("should extract");
        assert!(notes.contains("Bug Fix"));
        assert!(notes.contains("color screen distortion"));
        assert!(!notes.contains("Downloads"));
    }

    #[test]
    fn extract_notes_preserves_list_items_as_bullets() {
        let html = r#"
            <h3>What's new?</h3>
            <ul>
              <li>Race mode with stabilized latency</li>
              <li>Fixed DVR playback glitch</li>
            </ul>
            <h3>Downloads</h3>
        "#;
        let notes = extract_notes(html).expect("should extract");
        assert!(
            notes.contains("• Race mode with stabilized latency"),
            "got: {notes}"
        );
        assert!(notes.contains("• Fixed DVR playback glitch"));
    }

    #[test]
    fn extract_notes_handles_changelog_heading() {
        let html = r#"<h3>Changelog</h3><p>Lots of things happened.</p><h3>Downloads</h3>"#;
        assert_eq!(
            extract_notes(html).as_deref(),
            Some("Lots of things happened."),
        );
    }

    #[test]
    fn extract_detail_date_parses_header_format() {
        let html = "Walksnail Avatar system Official Stable Released: 8 December 2025 <h3>";
        assert_eq!(
            extract_detail_date(html),
            NaiveDate::from_ymd_opt(2025, 12, 8),
        );
    }

    #[test]
    fn refine_channel_uses_header() {
        assert_eq!(
            refine_channel("... Official Stable Released: ...", Channel::Unknown),
            Channel::Stable
        );
        assert_eq!(
            refine_channel("... Official Beta Released: ...", Channel::Unknown),
            Channel::Beta
        );
    }
}
