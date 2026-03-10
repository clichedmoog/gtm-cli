use clap::Args;
use serde::Deserialize;
use std::env;
use std::fs;
use std::process::Command;

use crate::error::{GtmError, Result};

const REPO: &str = "clichedmoog/gtmcli";

#[derive(Args)]
pub struct UpgradeArgs {
    /// Only check for updates without installing
    #[arg(long)]
    pub check: bool,

    /// Force reinstall even if already on latest version
    #[arg(long)]
    pub force: bool,
}

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

fn target_triple() -> Result<&'static str> {
    if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        Ok("aarch64-apple-darwin")
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "x86_64") {
        Ok("x86_64-apple-darwin")
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
        Ok("x86_64-unknown-linux-gnu")
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "aarch64") {
        Ok("aarch64-unknown-linux-gnu")
    } else if cfg!(target_os = "windows") && cfg!(target_arch = "x86_64") {
        Ok("x86_64-pc-windows-msvc")
    } else {
        Err(GtmError::InvalidParams(format!(
            "Unsupported platform: {} {}",
            env::consts::OS,
            env::consts::ARCH
        )))
    }
}

async fn fetch_latest_release() -> Result<GitHubRelease> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("User-Agent", "gtm-cli")
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(GtmError::ApiError {
            status: resp.status().as_u16(),
            message: "Failed to check for updates".into(),
        });
    }

    let release: GitHubRelease = resp
        .json()
        .await
        .map_err(|e| GtmError::InvalidParams(format!("Failed to parse release info: {e}")))?;
    Ok(release)
}

fn parse_version(tag: &str) -> &str {
    tag.strip_prefix('v').unwrap_or(tag)
}

pub async fn handle(args: UpgradeArgs) -> Result<()> {
    let current = current_version();
    eprintln!("Current version: v{current}");

    eprintln!("Checking for updates...");
    let release = fetch_latest_release().await?;
    let latest = parse_version(&release.tag_name);

    if current == latest && !args.force {
        eprintln!("Already on the latest version (v{current}).");
        return Ok(());
    }

    if args.check {
        if current != latest {
            eprintln!("Update available: v{current} → v{latest}");
            eprintln!("Run `gtm upgrade` to install.");
        }
        return Ok(());
    }

    eprintln!("Upgrading: v{current} → v{latest}");

    let target = target_triple()?;
    let is_windows = cfg!(target_os = "windows");
    let ext = if is_windows { "zip" } else { "tar.gz" };
    let asset_name = format!("gtm-{target}.{ext}");

    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .ok_or_else(|| {
            GtmError::InvalidParams(format!("No binary found for {target} in release {latest}"))
        })?;

    // Download
    eprintln!("Downloading {asset_name}...");
    let client = reqwest::Client::new();
    let data = client
        .get(&asset.browser_download_url)
        .header("User-Agent", "gtm-cli")
        .send()
        .await?
        .bytes()
        .await?;

    // Find current binary path
    let current_exe = env::current_exe()
        .map_err(|e| GtmError::InvalidParams(format!("Cannot determine binary path: {e}")))?;

    let tmp_dir = env::temp_dir().join("gtm-upgrade");
    fs::create_dir_all(&tmp_dir)?;
    let archive_path = tmp_dir.join(&asset_name);
    fs::write(&archive_path, &data)?;

    // Extract
    if is_windows {
        Command::new("powershell")
            .args([
                "-Command",
                &format!(
                    "Expand-Archive -Force '{}' '{}'",
                    archive_path.display(),
                    tmp_dir.display()
                ),
            ])
            .status()
            .map_err(|e| GtmError::InvalidParams(format!("Extract failed: {e}")))?;
    } else {
        Command::new("tar")
            .args([
                "xzf",
                &archive_path.display().to_string(),
                "-C",
                &tmp_dir.display().to_string(),
            ])
            .status()
            .map_err(|e| GtmError::InvalidParams(format!("Extract failed: {e}")))?;
    }

    // Replace binary
    let bin_name = if is_windows { "gtm.exe" } else { "gtm" };
    let new_bin = tmp_dir.join(bin_name);

    if !new_bin.exists() {
        return Err(GtmError::InvalidParams("Extracted binary not found".into()));
    }

    // On Unix, replace by rename (atomic if same filesystem)
    // Back up current binary first
    let backup_path = current_exe.with_extension("old");
    if fs::rename(&current_exe, &backup_path).is_err() {
        // If rename fails (cross-device), try copy
        fs::copy(&current_exe, &backup_path)?;
    }

    match fs::rename(&new_bin, &current_exe) {
        Ok(_) => {}
        Err(_) => {
            // Cross-device: copy instead
            fs::copy(&new_bin, &current_exe)?;
        }
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&current_exe, fs::Permissions::from_mode(0o755))?;
    }

    // Cleanup
    let _ = fs::remove_dir_all(&tmp_dir);
    let _ = fs::remove_file(&backup_path);

    eprintln!("Upgraded to v{latest} successfully!");
    Ok(())
}
