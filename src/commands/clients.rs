use clap::{Args, Subcommand};
use serde_json::json;

use crate::api::client::GtmApiClient;
use crate::api::params::params_from_json;
use crate::api::workspace::resolve_workspace;
use crate::error::{GtmError, Result};
use crate::output::formatter::{print_resource, OutputFormat};

#[derive(Args)]
pub struct ClientsArgs {
    #[command(subcommand)]
    pub action: ClientsAction,
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
pub enum ClientsAction {
    /// List all clients (server-side)
    List(ClientsListArgs),
    /// Get client details
    Get(ClientsGetArgs),
    /// Create a client
    Create(ClientsCreateArgs),
    /// Update a client
    Update(ClientsUpdateArgs),
    /// Delete a client
    Delete(ClientsDeleteArgs),
    /// Revert client changes
    Revert(ClientsRevertArgs),
}

#[derive(Args)]
pub struct ClientsListArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
}

#[derive(Args)]
pub struct ClientsGetArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    client_id: String,
}

#[derive(Args)]
pub struct ClientsCreateArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    name: String,
    #[arg(long = "type")]
    client_type: String,
    #[arg(long)]
    params: Option<String>,
}

#[derive(Args)]
pub struct ClientsUpdateArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    client_id: String,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    params: Option<String>,
}

#[derive(Args)]
pub struct ClientsDeleteArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    client_id: String,
    /// Required to confirm deletion
    #[arg(long)]
    force: bool,
}

#[derive(Args)]
pub struct ClientsRevertArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    client_id: String,
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

pub async fn handle(args: ClientsArgs, client: &GtmApiClient, format: &OutputFormat) -> Result<()> {
    match args.action {
        ClientsAction::List(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let result = client.get_all(&format!("{base}/clients")).await?;
            print_resource(&result, format, "clients");
        }
        ClientsAction::Get(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let result = client
                .get(&format!("{base}/clients/{}", a.client_id))
                .await?;
            print_resource(&result, format, "client");
        }
        ClientsAction::Create(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let mut body = json!({
                "name": a.name,
                "type": a.client_type,
            });
            if let Some(p) = &a.params {
                let raw: serde_json::Value =
                    serde_json::from_str(p).map_err(|_| GtmError::InvalidParams(p.clone()))?;
                body["parameter"] = json!(params_from_json(&raw));
            }
            let result = client.post(&format!("{base}/clients"), &body).await?;
            print_resource(&result, format, "client");
        }
        ClientsAction::Update(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let path = format!("{base}/clients/{}", a.client_id);
            let mut body = client.get(&path).await?;
            if let Some(name) = a.name {
                body["name"] = json!(name);
            }
            if let Some(p) = &a.params {
                let raw: serde_json::Value =
                    serde_json::from_str(p).map_err(|_| GtmError::InvalidParams(p.clone()))?;
                body["parameter"] = json!(params_from_json(&raw));
            }
            let result = client.put(&path, &body).await?;
            print_resource(&result, format, "client");
        }
        ClientsAction::Delete(a) => {
            if !a.force {
                eprintln!(
                    "WARNING: This will permanently delete client '{}'.",
                    a.client_id
                );
                eprintln!("Run the same command with --force to confirm.");
                return Ok(());
            }
            let base = workspace_path(&a.ws, client).await?;
            client
                .delete(&format!("{base}/clients/{}", a.client_id))
                .await?;
            crate::output::formatter::print_deleted("client", &a.client_id);
        }
        ClientsAction::Revert(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let result = client
                .post(
                    &format!("{base}/clients/{}:revert", a.client_id),
                    &json!({}),
                )
                .await?;
            print_resource(&result, format, "client");
        }
    }
    Ok(())
}
