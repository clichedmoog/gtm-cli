use clap::{Args, Subcommand};
use serde_json::json;

use crate::api::client::GtmApiClient;
use crate::api::params::{self, params_from_json};
use crate::api::workspace::resolve_workspace;
use crate::error::{GtmError, Result};
use crate::output::formatter::{print_resource, OutputFormat};

#[derive(Args)]
pub struct VariablesArgs {
    #[command(subcommand)]
    pub action: VariablesAction,
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
pub enum VariablesAction {
    /// List all variables
    List(VariablesListArgs),
    /// Get variable details
    Get(VariablesGetArgs),
    /// Create a new variable
    Create(VariablesCreateArgs),
    /// Update a variable
    Update(VariablesUpdateArgs),
    /// Delete a variable
    Delete(VariablesDeleteArgs),
    /// Revert variable changes
    Revert(VariablesRevertArgs),
}

#[derive(Args)]
pub struct VariablesListArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
}

#[derive(Args)]
pub struct VariablesGetArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    variable_id: String,
}

#[derive(Args)]
pub struct VariablesCreateArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    /// Variable name
    #[arg(long)]
    name: String,
    /// Variable type (e.g., v, c, jsm, k, gas)
    #[arg(long = "type")]
    variable_type: String,
    /// Variable value (uses type-specific parameter key)
    #[arg(long)]
    value: Option<String>,
    /// Variable parameters as JSON (advanced)
    #[arg(long)]
    params: Option<String>,
}

#[derive(Args)]
pub struct VariablesUpdateArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    variable_id: String,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    value: Option<String>,
    #[arg(long)]
    params: Option<String>,
}

#[derive(Args)]
pub struct VariablesDeleteArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    variable_id: String,
    /// Required to confirm deletion
    #[arg(long)]
    force: bool,
}

#[derive(Args)]
pub struct VariablesRevertArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    variable_id: String,
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
    args: VariablesArgs,
    client: &GtmApiClient,
    format: &OutputFormat,
) -> Result<()> {
    match args.action {
        VariablesAction::List(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let result = client.get_all(&format!("{base}/variables")).await?;
            print_resource(&result, format, "variables");
        }
        VariablesAction::Get(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let result = client
                .get(&format!("{base}/variables/{}", a.variable_id))
                .await?;
            print_resource(&result, format, "variable");
        }
        VariablesAction::Create(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let mut body = json!({
                "name": a.name,
                "type": a.variable_type,
            });

            if let Some(params_str) = &a.params {
                let raw: serde_json::Value = serde_json::from_str(params_str)
                    .map_err(|_| GtmError::InvalidParams(params_str.clone()))?;
                body["parameter"] = json!(params_from_json(&raw));
            } else if let Some(value) = &a.value {
                let key = params::get_variable_parameter_key(&a.variable_type);
                body["parameter"] = json!([{
                    "type": "template",
                    "key": key,
                    "value": value,
                }]);
            }

            let result = client.post(&format!("{base}/variables"), &body).await?;
            print_resource(&result, format, "variable");
        }
        VariablesAction::Update(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let path = format!("{base}/variables/{}", a.variable_id);
            let mut body = client.get(&path).await?;
            if let Some(name) = a.name {
                body["name"] = json!(name);
            }
            if let Some(params_str) = &a.params {
                let raw: serde_json::Value = serde_json::from_str(params_str)
                    .map_err(|_| GtmError::InvalidParams(params_str.clone()))?;
                body["parameter"] = json!(params_from_json(&raw));
            } else if let Some(value) = a.value {
                let var_type = body["type"].as_str().unwrap_or("c");
                let key = params::get_variable_parameter_key(var_type);
                body["parameter"] = json!([{
                    "type": "template",
                    "key": key,
                    "value": value,
                }]);
            }
            let result = client.put(&path, &body).await?;
            print_resource(&result, format, "variable");
        }
        VariablesAction::Delete(a) => {
            if !a.force {
                eprintln!(
                    "WARNING: This will permanently delete variable '{}'.",
                    a.variable_id
                );
                eprintln!("Run the same command with --force to confirm.");
                return Ok(());
            }
            let base = workspace_path(&a.ws, client).await?;
            client
                .delete(&format!("{base}/variables/{}", a.variable_id))
                .await?;
            crate::output::formatter::print_deleted("variable", &a.variable_id);
        }
        VariablesAction::Revert(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let result = client
                .post(
                    &format!("{base}/variables/{}:revert", a.variable_id),
                    &json!({}),
                )
                .await?;
            print_resource(&result, format, "variable");
        }
    }
    Ok(())
}
