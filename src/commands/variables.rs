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
    /// Filter by name (substring match, case-insensitive)
    #[arg(long)]
    name: Option<String>,
    /// Filter by variable type (e.g., v, c, jsm, k)
    #[arg(long = "type")]
    variable_type: Option<String>,
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
    #[arg(long, conflicts_with = "params_file")]
    params: Option<String>,
    /// Read parameters from a JSON file instead of --params
    #[arg(long)]
    params_file: Option<String>,
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
    #[arg(long, conflicts_with = "params_file")]
    params: Option<String>,
    /// Read parameters from a JSON file instead of --params
    #[arg(long)]
    params_file: Option<String>,
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

fn resolve_params(
    params: &Option<String>,
    params_file: &Option<String>,
) -> std::result::Result<Option<serde_json::Value>, GtmError> {
    if let Some(path) = params_file {
        let content = std::fs::read_to_string(path)
            .map_err(|e| GtmError::InvalidParams(format!("Cannot read {path}: {e}")))?;
        let val = serde_json::from_str(&content)
            .map_err(|_| GtmError::InvalidParams(format!("Invalid JSON in {path}")))?;
        Ok(Some(val))
    } else if let Some(p) = params {
        let val = serde_json::from_str(p).map_err(|_| GtmError::InvalidParams(p.clone()))?;
        Ok(Some(val))
    } else {
        Ok(None)
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

pub async fn handle(
    args: VariablesArgs,
    client: &GtmApiClient,
    format: &OutputFormat,
) -> Result<()> {
    match args.action {
        VariablesAction::List(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let mut result = client.get_all(&format!("{base}/variables")).await?;
            if a.name.is_some() || a.variable_type.is_some() {
                filter_resources(
                    &mut result,
                    "variable",
                    a.name.as_deref(),
                    a.variable_type.as_deref(),
                );
            }
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

            if let Some(raw) = resolve_params(&a.params, &a.params_file)? {
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
            if let Some(raw) = resolve_params(&a.params, &a.params_file)? {
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
