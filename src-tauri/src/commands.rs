//! Tauri `#[command]` handlers — the thin bridge between the JS frontend
//! and the Rust crates.
//!
//! Keep these functions short: they validate inputs, translate types, call
//! into `firmware-index` / `sdcard`, and surface results as JSON. No business
//! logic lives here; it belongs in the crates so it stays unit-testable.

use std::path::PathBuf;

use firmware_index::{FirmwareRelease, Index, Source};
use hardware::{safety_check, Hardware, SafetyVerdict, Version};
use sdcard::{scan, stage_firmware as stage_impl, SdContents, StageOutcome, StageProgress};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::mpsc;
use tracing::warn;

use crate::cache::{
    data_dir, delete_pref, firmware_cache_dir, index_cache_path, preferences_path, read_bool_pref,
    read_string_pref, write_bool_pref, write_string_pref,
};
use crate::events;
use crate::state::AppState;

type CmdResult<T> = Result<T, String>;

fn stringify<E: std::fmt::Display>(err: E) -> String {
    err.to_string()
}

#[tauri::command]
pub async fn fetch_index(
    app: AppHandle,
    state: State<'_, AppState>,
    source: Option<Source>,
) -> CmdResult<Index> {
    let source = source.unwrap_or_default();
    let cache_path = index_cache_path(&app).map_err(stringify)?;
    match firmware_index::fetch(&state.http, source).await {
        Ok(index) => {
            if let Err(e) = firmware_index::save_cache(&cache_path, &index).await {
                warn!(error = %e, "failed to persist index cache");
            }
            *state.index.write().await = Some(index.clone());
            Ok(index)
        }
        Err(err) => {
            warn!(error = %err, "online fetch failed; falling back to cache");
            match firmware_index::load_cache(&cache_path).await {
                Ok(cached) => {
                    *state.index.write().await = Some(cached.clone());
                    Ok(cached)
                }
                Err(cache_err) => Err(format!(
                    "fetch failed: {err:#}; cache unavailable: {cache_err:#}"
                )),
            }
        }
    }
}

#[tauri::command]
pub async fn list_releases(
    state: State<'_, AppState>,
    hardware: Hardware,
) -> CmdResult<Vec<FirmwareRelease>> {
    let guard = state.index.read().await;
    let index = guard.as_ref().ok_or("index not loaded")?;
    Ok(index.releases_for(hardware).cloned().collect())
}

#[tauri::command]
pub async fn latest_for(
    state: State<'_, AppState>,
    hardware: Hardware,
) -> CmdResult<Option<FirmwareRelease>> {
    let guard = state.index.read().await;
    let Some(index) = guard.as_ref() else {
        return Ok(None);
    };
    Ok(index.latest_stable_for(hardware).cloned())
}

#[derive(Debug, Serialize)]
pub struct ScanResult {
    pub contents: SdContents,
    pub latest_stable: Option<FirmwareRelease>,
    pub verdict: Option<SafetyVerdict>,
}

#[tauri::command]
pub async fn scan_sd(state: State<'_, AppState>, path: PathBuf) -> CmdResult<ScanResult> {
    let contents = scan(&path);
    let guard = state.index.read().await;
    let latest_stable = contents.variant.and_then(|hw| {
        guard
            .as_ref()
            .and_then(|idx| idx.latest_stable_for(hw).cloned())
    });
    let verdict = match (contents.variant, &latest_stable) {
        (Some(hw), Some(release)) => {
            let current = contents.running_version.or(contents.staged_version);
            Some(safety_check(hw, current, release.version))
        }
        _ => None,
    };
    Ok(ScanResult {
        contents,
        latest_stable,
        verdict,
    })
}

