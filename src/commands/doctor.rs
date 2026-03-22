use clap::Args;
use serde_json::json;

use crate::config::Config;
use crate::error::Result;
use crate::output::formatter::OutputFormat;

#[derive(Args)]
pub struct DoctorArgs {}

struct Check {
    name: &'static str,
    status: &'static str,
    message: String,
}

pub async fn handle(_args: DoctorArgs, format: &OutputFormat) -> Result<()> {
    let config = Config::load();
    let mut checks = Vec::new();

    // 1. Check credentials file
    let creds_path = &config.credentials_path;
    checks.push(if creds_path.exists() {
        Check {
            name: "credentials",
            status: "ok",
            message: format!("Found at {}", creds_path.display()),
        }
    } else {
        Check {
            name: "credentials",
            status: "missing",
            message: format!(
                "Not found at {}. Download from Google Cloud Console.",
                creds_path.display()
            ),
        }
    });

    // 2. Check token file (authenticated?)
    let token_path = &config.token_path;
    checks.push(if token_path.exists() {
        Check {
            name: "auth_token",
            status: "ok",
            message: "Authenticated".into(),
        }
    } else {
        Check {
            name: "auth_token",
            status: "missing",
            message: "Not authenticated. Run `gtm auth login`.".into(),
        }
    });

    // 3. Check config defaults
    let config_dir = Config::config_dir();
    let config_file = config_dir.join("config.json");
    let has_defaults = if config_file.exists() {
        let content = std::fs::read_to_string(&config_file).unwrap_or_default();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap_or(json!({}));
        parsed.get("defaultAccountId").is_some()
    } else {
        false
    };
    let env_account = std::env::var("GTM_ACCOUNT_ID").ok();
    checks.push(if has_defaults || env_account.is_some() {
        Check {
            name: "defaults",
            status: "ok",
            message: if env_account.is_some() {
                "Set via environment variables".into()
            } else {
                "Set via config file".into()
            },
        }
    } else {
        Check {
            name: "defaults",
            status: "warning",
            message:
                "No defaults configured. Run `gtm config setup` or set GTM_ACCOUNT_ID env var."
                    .into(),
        }
    });

    // 4. Check version
    let version = env!("CARGO_PKG_VERSION");
    checks.push(Check {
        name: "version",
        status: "ok",
        message: format!("v{version}"),
    });

    // Output
    let all_ok = checks.iter().all(|c| c.status == "ok");

    match format {
        OutputFormat::Json => {
            let items: Vec<_> = checks
                .iter()
                .map(|c| {
                    json!({
                        "check": c.name,
                        "status": c.status,
                        "message": c.message,
                    })
                })
                .collect();
            let output = json!({
                "checks": items,
                "healthy": all_ok,
            });
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Table | OutputFormat::Compact => {
            for c in &checks {
                let icon = match c.status {
                    "ok" => "✓",
                    "warning" => "!",
                    _ => "✗",
                };
                println!("  {icon} {}: {}", c.name, c.message);
            }
            println!();
            if all_ok {
                println!("All checks passed.");
            } else {
                println!("Some checks need attention.");
            }
        }
    }

    Ok(())
}
