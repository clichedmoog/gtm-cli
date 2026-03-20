use clap::{Args, Subcommand};
use serde_json::json;

use crate::api::client::GtmApiClient;
use crate::api::workspace::resolve_workspace;
use crate::error::Result;
use crate::output::formatter::{print_resource, OutputFormat};

#[derive(Args)]
pub struct ZonesArgs {
    #[command(subcommand)]
    pub action: ZonesAction,
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
pub enum ZonesAction {
    /// List zones (server-side)
    List(ZonesListArgs),
    /// Get zone details
    Get(ZonesGetArgs),
    /// Create a new zone
    Create(ZonesCreateArgs),
    /// Update a zone
    Update(ZonesUpdateArgs),
    /// Delete a zone
    Delete(ZonesDeleteArgs),
    /// Revert zone changes
    Revert(ZonesRevertArgs),
}

#[derive(Args)]
pub struct ZonesListArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
}

#[derive(Args)]
pub struct ZonesGetArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    zone_id: String,
}

#[derive(Args)]
pub struct ZonesCreateArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    /// Zone name
    #[arg(long)]
    name: String,
    /// Child containers as JSON array (e.g. '[{"publicId":"GTM-XXXX","nickname":"Child"}]')
    #[arg(long)]
    child_container: Option<String>,
    /// Zone boundary type and conditions as JSON
    #[arg(long)]
    boundary: Option<String>,
}

#[derive(Args)]
pub struct ZonesUpdateArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    zone_id: String,
    /// Zone name
    #[arg(long)]
    name: Option<String>,
    /// Child containers as JSON array
    #[arg(long)]
    child_container: Option<String>,
    /// Zone boundary type and conditions as JSON
    #[arg(long)]
    boundary: Option<String>,
}

#[derive(Args)]
pub struct ZonesDeleteArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    zone_id: String,
    /// Required to confirm deletion
    #[arg(long)]
    force: bool,
}

#[derive(Args)]
pub struct ZonesRevertArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    zone_id: String,
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

pub async fn handle(args: ZonesArgs, client: &GtmApiClient, format: &OutputFormat) -> Result<()> {
    match args.action {
        ZonesAction::List(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let result = client.get_all(&format!("{base}/zones")).await?;
            print_resource(&result, format, "zones");
        }
        ZonesAction::Get(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let result = client.get(&format!("{base}/zones/{}", a.zone_id)).await?;
            print_resource(&result, format, "zone");
        }
        ZonesAction::Create(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let mut body = json!({ "name": a.name });
            if let Some(child) = a.child_container {
                let parsed: serde_json::Value = serde_json::from_str(&child)
                    .map_err(|_| crate::error::GtmError::InvalidParams(child))?;
                body["childContainer"] = parsed;
            }
            if let Some(boundary) = a.boundary {
                let parsed: serde_json::Value = serde_json::from_str(&boundary)
                    .map_err(|_| crate::error::GtmError::InvalidParams(boundary))?;
                body["boundary"] = parsed;
            }
            let result = client.post(&format!("{base}/zones"), &body).await?;
            print_resource(&result, format, "zone");
        }
        ZonesAction::Update(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let path = format!("{base}/zones/{}", a.zone_id);
            let mut body = client.get(&path).await?;
            if let Some(name) = a.name {
                body["name"] = json!(name);
            }
            if let Some(child) = a.child_container {
                let parsed: serde_json::Value = serde_json::from_str(&child)
                    .map_err(|_| crate::error::GtmError::InvalidParams(child))?;
                body["childContainer"] = parsed;
            }
            if let Some(boundary) = a.boundary {
                let parsed: serde_json::Value = serde_json::from_str(&boundary)
                    .map_err(|_| crate::error::GtmError::InvalidParams(boundary))?;
                body["boundary"] = parsed;
            }
            let result = client.put(&path, &body).await?;
            print_resource(&result, format, "zone");
        }
        ZonesAction::Delete(a) => {
            if !a.force {
                eprintln!(
                    "WARNING: This will permanently delete zone '{}'.",
                    a.zone_id
                );
                eprintln!("Run the same command with --force to confirm.");
                return Ok(());
            }
            let base = workspace_path(&a.ws, client).await?;
            client
                .delete(&format!("{base}/zones/{}", a.zone_id))
                .await?;
            crate::output::formatter::print_deleted("zone", &a.zone_id);
        }
        ZonesAction::Revert(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let result = client
                .post(&format!("{base}/zones/{}:revert", a.zone_id), &json!({}))
                .await?;
            print_resource(&result, format, "zone");
        }
    }
    Ok(())
}
