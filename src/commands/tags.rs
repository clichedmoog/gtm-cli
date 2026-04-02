use clap::{Args, Subcommand};
use serde_json::json;

use crate::api::client::GtmApiClient;
use crate::api::params::{params_from_json, transform_event_params};
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
    /// Filter by name (substring match, case-insensitive)
    #[arg(long)]
    pub name: Option<String>,
    /// Filter by tag type (e.g., gaawe, html, gaawc)
    #[arg(long = "type")]
    pub tag_type: Option<String>,
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
    #[arg(long, conflicts_with = "params_file")]
    pub params: Option<String>,
    /// Read parameters from a JSON file instead of --params
    #[arg(long)]
    pub params_file: Option<String>,
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
    #[arg(long, conflicts_with = "params_file")]
    pub params: Option<String>,
    /// Read parameters from a JSON file instead of --params
    #[arg(long)]
    pub params_file: Option<String>,
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

fn resolve_params(
    params: &Option<String>,
    params_file: &Option<String>,
) -> Result<serde_json::Value> {
    if let Some(path) = params_file {
        let content = std::fs::read_to_string(path)
            .map_err(|e| GtmError::InvalidParams(format!("Cannot read {path}: {e}")))?;
        serde_json::from_str(&content)
            .map_err(|_| GtmError::InvalidParams(format!("Invalid JSON in {path}")))
    } else if let Some(p) = params {
        serde_json::from_str(p).map_err(|_| GtmError::InvalidParams(p.clone()))
    } else {
        Ok(json!({}))
    }
}

fn filter_resources(
    result: &mut serde_json::Value,
    key: &str,
    name: Option<&str>,
    type_filter: Option<&str>,
) {
    if let Some(arr) = result.get_mut(key).and_then(|v| v.as_array_mut()) {
        arr.retain(|item| {
            let name_match = name.is_none_or(|n| {
                item.get("name")
                    .and_then(|v| v.as_str())
                    .is_some_and(|s| s.to_lowercase().contains(&n.to_lowercase()))
            });
            let type_match = type_filter.is_none_or(|t| {
                item.get("type")
                    .and_then(|v| v.as_str())
                    .is_some_and(|s| s.eq_ignore_ascii_case(t))
            });
            name_match && type_match
        });
    }
}

pub async fn handle(args: TagsArgs, client: &GtmApiClient, format: &OutputFormat) -> Result<()> {
    match args.action {
        TagsAction::List(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let mut result = client.get_all(&format!("{base}/tags")).await?;
            if a.name.is_some() || a.tag_type.is_some() {
                filter_resources(&mut result, "tag", a.name.as_deref(), a.tag_type.as_deref());
            }
            print_resource(&result, format, "tags");
        }
        TagsAction::Get(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let result = client.get(&format!("{base}/tags/{}", a.tag_id)).await?;
            print_resource(&result, format, "tag");
        }
        TagsAction::Create(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let mut raw_params = resolve_params(&a.params, &a.params_file)?;
            if a.tag_type == "gaawe" {
                transform_event_params(&mut raw_params);
            }
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
            if a.params.is_some() || a.params_file.is_some() {
                let mut raw = resolve_params(&a.params, &a.params_file)?;
                let tag_type = body.get("type").and_then(|v| v.as_str()).unwrap_or("");
                if tag_type == "gaawe" {
                    transform_event_params(&mut raw);
                }
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
