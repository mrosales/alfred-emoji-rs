//! Background self-update check against GitHub Releases.
//!
//! There's no first-party Alfred plist key for "check a repo for updates"
//! — every third-party workflow implements it itself. This follows the
//! long-standing `deanishe/alfred-workflow` convention: poll
//! `GET /repos/<owner>/<repo>/releases/latest` at most once a day, expect
//! exactly one `.alfredworkflow` asset on the release, cache the result,
//! and let the caller decide whether to surface it (see `main.rs`, which
//! only does so on an empty query). The release itself is produced by the
//! `cut-release` skill.
//!
//! The network call never happens inline in the search path — Alfred runs
//! this binary on every keystroke, and blocking that on a GitHub round
//! trip would make typing laggy. Instead `check` reads a cache file and,
//! if it's stale, spawns a detached copy of this binary with `CHECK_ARG`
//! to refresh it in the background; the *next* invocation picks up
//! whatever that run found.

use std::env;
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

const CHECK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);
const CACHE_FILE: &str = "update_check.json";
/// First CLI arg that routes `main` into `run_background_check` instead of
/// a search — see `handle_update_check_subcommand` in `main.rs`.
pub const CHECK_ARG: &str = "--update-check";

#[derive(Serialize, Deserialize, Default)]
struct Cache {
    checked_at: u64,
    latest_version: Option<String>,
    download_url: Option<String>,
}

pub struct AvailableUpdate {
    pub version: String,
    pub download_url: String,
}

/// Alfred sets this to a per-workflow, per-user cache directory. Fall back
/// to the system temp dir so the binary still runs (e.g. under `cargo
/// test`, or from a plain terminal) when it's unset.
fn cache_path() -> std::path::PathBuf {
    let dir = env::var_os("alfred_workflow_cache")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(env::temp_dir);
    dir.join(CACHE_FILE)
}

fn read_cache() -> Cache {
    std::fs::read(cache_path())
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn write_cache(cache: &Cache) {
    let path = cache_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_vec(cache) {
        let _ = std::fs::write(path, json);
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Cheap on every call (cache read only, never blocks on the network).
/// Returns the last completed check's result, which may lag a run behind
/// while a background refresh is in flight.
pub fn check(current_version: &str, repository: &str) -> Option<AvailableUpdate> {
    let mut cache = read_cache();
    let stale = now_secs().saturating_sub(cache.checked_at) > CHECK_INTERVAL.as_secs();
    if stale {
        // Bump checked_at before spawning so rapid keystrokes (each a
        // fresh process) don't each see a stale cache and pile on their
        // own background check.
        cache.checked_at = now_secs();
        write_cache(&cache);
        spawn_background_check(repository);
    }

    let latest = cache.latest_version?;
    let download_url = cache.download_url?;
    is_newer(&latest, current_version).then_some(AvailableUpdate {
        version: latest,
        download_url,
    })
}

fn spawn_background_check(repository: &str) {
    let Ok(exe) = env::current_exe() else {
        return;
    };
    let _ = Command::new(exe)
        .arg(CHECK_ARG)
        .arg(repository)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

/// Entry point for the detached `--update-check` subprocess. Never panics
/// — a failed check just leaves the cache stale, and `check` retries it
/// next time the interval elapses.
pub fn run_background_check(repository: &str) {
    let mut cache = read_cache();
    cache.checked_at = now_secs();
    if let Some((version, download_url)) = fetch_latest_release(repository) {
        cache.latest_version = Some(version);
        cache.download_url = Some(download_url);
    }
    write_cache(&cache);
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

/// `repository` is Cargo's `repository` field, e.g.
/// `"https://github.com/mrosales/alfred-emoji-rs"`.
fn fetch_latest_release(repository: &str) -> Option<(String, String)> {
    let slug = repository
        .trim_end_matches('/')
        .rsplit("github.com/")
        .next()?;
    let url = format!("https://api.github.com/repos/{slug}/releases/latest");
    let release: Release = ureq::get(&url)
        .set("User-Agent", "alfred-emoji-rs-update-check")
        .call()
        .ok()?
        .into_json()
        .ok()?;
    // The `cut-release` skill's contract: exactly one `.alfredworkflow`
    // asset per release. If that's ever violated, silently pick the
    // first match rather than guessing which one is right.
    let asset = release
        .assets
        .iter()
        .find(|a| a.name.ends_with(".alfredworkflow"))?;
    let version = release.tag_name.trim_start_matches('v').to_string();
    Some((version, asset.browser_download_url.clone()))
}

fn is_newer(latest: &str, current: &str) -> bool {
    match (
        semver::Version::parse(latest),
        semver::Version::parse(current),
    ) {
        (Ok(l), Ok(c)) => l > c,
        // Versions this project doesn't control the shape of (a malformed
        // tag) shouldn't crash the check — fall back to "different means
        // newer" so the item still surfaces rather than silently hiding.
        _ => latest != current,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_newer_compares_semver() {
        assert!(is_newer("0.2.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.2.0"));
    }

    #[test]
    fn is_newer_falls_back_to_string_inequality_on_unparsable_versions() {
        assert!(is_newer("not-semver", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.1.0"));
    }
}
