//! App-local paths and a tiny JSON-backed key-value store.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tauri::{AppHandle, Manager};

/// Ensures the directory exists, then returns it.
fn ensure(path: PathBuf) -> Result<PathBuf> {
    std::fs::create_dir_all(&path).with_context(|| format!("creating {}", path.display()))?;
    Ok(path)
}

pub fn data_dir(app: &AppHandle) -> Result<PathBuf> {
    let dir = app
        .path()
        .app_local_data_dir()
        .context("resolving app_local_data_dir")?;
    ensure(dir)
}

/// The directory where firmware `.img` files are cached by SHA1 prefix.
/// When the user picks a custom download dir in Settings, we persist it and
/// use it instead of the app-local default.
pub fn firmware_cache_dir(app: &AppHandle) -> Result<PathBuf> {
    let prefs = preferences_path(app)?;
    if let Some(custom) = read_string_pref(&prefs, "download_dir") {
        let p = PathBuf::from(custom);
        if !p.as_os_str().is_empty() {
            return ensure(p);
        }
    }
    ensure(data_dir(app)?.join("cache"))
}

pub fn index_cache_path(app: &AppHandle) -> Result<PathBuf> {
    Ok(data_dir(app)?.join("index.json"))
}

pub fn preferences_path(app: &AppHandle) -> Result<PathBuf> {
    Ok(data_dir(app)?.join("prefs.json"))
}

pub fn read_bool_pref(path: &Path, key: &str) -> bool {
    let Ok(bytes) = std::fs::read(path) else {
        return false;
    };
    let Ok(v): std::result::Result<serde_json::Value, _> = serde_json::from_slice(&bytes) else {
        return false;
    };
    v.get(key).and_then(|b| b.as_bool()).unwrap_or(false)
}

pub fn read_string_pref(path: &Path, key: &str) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let v: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    v.get(key).and_then(|b| b.as_str()).map(str::to_string)
}

pub fn write_bool_pref(path: &Path, key: &str, value: bool) -> Result<()> {
    merge_pref(path, key, serde_json::Value::Bool(value))
}

pub fn write_string_pref(path: &Path, key: &str, value: &str) -> Result<()> {
    merge_pref(path, key, serde_json::Value::String(value.to_string()))
}

pub fn delete_pref(path: &Path, key: &str) -> Result<()> {
    let mut v: serde_json::Value = match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|_| serde_json::json!({})),
        Err(_) => return Ok(()),
    };
    if let Some(obj) = v.as_object_mut() {
        obj.remove(key);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(path, serde_json::to_vec_pretty(&v)?)?;
    Ok(())
}

fn merge_pref(path: &Path, key: &str, value: serde_json::Value) -> Result<()> {
    let mut v: serde_json::Value = match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|_| serde_json::json!({})),
        Err(_) => serde_json::json!({}),
    };
    v[key] = value;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(path, serde_json::to_vec_pretty(&v)?)?;
    Ok(())
}
