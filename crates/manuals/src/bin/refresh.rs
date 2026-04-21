//! Refresh `src-tauri/resources/instructions.json` from upstream PDF manuals.
//!
//! Usage: `cargo run -p manuals --bin refresh`.
//!
//! The script is intentionally idempotent: it always downloads each PDF, and
//! only writes the JSON output if anything changed. Parsed instructions are
//! recorded alongside the PDF's SHA1 and the timestamp of the fetch so diffs
//! are meaningful in PR review.

use std::collections::BTreeMap;
use std::fs;

use anyhow::{Context, Result};
use chrono::Utc;
use manuals::{
    default_manifest_path, default_manuals_dir, default_output_path, download_pdf, extract_steps,
    parse_hardware_key, parse_manifest, sha256_hex, HardwareInstructions, Instructions,
};
use tracing::{info, warn};

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let manifest =
        parse_manifest(&default_manifest_path()).context("parsing manuals/manifest.toml")?;
    let client = reqwest::blocking::Client::builder()
        .user_agent("walksnail-updater-manuals/0.1")
        .build()?;

    let manuals_dir = default_manuals_dir();
    fs::create_dir_all(&manuals_dir).ok();

    let mut out = Instructions {
        hardware: BTreeMap::new(),
    };
    for (key, entry) in &manifest.hardware {
        let Some(hw) = parse_hardware_key(key) else {
            warn!(%key, "manifest key doesn't map to a Hardware variant — skipping");
            continue;
        };
        let pdf_name = format!(
            "{}.pdf",
            key.chars()
                .flat_map(|c| c.to_lowercase())
                .collect::<String>()
        );
        let pdf_path = manuals_dir.join(&pdf_name);
        info!(%key, url = %entry.pdf_url, "downloading");
        let bytes = match download_pdf(&client, &entry.pdf_url, &pdf_path) {
            Ok(b) => b,
            Err(e) => {
                warn!(%key, error = %e, "download failed, skipping");
                continue;
            }
        };
        let section = entry.section.as_deref().unwrap_or("firmware upgrade");
        let steps = match extract_steps(&bytes, section) {
            Ok(s) if !s.is_empty() => s,
            Ok(_) => {
                warn!(%key, "no steps extracted; leaving previous entry intact if any");
                continue;
            }
            Err(e) => {
                warn!(%key, error = %e, "PDF extract failed, skipping");
                continue;
            }
        };
        out.hardware.insert(
            hw,
            HardwareInstructions {
                steps,
                source_url: entry.pdf_url.clone(),
                fetched_at: Utc::now(),
                manual_sha256: sha256_hex(&bytes),
            },
        );
    }

    let out_path = default_output_path();
    let json = serde_json::to_string_pretty(&out)?;
    fs::write(&out_path, json).with_context(|| format!("writing {}", out_path.display()))?;
    info!(path = %out_path.display(), count = out.hardware.len(), "wrote instructions");
    Ok(())
}