#[derive(Debug, Deserialize)]
pub struct DownloadArgs {
    pub hardware: Hardware,
    pub version: Version,
}

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DownloadProgress {
    Started {
        filename: String,
    },
    Progress {
        filename: String,
        received: u64,
        total: Option<u64>,
    },
    Verifying {
        filename: String,
    },
    Done {
        filename: String,
        path: PathBuf,
        sha_verified: bool,
    },
    Failed {
        filename: String,
        reason: String,
    },
}

#[tauri::command]
pub async fn download_firmware(
    app: AppHandle,
    state: State<'_, AppState>,
    args: DownloadArgs,
) -> CmdResult<PathBuf> {
    let release = find_release(&state, args.hardware, args.version).await?;
    let download = release
        .downloads
        .into_iter()
        .find(|d| d.hardware == args.hardware)
        .ok_or_else(|| format!("no download for {:?}", args.hardware))?;
    let cache_dir = firmware_cache_dir(&app).map_err(stringify)?;
    let key = if download.sha1.is_empty() {
        format!("nohash-{}", download.filename)
    } else {
        download.sha1.to_lowercase()
    };
    let target = cache_dir.join(&key).join(&download.filename);
    if target.exists() {
        // Announce done so the UI's spinner-state is consistent even when
        // we hit the cache without doing any network work.
        let _ = app.emit(
            events::DOWNLOAD_PROGRESS,
            &DownloadProgress::Done {
                filename: download.filename.clone(),
                path: target.clone(),
                sha_verified: false,
            },
        );
        return Ok(target);
    }
    download_streaming(&app, &state.http, &download, &cache_dir).await
}

async fn download_streaming(
    app: &AppHandle,
    client: &reqwest::Client,
    download: &firmware_index::Download,
    cache_dir: &std::path::Path,
) -> CmdResult<PathBuf> {
    use futures_util::StreamExt as _;
    use tokio::io::AsyncWriteExt;

    let filename = download.filename.clone();
    let emit = |p: DownloadProgress| {
        let _ = app.emit(events::DOWNLOAD_PROGRESS, &p);
    };
    emit(DownloadProgress::Started {
        filename: filename.clone(),
    });

    let key = if download.sha1.is_empty() {
        format!("nohash-{}", download.filename)
    } else {
        download.sha1.to_lowercase()
    };
    let dir = cache_dir.join(&key);
    if let Err(e) = tokio::fs::create_dir_all(&dir).await {
        let reason = e.to_string();
        emit(DownloadProgress::Failed {
            filename: filename.clone(),
            reason: reason.clone(),
        });
        return Err(reason);
    }
    let target = dir.join(&download.filename);
    let tmp = target.with_extension("img.part");

    let resp = match client.get(&download.url).send().await {
        Ok(r) => match r.error_for_status() {
            Ok(r) => r,
            Err(e) => {
                let reason = e.to_string();
                emit(DownloadProgress::Failed {
                    filename: filename.clone(),
                    reason: reason.clone(),
                });
                return Err(reason);
            }
        },
        Err(e) => {
            let reason = e.to_string();
            emit(DownloadProgress::Failed {
                filename: filename.clone(),
                reason: reason.clone(),
            });
            return Err(reason);
        }
    };
    let total = resp.content_length();
    let mut stream = resp.bytes_stream();
    let mut file = match tokio::fs::File::create(&tmp).await {
        Ok(f) => f,
        Err(e) => {
            let reason = e.to_string();
            emit(DownloadProgress::Failed {
                filename: filename.clone(),
                reason: reason.clone(),
            });
            return Err(reason);
        }
    };

    use sha1::{Digest, Sha1};
    let mut hasher = Sha1::new();
    let mut received: u64 = 0;
    // Throttle progress emission to ~10 events/sec so the frontend can
    // animate smoothly instead of re-rendering on every 16 KB chunk.
    let mut last_emitted_at = std::time::Instant::now();
    let mut last_emitted_bytes: u64 = 0;
    const EMIT_INTERVAL: std::time::Duration = std::time::Duration::from_millis(100);
    const EMIT_BYTES: u64 = 256 * 1024;
    while let Some(chunk) = stream.next().await {
        let bytes = match chunk {
            Ok(b) => b,
            Err(e) => {
                let reason = e.to_string();
                emit(DownloadProgress::Failed {
                    filename: filename.clone(),
                    reason: reason.clone(),
                });
                tokio::fs::remove_file(&tmp).await.ok();
                return Err(reason);
            }
        };
        if let Err(e) = file.write_all(&bytes).await {
            let reason = e.to_string();
            emit(DownloadProgress::Failed {
                filename: filename.clone(),
                reason: reason.clone(),
            });
            return Err(reason);
        }
        if !download.sha1.is_empty() {
            hasher.update(&bytes);
        }
        received += bytes.len() as u64;
        let now = std::time::Instant::now();
        if now.duration_since(last_emitted_at) >= EMIT_INTERVAL
            || received - last_emitted_bytes >= EMIT_BYTES
        {
            last_emitted_at = now;
            last_emitted_bytes = received;
            emit(DownloadProgress::Progress {
                filename: filename.clone(),
                received,
                total,
            });
        }
    }
    // Final tick — guarantees the UI sees 100% before we switch to Done.
    emit(DownloadProgress::Progress {
        filename: filename.clone(),
        received,
        total,
    });
    file.flush().await.ok();
    drop(file);

    let sha_verified = if download.sha1.is_empty() {
        false
    } else {
        emit(DownloadProgress::Verifying {
            filename: filename.clone(),
        });
        let actual = hex::encode(hasher.finalize());
        if !actual.eq_ignore_ascii_case(&download.sha1) {
            let reason = format!(
                "SHA1 mismatch for {}: expected {}, got {}",
                download.filename, download.sha1, actual
            );
            tokio::fs::remove_file(&tmp).await.ok();
            emit(DownloadProgress::Failed {
                filename: filename.clone(),
                reason: reason.clone(),
            });
            return Err(reason);
        }
        true
    };

    if let Err(e) = tokio::fs::rename(&tmp, &target).await {
        let reason = e.to_string();
        emit(DownloadProgress::Failed {
            filename: filename.clone(),
            reason: reason.clone(),
        });
        return Err(reason);
    }
    emit(DownloadProgress::Done {
        filename: filename.clone(),
        path: target.clone(),
        sha_verified,
    });
    Ok(target)
}

