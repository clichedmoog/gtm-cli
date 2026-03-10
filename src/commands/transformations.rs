use clap::{Args, Subcommand};
use serde_json::json;

use crate::api::client::GtmApiClient;
use crate::api::params::params_from_json;
use crate::api::workspace::resolve_workspace;
use crate::error::{GtmError, Result};
use crate::output::formatter::{print_resource, OutputFormat};

#[derive(Args)]
pub struct TransformationsArgs {
    #[command(subcommand)]
    pub action: TransformationsAction,
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
pub enum TransformationsAction {
    /// List transformations (server-side)
    List(TransformationsListArgs),
    /// Get transformation details
    Get(TransformationsGetArgs),
    /// Create a transformation
    Create(TransformationsCreateArgs),
    /// Update a transformation
    Update(TransformationsUpdateArgs),
    /// Delete a transformation
    Delete(TransformationsDeleteArgs),
    /// Revert transformation changes
    Revert(TransformationsRevertArgs),
}

#[derive(Args)]
pub struct TransformationsListArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
}

#[derive(Args)]
pub struct TransformationsGetArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    transformation_id: String,
}

#[derive(Args)]
pub struct TransformationsCreateArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    name: String,
    #[arg(long = "type")]
    transformation_type: String,
    #[arg(long)]
    params: Option<String>,
}

#[derive(Args)]
pub struct TransformationsUpdateArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    transformation_id: String,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    params: Option<String>,
}

#[derive(Args)]
pub struct TransformationsDeleteArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    transformation_id: String,
    /// Required to confirm deletion
    #[arg(long)]
    force: bool,
}

#[derive(Args)]
pub struct TransformationsRevertArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    transformation_id: String,
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
    args: TransformationsArgs,
    client: &GtmApiClient,
    format: &OutputFormat,
) -> Result<()> {
    match args.action {
        TransformationsAction::List(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let result = client.get_all(&format!("{base}/transformations")).await?;
            print_resource(&result, format, "transformations");
        }
        TransformationsAction::Get(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let result = client
                .get(&format!("{base}/transformations/{}", a.transformation_id))
                .await?;
            print_resource(&result, format, "transformation");
        }
        TransformationsAction::Create(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let mut body = json!({
                "name": a.name,
                "type": a.transformation_type,
            });
            if let Some(p) = &a.params {
                let raw: serde_json::Value =
                    serde_json::from_str(p).map_err(|_| GtmError::InvalidParams(p.clone()))?;
                body["parameter"] = json!(params_from_json(&raw));
            }
            let result = client
                .post(&format!("{base}/transformations"), &body)
                .await?;
            print_resource(&result, format, "transformation");
        }
        TransformationsAction::Update(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let path = format!("{base}/transformations/{}", a.transformation_id);
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
            print_resource(&result, format, "transformation");
        }
        TransformationsAction::Delete(a) => {
            if !a.force {
                eprintln!(
                    "WARNING: This will permanently delete transformation '{}'.",
                    a.transformation_id
                );
                eprintln!("Run the same command with --force to confirm.");
                return Ok(());
            }
            let base = workspace_path(&a.ws, client).await?;
            client
                .delete(&format!("{base}/transformations/{}", a.transformation_id))
                .await?;
            crate::output::formatter::print_deleted("transformation", &a.transformation_id);
        }
        TransformationsAction::Revert(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let result = client
                .post(
                    &format!("{base}/transformations/{}:revert", a.transformation_id),
                    &json!({}),
                )
                .await?;
            print_resource(&result, format, "transformation");
        }
    }
    Ok(())
}
