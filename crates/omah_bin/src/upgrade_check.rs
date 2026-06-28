use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const CACHE_TTL_SECS: u64 = 3600;
const CACHE_FILE: &str = ".version-cache";
const GITHUB_API: &str =
    "https://api.github.com/repos/brilyyy/omah/releases/latest";

fn cache_path() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(
        PathBuf::from(home)
            .join(".config/omah")
            .join(CACHE_FILE),
    )
}

fn read_cache() -> Option<String> {
    let path = cache_path()?;
    let content = std::fs::read_to_string(path).ok()?;
    let mut lines = content.lines();
    let ts: u64 = lines.next()?.parse().ok()?;
    let tag = lines.next()?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_secs();
    if now.saturating_sub(ts) < CACHE_TTL_SECS {
        Some(tag.to_string())
    } else {
        None
    }
}

fn write_cache(tag: &str) {
    if let Some(path) = cache_path() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) {
            let _ = std::fs::write(path, format!("{}\n{}\n", now.as_secs(), tag));
        }
    }
}

fn fetch_latest_tag() -> Option<String> {
    let output = Command::new("curl")
        .args([
            "-sS",
            "-L",
            "--connect-timeout",
            "10",
            "--max-time",
            "15",
            "-H",
            "Accept: application/vnd.github+json",
            "-H",
            "User-Agent: omah-upgrade",
            GITHUB_API,
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    if json.get("message").and_then(|m| m.as_str()).is_some() {
        return None;
    }
    json["tag_name"].as_str().map(|s| s.to_string())
}

pub fn check_for_upgrade() -> Option<String> {
    if let Some(tag) = read_cache() {
        if omah_lib::upgrade::is_newer(&tag, omah_lib::upgrade::CURRENT_VERSION) {
            return Some(tag);
        }
        return None;
    }

    let tag = match fetch_latest_tag() {
        Some(t) => t,
        None => return None,
    };

    write_cache(&tag);

    if omah_lib::upgrade::is_newer(&tag, omah_lib::upgrade::CURRENT_VERSION) {
        Some(tag)
    } else {
        None
    }
}