async fn find_release(
    state: &State<'_, AppState>,
    hardware: Hardware,
    version: Version,
) -> CmdResult<FirmwareRelease> {
    let guard = state.index.read().await;
    let index = guard.as_ref().ok_or("index not loaded")?;
    let found = index
        .releases_for(hardware)
        .find(|r| r.version == version)
        .cloned();
    found.ok_or_else(|| format!("no release {version} for {hardware:?}"))
}

#[derive(Debug, Deserialize)]
pub struct StageArgs {
    pub hardware: Hardware,
    pub version: Version,
    pub sd_root: PathBuf,
    #[serde(default)]
    pub moonlight_dvr_only: bool,
}

#[tauri::command]
pub async fn stage_firmware(
    app: AppHandle,
    state: State<'_, AppState>,
    args: StageArgs,
) -> CmdResult<StageOutcome> {
    let release = find_release(&state, args.hardware, args.version).await?;
    let download = release
        .downloads
        .into_iter()
        .find(|d| d.hardware == args.hardware)
        .ok_or_else(|| "no matching download".to_string())?;
    let cache_dir = firmware_cache_dir(&app).map_err(stringify)?;

    let (tx, mut rx) = mpsc::channel::<StageProgress>(32);
    let handle = app.clone();
    let forwarder = tokio::spawn(async move {
        while let Some(p) = rx.recv().await {
            let _ = handle.emit(events::STAGE_PROGRESS, &p);
        }
    });

    let result = stage_impl(
        &state.http,
        &download,
        &cache_dir,
        &args.sd_root,
        args.moonlight_dvr_only,
        Some(tx),
    )
    .await
    .map_err(stringify);

    forwarder.abort();
    result
}

