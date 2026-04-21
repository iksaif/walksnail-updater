//! Build-time utility: downloads upstream Walksnail PDF manuals, extracts
//! each product's firmware-upgrade chapter, and writes a JSON blob that the
//! Tauri app bundles into its binary.
//!
//! The runtime app never fetches PDFs itself. This is intentional: parsing a
//! newly-updated PDF at runtime could produce garbage and erode trust.
//! Instead, a scheduled CI job reruns the extractor and opens a PR if the
//! output changed — updates are reviewed like any other code change.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use hardware::Hardware;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub hardware: BTreeMap<String, HardwareEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HardwareEntry {
    pub pdf_url: String,
    /// Regex (case-insensitive) that marks the start of the upgrade chapter in
    /// the extracted text. Defaults to `firmware upgrade` if absent.
    pub section: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instructions {
    pub hardware: BTreeMap<Hardware, HardwareInstructions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInstructions {
    pub steps: Vec<String>,
    pub source_url: String,
    pub fetched_at: chrono::DateTime<chrono::Utc>,
    pub manual_sha256: String,
}

pub fn parse_manifest(path: &Path) -> Result<Manifest> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("reading manifest at {}", path.display()))?;
    toml::from_str(&text).context("parsing manifest TOML")
}

/// Map a hardware key in the manifest to a [`Hardware`] enum variant.
pub fn parse_hardware_key(key: &str) -> Option<Hardware> {
    // Accept either the enum variant name or the display name.
    match key {
        "AvatarSky" => Some(Hardware::AvatarSky),
        "AvatarMiniSky" => Some(Hardware::AvatarMiniSky),
        "MoonlightSky" => Some(Hardware::MoonlightSky),
        "AvatarGnd" => Some(Hardware::AvatarGnd),
        "GogglesX" => Some(Hardware::GogglesX),
        "GogglesL" => Some(Hardware::GogglesL),
        "VrxSE" => Some(Hardware::VrxSE),
        "ReconHd" => Some(Hardware::ReconHd),
        _ => None,
    }
}

pub fn download_pdf(client: &reqwest::blocking::Client, url: &str, into: &Path) -> Result<Vec<u8>> {
    let bytes = client
        .get(url)
        .send()
        .with_context(|| format!("GET {url}"))?
        .error_for_status()
        .with_context(|| format!("non-2xx from {url}"))?
        .bytes()
        .with_context(|| format!("reading body from {url}"))?;
    if let Some(parent) = into.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(into, &bytes).with_context(|| format!("writing {}", into.display()))?;
    Ok(bytes.to_vec())
}

pub fn extract_steps(pdf_bytes: &[u8], section_re: &str) -> Result<Vec<String>> {
    let text = pdf_extract::extract_text_from_mem(pdf_bytes).context("PDF text extraction")?;
    Ok(parse_section(&text, section_re))
}

/// Slice the chapter beginning at a header matching `section_re`, then break
/// it into steps.
///
/// Heuristic: the section spans from the first match of `section_re` to the
/// next "blank line followed by ALL-CAPS line" (a common chapter break in
/// these manuals). Steps are produced from numbered lines (`1.`, `2)`, `•`)
/// and fall back to paragraph splitting when no numbering is present.
pub fn parse_section(text: &str, section_re: &str) -> Vec<String> {
    let re = Regex::new(&format!("(?i){section_re}"))
        .unwrap_or_else(|_| Regex::new("(?i)firmware upgrade").expect("fallback regex is valid"));
    let Some(start) = re.find(text) else {
        return Vec::new();
    };
    let tail = &text[start.start()..];

    let end = find_next_chapter(tail).unwrap_or(tail.len());
    let chapter = &tail[..end];

    static STEP_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?m)^\s*(?:(?:\d+[\.)])|[•\-\*])\s+(.+)$").expect("valid regex"));

    let steps: Vec<String> = STEP_RE
        .captures_iter(chapter)
        .map(|c| normalize_step(&c[1]))
        .filter(|s| !s.is_empty())
        .collect();

    if !steps.is_empty() {
        return steps;
    }
    // Fallback: split on blank lines, strip the heading.
    chapter
        .split("\n\n")
        .skip(1)
        .map(normalize_step)
        .filter(|s| !s.is_empty() && s.len() < 600)
        .collect()
}

fn find_next_chapter(chapter: &str) -> Option<usize> {
    let mut prev_blank = false;
    let mut pos = 0usize;
    for line in chapter.lines() {
        let len = line.len() + 1;
        let trimmed = line.trim();
        let is_caps_heading = !trimmed.is_empty()
            && trimmed.len() >= 3
            && trimmed.chars().all(|c| !c.is_lowercase())
            && trimmed.chars().any(|c| c.is_alphabetic());
        if prev_blank && is_caps_heading && pos > 32 {
            return Some(pos);
        }
        prev_blank = trimmed.is_empty();
        pos += len;
    }
    None
}

fn normalize_step(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha1::Digest;
    // We deliberately use SHA1 everywhere in this crate set for consistency
    // with the firmware index's SHA1 verification. Rename the helper as
    // 'hash_hex' later if we want; for now SHA1 is fine for change detection.
    hex::encode(sha1::Sha1::digest(bytes))
}

pub fn default_manifest_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("manifest.toml")
}

pub fn default_output_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/resources/instructions.json")
}

pub fn default_manuals_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/resources/manuals")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_section_extracts_numbered_steps() {
        let text = "\
Introduction

Firmware Upgrade
1. Copy the firmware to the SD card root.
2. Insert the SD card into the goggles.
3. Power on and wait for the upgrade to complete.

NEXT CHAPTER
Some unrelated content.
";
        let steps = parse_section(text, "firmware upgrade");
        assert_eq!(steps.len(), 3);
        assert!(steps[0].contains("Copy the firmware"));
    }

    #[test]
    fn parse_section_returns_empty_when_no_match() {
        assert!(parse_section("unrelated", "firmware upgrade").is_empty());
    }

    #[test]
    fn manifest_roundtrip() {
        let toml = r#"
            [hardware.GogglesX]
            pdf_url = "https://example/x.pdf"
            section = "firmware upgrade"
        "#;
        let m: Manifest = toml::from_str(toml).unwrap();
        assert_eq!(m.hardware.len(), 1);
        assert!(parse_hardware_key("GogglesX").is_some());
    }
}
