use clap::{Args, Subcommand};

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

async fn workspace_path(ws: &WorkspaceFlags, client: &GtmApiClient) -> Result<String> {
    let ws_id = resolve_workspace(client, &ws.account_id, &ws.container_id, ws.workspace_id.as_deref()).await?;
    Ok(format!(
        "accounts/{}/containers/{}/workspaces/{}",
        ws.account_id, ws.container_id, ws_id
    ))
}

pub async fn handle(args: ZonesArgs, client: &GtmApiClient, format: &OutputFormat) -> Result<()> {
    match args.action {
        ZonesAction::List(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let result = client.get(&format!("{base}/zones")).await?;
            print_resource(&result, format, "zones");
        }
        ZonesAction::Get(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let result = client.get(&format!("{base}/zones/{}", a.zone_id)).await?;
            print_resource(&result, format, "zone");
        }
    }
    Ok(())
}