#[tauri::command]
pub async fn reveal_in_file_manager(app: AppHandle, path: PathBuf) -> CmdResult<()> {
    use tauri_plugin_opener::OpenerExt;
    app.opener().reveal_item_in_dir(&path).map_err(stringify)
}

#[tauri::command]
pub async fn open_url(app: AppHandle, url: String) -> CmdResult<()> {
    use tauri_plugin_opener::OpenerExt;
    app.opener().open_url(url, None::<&str>).map_err(stringify)
}

#[tauri::command]
pub async fn load_instructions(app: AppHandle) -> CmdResult<serde_json::Value> {
    let path = app
        .path()
        .resolve(
            "resources/instructions.json",
            tauri::path::BaseDirectory::Resource,
        )
        .map_err(stringify)?;
    let bytes = tokio::fs::read(&path).await.map_err(stringify)?;
    let v: serde_json::Value = serde_json::from_slice(&bytes).map_err(stringify)?;
    Ok(v)
}

#[derive(Debug, Serialize)]
pub struct AppPaths {
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
}

#[tauri::command]
pub async fn get_app_paths(app: AppHandle) -> CmdResult<AppPaths> {
    Ok(AppPaths {
        data_dir: data_dir(&app).map_err(stringify)?,
        cache_dir: firmware_cache_dir(&app).map_err(stringify)?,
    })
}

#[tauri::command]
pub async fn mark_onboarded(app: AppHandle) -> CmdResult<()> {
    let path = preferences_path(&app).map_err(stringify)?;
    write_bool_pref(&path, "onboarded", true).map_err(stringify)
}

#[tauri::command]
pub async fn is_onboarded(app: AppHandle) -> CmdResult<bool> {
    let path = preferences_path(&app).map_err(stringify)?;
    Ok(read_bool_pref(&path, "onboarded"))
}

#[tauri::command]
pub async fn get_download_dir(app: AppHandle) -> CmdResult<PathBuf> {
    firmware_cache_dir(&app).map_err(stringify)
}

/// Enumerate every cached firmware file so the UI can mark downloads as
/// already-present without having to query one-by-one.
///
/// Key is the canonical filename (e.g. `AvatarX_Gnd_39.44.5.img`).
#[tauri::command]
pub async fn list_cached_firmware(
    app: AppHandle,
) -> CmdResult<std::collections::HashMap<String, PathBuf>> {
    let dir = firmware_cache_dir(&app).map_err(stringify)?;
    let mut out = std::collections::HashMap::new();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Ok(out);
    };
    for subdir in entries.flatten() {
        if !subdir.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let Ok(files) = std::fs::read_dir(subdir.path()) else {
            continue;
        };
        for file in files.flatten() {
            let name = file.file_name().to_string_lossy().into_owned();
            if name.ends_with(".img") {
                out.insert(name, file.path());
            }
        }
    }
    Ok(out)
}

#[derive(Debug, Deserialize)]
pub struct SetDownloadDirArgs {
    pub path: Option<String>,
}

#[tauri::command]
pub async fn set_download_dir(app: AppHandle, args: SetDownloadDirArgs) -> CmdResult<PathBuf> {
    let prefs = preferences_path(&app).map_err(stringify)?;
    match args.path {
        Some(p) if !p.is_empty() => {
            write_string_pref(&prefs, "download_dir", &p).map_err(stringify)?;
        }
        _ => {
            delete_pref(&prefs, "download_dir").map_err(stringify)?;
        }
    }
    firmware_cache_dir(&app).map_err(stringify)
}

#[tauri::command]
pub async fn get_download_dir_pref(app: AppHandle) -> CmdResult<Option<String>> {
    let prefs = preferences_path(&app).map_err(stringify)?;
    Ok(read_string_pref(&prefs, "download_dir"))
}

use tauri::Manager;
