use clap::{Args, Subcommand};
use serde_json::json;

use crate::api::client::GtmApiClient;
use crate::api::workspace::resolve_workspace;
use crate::error::{GtmError, Result};
use crate::output::formatter::{print_resource, OutputFormat};

#[derive(Args)]
pub struct TriggersArgs {
    #[command(subcommand)]
    pub action: TriggersAction,
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
pub enum TriggersAction {
    /// List all triggers
    List(TriggersListArgs),
    /// Get trigger details
    Get(TriggersGetArgs),
    /// Create a new trigger
    Create(TriggersCreateArgs),
    /// Update a trigger
    Update(TriggersUpdateArgs),
    /// Delete a trigger
    Delete(TriggersDeleteArgs),
    /// Revert trigger changes
    Revert(TriggersRevertArgs),
}

#[derive(Args)]
pub struct TriggersListArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
}

#[derive(Args)]
pub struct TriggersGetArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    trigger_id: String,
}

#[derive(Args)]
pub struct TriggersCreateArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    /// Trigger name
    #[arg(long)]
    name: String,
    /// Trigger type (e.g., pageview, click, customEvent, formSubmission)
    #[arg(long = "type")]
    trigger_type: String,
    /// Custom event name (for customEvent type)
    #[arg(long)]
    custom_event_filter: Option<String>,
    /// Filter conditions as JSON array
    #[arg(long)]
    filter: Option<String>,
}

#[derive(Args)]
pub struct TriggersUpdateArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    trigger_id: String,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    filter: Option<String>,
}

#[derive(Args)]
pub struct TriggersDeleteArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    trigger_id: String,
    /// Required to confirm deletion
    #[arg(long)]
    force: bool,
}

#[derive(Args)]
pub struct TriggersRevertArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    trigger_id: String,
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
    args: TriggersArgs,
    client: &GtmApiClient,
    format: &OutputFormat,
) -> Result<()> {
    match args.action {
        TriggersAction::List(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let result = client.get_all(&format!("{base}/triggers")).await?;
            print_resource(&result, format, "triggers");
        }
        TriggersAction::Get(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let result = client
                .get(&format!("{base}/triggers/{}", a.trigger_id))
                .await?;
            print_resource(&result, format, "trigger");
        }
        TriggersAction::Create(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let mut body = json!({
                "name": a.name,
                "type": a.trigger_type,
            });
            if let Some(event_filter) = a.custom_event_filter {
                body["customEventFilter"] = json!([{
                    "type": "equals",
                    "parameter": [
                        {"type": "template", "key": "arg0", "value": "{{_event}}"},
                        {"type": "template", "key": "arg1", "value": event_filter}
                    ]
                }]);
            }
            if let Some(filter) = a.filter {
                let parsed: serde_json::Value =
                    serde_json::from_str(&filter).map_err(|_| GtmError::InvalidParams(filter))?;
                body["filter"] = parsed;
            }
            let result = client.post(&format!("{base}/triggers"), &body).await?;
            print_resource(&result, format, "trigger");
        }
        TriggersAction::Update(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let path = format!("{base}/triggers/{}", a.trigger_id);
            let mut body = client.get(&path).await?;
            if let Some(name) = a.name {
                body["name"] = json!(name);
            }
            if let Some(filter) = a.filter {
                let parsed: serde_json::Value =
                    serde_json::from_str(&filter).map_err(|_| GtmError::InvalidParams(filter))?;
                body["filter"] = parsed;
            }
            let result = client.put(&path, &body).await?;
            print_resource(&result, format, "trigger");
        }
        TriggersAction::Delete(a) => {
            if !a.force {
                eprintln!(
                    "WARNING: This will permanently delete trigger '{}'.",
                    a.trigger_id
                );
                eprintln!("Run the same command with --force to confirm.");
                return Ok(());
            }
            let base = workspace_path(&a.ws, client).await?;
            client
                .delete(&format!("{base}/triggers/{}", a.trigger_id))
                .await?;
            crate::output::formatter::print_deleted("trigger", &a.trigger_id);
        }
        TriggersAction::Revert(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let result = client
                .post(
                    &format!("{base}/triggers/{}:revert", a.trigger_id),
                    &json!({}),
                )
                .await?;
            print_resource(&result, format, "trigger");
        }
    }
    Ok(())
}
