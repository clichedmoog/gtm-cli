use clap::{Args, Subcommand};
use serde_json::json;

use crate::api::client::GtmApiClient;
use crate::api::params::params_from_json;
use crate::api::workspace::resolve_workspace;
use crate::error::{GtmError, Result};
use crate::output::formatter::{print_resource, OutputFormat};

#[derive(Args)]
pub struct GtagConfigsArgs {
    #[command(subcommand)]
    pub action: GtagConfigsAction,
}

#[derive(Args)]
struct WorkspaceFlags {
    #[arg(long, env = "GTM_ACCOUNT_ID")]
    account_id: String,
    #[arg(long, env = "GTM_CONTAINER_ID")]
    container_id: String,
    #[arg(long, env = "GTM_WORKSPACE_ID")]
    workspace_id: Option<String>,
}

#[derive(Subcommand)]
pub enum GtagConfigsAction {
    /// List Google Tag configs
    List(GtagListArgs),
    /// Get config details
    Get(GtagGetArgs),
    /// Create a Google Tag config
    Create(GtagCreateArgs),
    /// Update a Google Tag config
    Update(GtagUpdateArgs),
    /// Delete a Google Tag config
    Delete(GtagDeleteArgs),
    /// Revert config changes
    Revert(GtagRevertArgs),
}

#[derive(Args)]
pub struct GtagListArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
}

#[derive(Args)]
pub struct GtagGetArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    gtag_config_id: String,
}

#[derive(Args)]
pub struct GtagCreateArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    /// Measurement ID (e.g., G-XXXXXXX)
    #[arg(long)]
    measurement_id: String,
    #[arg(long)]
    params: Option<String>,
}

#[derive(Args)]
pub struct GtagUpdateArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    gtag_config_id: String,
    #[arg(long)]
    measurement_id: Option<String>,
    #[arg(long)]
    params: Option<String>,
}

#[derive(Args)]
pub struct GtagDeleteArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    gtag_config_id: String,
    /// Required to confirm deletion
    #[arg(long)]
    force: bool,
}

#[derive(Args)]
pub struct GtagRevertArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    gtag_config_id: String,
}

async fn workspace_path(ws: &WorkspaceFlags, client: &GtmApiClient) -> Result<String> {
    let ws_id = resolve_workspace(
        client,
        &ws.account_id,
        &ws.container_id,
        ws.workspace_id.as_deref(),
    )
    .await?;
    Ok(format!(
        "accounts/{}/containers/{}/workspaces/{}",
        ws.account_id, ws.container_id, ws_id
    ))
}

pub async fn handle(
    args: GtagConfigsArgs,
    client: &GtmApiClient,
    format: &OutputFormat,
) -> Result<()> {
    match args.action {
        GtagConfigsAction::List(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let result = client.get_all(&format!("{base}/gtag_config")).await?;
            print_resource(&result, format, "gtag_configs");
        }
        GtagConfigsAction::Get(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let result = client
                .get(&format!("{base}/gtag_config/{}", a.gtag_config_id))
                .await?;
            print_resource(&result, format, "gtag_config");
        }
        GtagConfigsAction::Create(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let mut body = json!({
                "measurementId": a.measurement_id,
                "type": "googtag",
            });
            if let Some(p) = &a.params {
                let raw: serde_json::Value =
                    serde_json::from_str(p).map_err(|_| GtmError::InvalidParams(p.clone()))?;
                body["parameter"] = json!(params_from_json(&raw));
            }
            let result = client.post(&format!("{base}/gtag_config"), &body).await?;
            print_resource(&result, format, "gtag_config");
        }
        GtagConfigsAction::Update(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let path = format!("{base}/gtag_config/{}", a.gtag_config_id);
            let mut body = client.get(&path).await?;
            if let Some(mid) = a.measurement_id {
                body["measurementId"] = json!(mid);
            }
            if let Some(p) = &a.params {
                let raw: serde_json::Value =
                    serde_json::from_str(p).map_err(|_| GtmError::InvalidParams(p.clone()))?;
                body["parameter"] = json!(params_from_json(&raw));
            }
            let result = client.put(&path, &body).await?;
            print_resource(&result, format, "gtag_config");
        }
        GtagConfigsAction::Delete(a) => {
            if !a.force {
                eprintln!(
                    "WARNING: This will permanently delete gtag config '{}'.",
                    a.gtag_config_id
                );
                eprintln!("Run the same command with --force to confirm.");
                return Ok(());
            }
            let base = workspace_path(&a.ws, client).await?;
            client
                .delete(&format!("{base}/gtag_config/{}", a.gtag_config_id))
                .await?;
            crate::output::formatter::print_deleted("gtag_config", &a.gtag_config_id);
        }
        GtagConfigsAction::Revert(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let result = client
                .post(
                    &format!("{base}/gtag_config/{}:revert", a.gtag_config_id),
                    &json!({}),
                )
                .await?;
            print_resource(&result, format, "gtag_config");
        }
    }
    Ok(())
}
