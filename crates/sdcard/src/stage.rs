//! Download a firmware `.img`, verify its SHA1, and copy it into the SD root
//! with all the side-effects a fresh install expects:
//!
//! * Delete any previous Walksnail `.img` in the SD root (only files that
//!   match the strict filename regex — we never wildcard-delete).
//! * Write the new `.img` with its canonical filename.
//! * For Moonlight, optionally drop an empty `independ_upgrade.txt` so the
//!   DVR board updates on first boot.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, bail, Context, Result};
use firmware_index::Download;
use futures_util::StreamExt;
use hardware::{parse_filename, Hardware};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StageProgress {
    Downloading { received: u64, total: Option<u64> },
    Verifying,
    Copying,
    Done(StageOutcome),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageOutcome {
    pub written: PathBuf,
    pub deleted_previous: Vec<PathBuf>,
    pub wrote_independ_upgrade: bool,
}

/// Resolve the cached `.img` for `download`, downloading (with progress +
/// SHA1 verify) if necessary, then stage it onto the SD at `sd_root`.
///
/// `cache_dir` is where the verified blob lives between runs so repeat users
/// don't re-download. Files are keyed by `<sha1>/<filename>`.
pub async fn stage_firmware(
    client: &reqwest::Client,
    download: &Download,
    cache_dir: &Path,
    sd_root: &Path,
    moonlight_dvr_only: bool,
    progress: Option<mpsc::Sender<StageProgress>>,
) -> Result<StageOutcome> {
    let (hardware, _) = parse_filename(&download.filename).ok_or_else(|| {
        anyhow!(
            "download `{}` doesn't look like a firmware file",
            download.filename
        )
    })?;
    let progress = Arc::new(progress);

    let cached = ensure_cached(client, download, cache_dir, progress.clone()).await?;

    emit(&progress, StageProgress::Copying).await;
    let outcome = copy_to_sd(&cached, download, hardware, sd_root, moonlight_dvr_only).await?;

    emit(&progress, StageProgress::Done(outcome.clone())).await;
    Ok(outcome)
}

async fn emit(progress: &Arc<Option<mpsc::Sender<StageProgress>>>, msg: StageProgress) {
    if let Some(tx) = progress.as_ref() {
        // Best-effort: ignore a closed channel.
        let _ = tx.send(msg).await;
    }
}

async fn ensure_cached(
    client: &reqwest::Client,
    download: &Download,
    cache_dir: &Path,
    progress: Arc<Option<mpsc::Sender<StageProgress>>>,
) -> Result<PathBuf> {
    let key = if download.sha1.is_empty() {
        // Official scraper doesn't publish SHA1. Fall back to a derived key so
        // the cache still deduplicates by filename.
        format!("nohash-{}", download.filename)
    } else {
        download.sha1.to_lowercase()
    };
    let dir = cache_dir.join(&key);
    let target = dir.join(&download.filename);
    if target.exists()
        && (download.sha1.is_empty() || verify_sha1(&target, &download.sha1).await.unwrap_or(false))
    {
        info!(path = %target.display(), "using cached firmware");
        return Ok(target);
    }
    tokio::fs::create_dir_all(&dir).await.ok();
    download_to(client, download, &target, progress.clone()).await?;
    if !download.sha1.is_empty() {
        emit(&progress, StageProgress::Verifying).await;
        let ok = verify_sha1(&target, &download.sha1).await?;
        if !ok {
            // Don't leave a corrupt blob in the cache.
            tokio::fs::remove_file(&target).await.ok();
            bail!("SHA1 verification failed for {}", download.filename);
        }
    }
    Ok(target)
}

async fn download_to(
    client: &reqwest::Client,
    download: &Download,
    target: &Path,
    progress: Arc<Option<mpsc::Sender<StageProgress>>>,
) -> Result<()> {
    let resp = client
        .get(&download.url)
        .send()
        .await
        .with_context(|| format!("GET {}", download.url))?
        .error_for_status()
        .with_context(|| format!("non-2xx from {}", download.url))?;
    let total = resp.content_length();
    let mut stream = resp.bytes_stream();
    let tmp = target.with_extension("img.part");
    let mut file = tokio::fs::File::create(&tmp)
        .await
        .with_context(|| format!("creating {}", tmp.display()))?;
    let mut received: u64 = 0;
    while let Some(chunk) = stream.next().await {
        let bytes = chunk.context("reading download chunk")?;
        file.write_all(&bytes)
            .await
            .context("writing download chunk")?;
        received += bytes.len() as u64;
        emit(&progress, StageProgress::Downloading { received, total }).await;
    }
    file.flush().await?;
    drop(file);
    tokio::fs::rename(&tmp, target)
        .await
        .context("promoting tmp file")?;
    Ok(())
}

async fn verify_sha1(path: &Path, expected: &str) -> Result<bool> {
    let bytes = tokio::fs::read(path).await?;
    let actual = hex::encode(Sha1::digest(&bytes));
    Ok(actual.eq_ignore_ascii_case(expected))
}

async fn copy_to_sd(
    src: &Path,
    download: &Download,
    hardware: Hardware,
    sd_root: &Path,
    moonlight_dvr_only: bool,
) -> Result<StageOutcome> {
    if !sd_root.is_dir() {
        bail!("SD root `{}` is not a directory", sd_root.display());
    }
    let mut deleted_previous = Vec::new();
    for entry in
        std::fs::read_dir(sd_root).with_context(|| format!("reading {}", sd_root.display()))?
    {
        let entry = entry?;
        let name = entry.file_name();
        let name_s = name.to_string_lossy();
        if parse_filename(&name_s).is_some() {
            let p = entry.path();
            tokio::fs::remove_file(&p)
                .await
                .with_context(|| format!("removing stale firmware `{}`", p.display()))?;
            deleted_previous.push(p);
        }
    }
    let dest = sd_root.join(&download.filename);
    tokio::fs::copy(src, &dest)
        .await
        .with_context(|| format!("copying to {}", dest.display()))?;
    if let Ok(file) = tokio::fs::File::open(&dest).await {
        let _ = file.sync_all().await;
    }

    let wrote_independ_upgrade = moonlight_dvr_only && hardware == Hardware::MoonlightSky;
    if wrote_independ_upgrade {
        let marker = sd_root.join("independ_upgrade.txt");
        tokio::fs::write(&marker, b"").await?;
    }

    Ok(StageOutcome {
        written: dest,
        deleted_previous,
        wrote_independ_upgrade,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use firmware_index::Download;
    use hardware::Version;

    fn mk_file(dir: &Path, name: &str, bytes: &[u8]) {
        std::fs::write(dir.join(name), bytes).unwrap();
    }

    fn cached_dl(sha: &str, filename: &str) -> Download {
        Download {
            hardware: Hardware::GogglesX,
            filename: filename.to_string(),
            url: "https://example.invalid/not-reachable".into(),
            sha1: sha.into(),
        }
    }

    #[tokio::test]
    async fn copy_deletes_old_firmware_and_writes_new() {
        let sd = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let payload = b"new-firmware-bytes";
        let sha = hex::encode(Sha1::digest(payload));

        let cached_dir = cache.path().join(&sha);
        std::fs::create_dir_all(&cached_dir).unwrap();
        let cached_path = cached_dir.join("AvatarX_Gnd_39.44.5.img");
        std::fs::write(&cached_path, payload).unwrap();

        mk_file(sd.path(), "AvatarX_Gnd_37.42.4.img", b"old");
        mk_file(sd.path(), "random.txt", b"untouched");

        let dl = cached_dl(&sha, "AvatarX_Gnd_39.44.5.img");
        let outcome = stage_firmware(
            &reqwest::Client::new(),
            &dl,
            cache.path(),
            sd.path(),
            false,
            None,
        )
        .await
        .unwrap();

        assert!(outcome.written.ends_with("AvatarX_Gnd_39.44.5.img"));
        assert_eq!(outcome.deleted_previous.len(), 1);
        assert!(
            sd.path().join("random.txt").exists(),
            "unrelated files preserved"
        );
        assert!(!sd.path().join("AvatarX_Gnd_37.42.4.img").exists());
        assert!(!outcome.wrote_independ_upgrade);
    }

    #[tokio::test]
    async fn moonlight_writes_independ_upgrade_when_requested() {
        let sd = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let payload = b"moonlight-bytes";
        let sha = hex::encode(Sha1::digest(payload));

        let cached_dir = cache.path().join(&sha);
        std::fs::create_dir_all(&cached_dir).unwrap();
        let cached_path =
            cached_dir.join(Hardware::MoonlightSky.canonical_filename(Version::new(39, 44, 5)));
        std::fs::write(&cached_path, payload).unwrap();

        let dl = Download {
            hardware: Hardware::MoonlightSky,
            filename: Hardware::MoonlightSky.canonical_filename(Version::new(39, 44, 5)),
            url: "https://example.invalid/not-reachable".into(),
            sha1: sha,
        };
        let outcome = stage_firmware(
            &reqwest::Client::new(),
            &dl,
            cache.path(),
            sd.path(),
            true,
            None,
        )
        .await
        .unwrap();
        assert!(outcome.wrote_independ_upgrade);
        assert!(sd.path().join("independ_upgrade.txt").exists());
    }

    #[tokio::test]
    async fn cache_corrupt_sha1_rejected() {
        let sd = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let sha = "deadbeef".to_string();

        let cached_dir = cache.path().join(&sha);
        std::fs::create_dir_all(&cached_dir).unwrap();
        // Wrong bytes for the advertised sha
        std::fs::write(cached_dir.join("AvatarX_Gnd_39.44.5.img"), b"wrong").unwrap();

        let dl = cached_dl(&sha, "AvatarX_Gnd_39.44.5.img");
        let err = stage_firmware(
            &reqwest::Client::new(),
            &dl,
            cache.path(),
            sd.path(),
            false,
            None,
        )
        .await;
        assert!(err.is_err());
    }
}
