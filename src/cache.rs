//! Incremental lint cache — skips unchanged files on repeated runs.
//! Stores violations per file keyed by (path, mtime, file_size).
//! Cache lives in `.ktlint-rs/cache.json` at the project root.

use crate::rules::Violation;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// Bump on format change to invalidate old caches.
const CACHE_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
struct CacheFile {
    version: u32,
    entries: HashMap<String, CachedViolations>,
}

#[derive(Serialize, Deserialize, Clone)]
struct CachedViolations {
    mtime_secs: u64,
    file_size: u64,
    violations: Vec<CachedViolation>,
}

#[derive(Serialize, Deserialize, Clone)]
struct CachedViolation {
    line: usize,
    col: usize,
    rule_id: String,
    message: String,
    auto_fixable: bool,
}

/// Determine the cache path: `.ktlint-rs/cache.json` under the project root.
pub fn cache_path(project_root: &Path) -> PathBuf {
    project_root.join(".ktlint-rs").join("cache.json")
}

/// Try to load cached violations for a file. Returns None if cache miss.
pub fn get_cached(path: &Path, project_root: &Path) -> Option<Vec<Violation>> {
    let meta = path.metadata().ok()?;
    let mtime = meta.modified().ok()?
        .duration_since(UNIX_EPOCH).ok()?
        .as_secs();
    let size = meta.len();

    let cache = load_cache(project_root)?;
    let key = cache_key(path, project_root);
    let cached = cache.entries.get(&key)?;

    // Check mtime + size match
    if cached.mtime_secs != mtime || cached.file_size != size {
        return None;
    }

    Some(cached.violations.iter().map(|v| Violation {
        file: path.to_string_lossy().to_string(),
        line: v.line,
        col: v.col,
        rule_id: v.rule_id.clone(),
        message: v.message.clone(),
        auto_fixable: v.auto_fixable,
    }).collect())
}

/// Save violations for a file to the cache.
pub fn save_cached(path: &Path, violations: &[Violation], project_root: &Path) {
    let meta = match path.metadata() {
        Ok(m) => m,
        Err(_) => return,
    };
    let mtime = match meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()) {
        Some(t) => t.as_secs(),
        None => return,
    };
    let size = meta.len();

    let mut cache = load_cache(project_root).unwrap_or(CacheFile {
        version: CACHE_VERSION,
        entries: HashMap::new(),
    });

    let key = cache_key(path, project_root);
    cache.entries.insert(key, CachedViolations {
        mtime_secs: mtime,
        file_size: size,
        violations: violations.iter().map(|v| CachedViolation {
            line: v.line,
            col: v.col,
            rule_id: v.rule_id.clone(),
            message: v.message.clone(),
            auto_fixable: v.auto_fixable,
        }).collect(),
    });

    save_cache(&cache, project_root);
}

fn cache_key(path: &Path, project_root: &Path) -> String {
    path.strip_prefix(project_root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

fn load_cache(project_root: &Path) -> Option<CacheFile> {
    let path = cache_path(project_root);
    let data = std::fs::read_to_string(&path).ok()?;
    let cache: CacheFile = serde_json::from_str(&data).ok()?;
    if cache.version != CACHE_VERSION { None } else { Some(cache) }
}

fn save_cache(cache: &CacheFile, project_root: &Path) {
    let path = cache_path(project_root);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string(cache) {
        let _ = std::fs::write(&path, json);
    }
}
