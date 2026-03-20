use clap::{Args, Subcommand};
use serde_json::json;

use crate::api::client::GtmApiClient;
use crate::api::params::params_from_json;
use crate::api::workspace::resolve_workspace;
use crate::error::{GtmError, Result};
use crate::output::formatter::{print_resource, OutputFormat};

#[derive(Args)]
pub struct TagsArgs {
    #[command(subcommand)]
    pub action: TagsAction,
}

#[derive(Args)]
pub struct WorkspaceFlags {
    #[arg(long, env = "GTM_ACCOUNT_ID")]
    account_id: String,
    #[arg(long, env = "GTM_CONTAINER_ID")]
    container_id: String,
    #[arg(long, env = "GTM_WORKSPACE_ID")]
    workspace_id: Option<String>,
}

#[derive(Subcommand)]
pub enum TagsAction {
    /// List all tags
    List(TagsListArgs),
    /// Get tag details
    Get(TagsGetArgs),
    /// Create a new tag
    Create(TagsCreateArgs),
    /// Update a tag
    Update(TagsUpdateArgs),
    /// Delete a tag
    Delete(TagsDeleteArgs),
    /// Revert tag changes
    Revert(TagsRevertArgs),
}

#[derive(Args)]
pub struct TagsListArgs {
    #[command(flatten)]
    pub ws: WorkspaceFlags,
}

#[derive(Args)]
pub struct TagsGetArgs {
    #[command(flatten)]
    pub ws: WorkspaceFlags,
    /// Tag ID
    #[arg(long)]
    pub tag_id: String,
}

#[derive(Args)]
pub struct TagsCreateArgs {
    #[command(flatten)]
    pub ws: WorkspaceFlags,
    /// Tag name
    #[arg(long)]
    pub name: String,
    /// Tag type (e.g., gaawc, gaawe, html, img)
    #[arg(long = "type")]
    pub tag_type: String,
    /// Tag parameters as JSON (e.g., '{"measurementId":"G-XXX"}')
    #[arg(long)]
    pub params: Option<String>,
    /// Firing trigger IDs (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub firing_trigger_id: Vec<String>,
    /// Blocking trigger IDs (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub blocking_trigger_id: Vec<String>,
}

#[derive(Args)]
pub struct TagsUpdateArgs {
    #[command(flatten)]
    pub ws: WorkspaceFlags,
    /// Tag ID to update
    #[arg(long)]
    pub tag_id: String,
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long)]
    pub params: Option<String>,
    #[arg(long, value_delimiter = ',')]
    pub firing_trigger_id: Vec<String>,
    #[arg(long, value_delimiter = ',')]
    pub blocking_trigger_id: Vec<String>,
}

#[derive(Args)]
pub struct TagsDeleteArgs {
    #[command(flatten)]
    pub ws: WorkspaceFlags,
    #[arg(long)]
    pub tag_id: String,
    /// Required to confirm deletion
    #[arg(long)]
    pub force: bool,
}

#[derive(Args)]
pub struct TagsRevertArgs {
    #[command(flatten)]
    pub ws: WorkspaceFlags,
    #[arg(long)]
    pub tag_id: String,
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

fn parse_params(params: &Option<String>) -> Result<serde_json::Value> {
    match params {
        Some(p) => serde_json::from_str(p).map_err(|_| GtmError::InvalidParams(p.clone())),
        None => Ok(json!({})),
    }
}

pub async fn handle(args: TagsArgs, client: &GtmApiClient, format: &OutputFormat) -> Result<()> {
    match args.action {
        TagsAction::List(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let result = client.get_all(&format!("{base}/tags")).await?;
            print_resource(&result, format, "tags");
        }
        TagsAction::Get(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let result = client.get(&format!("{base}/tags/{}", a.tag_id)).await?;
            print_resource(&result, format, "tag");
        }
        TagsAction::Create(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let raw_params = parse_params(&a.params)?;
            let parameters = params_from_json(&raw_params);

            let mut body = json!({
                "name": a.name,
                "type": a.tag_type,
                "parameter": parameters,
            });
            if !a.firing_trigger_id.is_empty() {
                body["firingTriggerId"] = json!(a.firing_trigger_id);
            }
            if !a.blocking_trigger_id.is_empty() {
                body["blockingTriggerId"] = json!(a.blocking_trigger_id);
            }

            let result = client.post(&format!("{base}/tags"), &body).await?;
            print_resource(&result, format, "tag");
        }
        TagsAction::Update(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let path = format!("{base}/tags/{}", a.tag_id);
            // GET existing tag first, then merge changes (GTM API PUT = full replace)
            let mut body = client.get(&path).await?;
            if let Some(name) = a.name {
                body["name"] = json!(name);
            }
            if let Some(params) = &a.params {
                let raw: serde_json::Value = serde_json::from_str(params)
                    .map_err(|_| GtmError::InvalidParams(params.clone()))?;
                body["parameter"] = json!(params_from_json(&raw));
            }
            if !a.firing_trigger_id.is_empty() {
                body["firingTriggerId"] = json!(a.firing_trigger_id);
            }
            if !a.blocking_trigger_id.is_empty() {
                body["blockingTriggerId"] = json!(a.blocking_trigger_id);
            }

            let result = client.put(&path, &body).await?;
            print_resource(&result, format, "tag");
        }
        TagsAction::Delete(a) => {
            if !a.force {
                eprintln!("WARNING: This will permanently delete tag '{}'.", a.tag_id);
                eprintln!("Run the same command with --force to confirm.");
                return Ok(());
            }
            let base = workspace_path(&a.ws, client).await?;
            client.delete(&format!("{base}/tags/{}", a.tag_id)).await?;
            crate::output::formatter::print_deleted("tag", &a.tag_id);
        }
        TagsAction::Revert(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let result = client
                .post(&format!("{base}/tags/{}:revert", a.tag_id), &json!({}))
                .await?;
            print_resource(&result, format, "tag");
        }
    }
    Ok(())
}
