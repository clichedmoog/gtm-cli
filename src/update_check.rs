use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const REPO: &str = "clichedmoog/gtmcli";
const CHECK_INTERVAL_SECS: i64 = 86400; // 24 hours

#[derive(Serialize, Deserialize, Default)]
struct UpdateCache {
    last_check: Option<i64>,
    latest_version: Option<String>,
}

fn cache_path() -> PathBuf {
    crate::config::Config::config_dir().join("update-check.json")
}

fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Check for updates in the background. Prints a message to stderr if a new version is available.
/// This function never blocks the main program — errors are silently ignored.
pub fn check_for_updates() {
    tokio::spawn(async { let _ = check_and_notify().await; });
}

async fn check_and_notify() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = cache_path();
    let now = chrono::Utc::now().timestamp();

    // Load cache
    let cache: UpdateCache = if path.exists() {
        let content = std::fs::read_to_string(&path)?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        UpdateCache::default()
    };

    // Check if we need to fetch (once per day)
    let should_fetch = match cache.last_check {
        Some(last) => now - last >= CHECK_INTERVAL_SECS,
        None => true,
    };

    let latest = if should_fetch {
        let version = fetch_latest_version().await?;
        let new_cache = UpdateCache {
            last_check: Some(now),
            latest_version: Some(version.clone()),
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, serde_json::to_string(&new_cache)?)?;
        version
    } else {
        match cache.latest_version {
            Some(v) => v,
            None => return Ok(()),
        }
    };

    let current = current_version();
    if latest != current && !current.contains("alpha") && !current.contains("beta") {
        eprintln!("\n  Update available: v{current} → v{latest}\n  Run `gtm upgrade` to update.\n");
    }

    Ok(())
}

async fn fetch_latest_version() -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("User-Agent", "gtm-cli")
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err("Failed to check for updates".into());
    }

    #[derive(Deserialize)]
    struct Release {
        tag_name: String,
    }

    let release: Release = resp.json().await?;
    Ok(release
        .tag_name
        .strip_prefix('v')
        .unwrap_or(&release.tag_name)
        .to_string())
}
