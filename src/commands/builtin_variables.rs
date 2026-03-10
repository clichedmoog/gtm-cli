use clap::{Args, Subcommand};
use serde_json::json;

use crate::api::client::GtmApiClient;
use crate::api::workspace::resolve_workspace;
use crate::error::Result;
use crate::output::formatter::{print_resource, OutputFormat};

#[derive(Args)]
pub struct BuiltinVariablesArgs {
    #[command(subcommand)]
    pub action: BuiltinVariablesAction,
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
pub enum BuiltinVariablesAction {
    /// List enabled built-in variables
    List(BuiltinVarsListArgs),
    /// Enable a built-in variable
    Create(BuiltinVarsCreateArgs),
    /// Disable a built-in variable
    Delete(BuiltinVarsDeleteArgs),
    /// Revert all built-in variable changes
    Revert(BuiltinVarsRevertArgs),
}

#[derive(Args)]
pub struct BuiltinVarsListArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
}

#[derive(Args)]
pub struct BuiltinVarsCreateArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    /// Built-in variable types to enable (comma-separated)
    /// e.g., pageUrl,pageHostname,pagePath,referrer,event
    #[arg(long = "type", value_delimiter = ',')]
    variable_types: Vec<String>,
}

#[derive(Args)]
pub struct BuiltinVarsDeleteArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    /// Built-in variable types to disable (comma-separated)
    #[arg(long = "type", value_delimiter = ',')]
    variable_types: Vec<String>,
}

#[derive(Args)]
pub struct BuiltinVarsRevertArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
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
    args: BuiltinVariablesArgs,
    client: &GtmApiClient,
    format: &OutputFormat,
) -> Result<()> {
    match args.action {
        BuiltinVariablesAction::List(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let result = client
                .get_all(&format!("{base}/built_in_variables"))
                .await?;
            print_resource(&result, format, "built_in_variables");
        }
        BuiltinVariablesAction::Create(a) => {
            let base = workspace_path(&a.ws, client).await?;
            // Built-in variables are enabled via query parameter type=
            let query: Vec<(&str, &str)> = a
                .variable_types
                .iter()
                .map(|t| ("type", t.as_str()))
                .collect();
            let result = client
                .post_with_query(&format!("{base}/built_in_variables"), &query, &json!({}))
                .await?;
            print_resource(&result, format, "built_in_variables");
        }
        BuiltinVariablesAction::Delete(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let query: Vec<(&str, &str)> = a
                .variable_types
                .iter()
                .map(|t| ("type", t.as_str()))
                .collect();
            client
                .delete_with_query(&format!("{base}/built_in_variables"), &query)
                .await?;
            eprintln!("Built-in variables disabled successfully.");
        }
        BuiltinVariablesAction::Revert(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let result = client
                .post(&format!("{base}/built_in_variables:revert"), &json!({}))
                .await?;
            print_resource(&result, format, "built_in_variables");
        }
    }
    Ok(())
}
