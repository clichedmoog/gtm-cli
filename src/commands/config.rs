use clap::{Args, Subcommand};
use serde_json::json;
use std::io::{self, BufRead, Write};

use crate::api::client::GtmApiClient;
use crate::app_config::AppConfig;
use crate::config::Config;
use crate::error::Result;
use crate::output::formatter::{print_resource, OutputFormat};

#[derive(Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub action: ConfigAction,
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Interactive setup for default account, container, and workspace
    Setup,
    /// Show config value(s)
    Get(ConfigGetArgs),
    /// Set a config value
    Set(ConfigSetArgs),
    /// Remove a config value
    Unset(ConfigUnsetArgs),
}

#[derive(Args)]
pub struct ConfigGetArgs {
    /// Config key (omit to show all)
    pub key: Option<String>,
}

#[derive(Args)]
pub struct ConfigSetArgs {
    /// Config key
    pub key: String,
    /// Config value
    pub value: String,
}

#[derive(Args)]
pub struct ConfigUnsetArgs {
    /// Config key
    pub key: String,
}

pub async fn handle(
    args: ConfigArgs,
    client: Option<&GtmApiClient>,
    _config: &Config,
    format: &OutputFormat,
) -> Result<()> {
    let config_path = Config::config_dir().join("config.json");

    match args.action {
        ConfigAction::Setup => {
            let client = client.expect("Setup requires authentication");
            let mut app_config = AppConfig::load(&config_path);

            // 1. Select account
            let accounts_result = client.get("accounts").await?;
            if let Some(accounts) = accounts_result.get("account").and_then(|a| a.as_array()) {
                eprintln!("\nAvailable accounts:");
                for (i, acc) in accounts.iter().enumerate() {
                    let name = acc["name"].as_str().unwrap_or("?");
                    let id = acc["accountId"].as_str().unwrap_or("?");
                    eprintln!("  [{}] {} ({})", i + 1, name, id);
                }
                if let Some(choice) = prompt_choice("Select account", accounts.len())? {
                    let account_id = accounts[choice]["accountId"]
                        .as_str()
                        .unwrap_or("")
                        .to_string();
                    app_config.default_account_id = Some(account_id.clone());
                    eprintln!("Set defaultAccountId = {account_id}");

                    // 2. Select container
                    let containers_result = client
                        .get(&format!("accounts/{account_id}/containers"))
                        .await?;
                    if let Some(containers) = containers_result
                        .get("container")
                        .and_then(|c| c.as_array())
                    {
                        eprintln!("\nAvailable containers:");
                        for (i, c) in containers.iter().enumerate() {
                            let name = c["name"].as_str().unwrap_or("?");
                            let id = c["containerId"].as_str().unwrap_or("?");
                            let public_id = c["publicId"].as_str().unwrap_or("");
                            eprintln!("  [{}] {} ({}) {}", i + 1, name, id, public_id);
                        }
                        if let Some(choice) = prompt_choice("Select container", containers.len())? {
                            let container_id = containers[choice]["containerId"]
                                .as_str()
                                .unwrap_or("")
                                .to_string();
                            app_config.default_container_id = Some(container_id.clone());
                            eprintln!("Set defaultContainerId = {container_id}");

                            // 3. Select workspace
                            let ws_result = client
                                .get(&format!(
                                    "accounts/{account_id}/containers/{container_id}/workspaces"
                                ))
                                .await?;
                            if let Some(workspaces) =
                                ws_result.get("workspace").and_then(|w| w.as_array())
                            {
                                eprintln!("\nAvailable workspaces:");
                                for (i, ws) in workspaces.iter().enumerate() {
                                    let name = ws["name"].as_str().unwrap_or("?");
                                    let id = ws["workspaceId"].as_str().unwrap_or("?");
                                    eprintln!("  [{}] {} ({})", i + 1, name, id);
                                }
                                if let Some(choice) =
                                    prompt_choice("Select workspace", workspaces.len())?
                                {
                                    let ws_id = workspaces[choice]["workspaceId"]
                                        .as_str()
                                        .unwrap_or("")
                                        .to_string();
                                    app_config.default_workspace_id = Some(ws_id.clone());
                                    eprintln!("Set defaultWorkspaceId = {ws_id}");
                                }
                            }
                        }
                    }
                }
            }

            app_config.save(&config_path)?;
            eprintln!("\nConfiguration saved.");
            eprintln!("Tip: You can also set env vars for per-session overrides:");
            eprintln!("  export GTM_ACCOUNT_ID=...");
            eprintln!("  export GTM_CONTAINER_ID=...");
            eprintln!("  export GTM_WORKSPACE_ID=...");
        }

        ConfigAction::Get(a) => {
            let app_config = AppConfig::load(&config_path);
            if let Some(key) = a.key {
                match app_config.get(&key) {
                    Some(val) => println!("{val}"),
                    None => eprintln!("{key}: not set"),
                }
            } else {
                let value = json!(&app_config);
                print_resource(&value, format, "config");
            }
        }

        ConfigAction::Set(a) => {
            let mut app_config = AppConfig::load(&config_path);
            app_config.set(&a.key, a.value.clone())?;
            app_config.save(&config_path)?;
            eprintln!("Set {} = {}", a.key, a.value);
        }

        ConfigAction::Unset(a) => {
            let mut app_config = AppConfig::load(&config_path);
            app_config.unset(&a.key)?;
            app_config.save(&config_path)?;
            eprintln!("Removed {}", a.key);
        }
    }
    Ok(())
}

fn prompt_choice(label: &str, max: usize) -> Result<Option<usize>> {
    print!("{label} [1-{max}]: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().lock().read_line(&mut input)?;
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    match trimmed.parse::<usize>() {
        Ok(n) if n >= 1 && n <= max => Ok(Some(n - 1)),
        _ => {
            eprintln!("Invalid choice, skipping.");
            Ok(None)
        }
    }
}
