use clap::{Args, Subcommand};
use serde_json::json;

use crate::api::client::GtmApiClient;
use crate::api::workspace::resolve_workspace;
use crate::error::Result;
use crate::output::formatter::{print_resource, OutputFormat};

#[derive(Args)]
pub struct TemplatesArgs {
    #[command(subcommand)]
    pub action: TemplatesAction,
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
pub enum TemplatesAction {
    /// List all templates
    List(TemplatesListArgs),
    /// Get template details
    Get(TemplatesGetArgs),
    /// Create a new template
    Create(TemplatesCreateArgs),
    /// Update a template
    Update(TemplatesUpdateArgs),
    /// Delete a template
    Delete(TemplatesDeleteArgs),
    /// Revert template changes
    Revert(TemplatesRevertArgs),
    /// Import a template from the Community Template Gallery
    Import(TemplatesImportArgs),
}

#[derive(Args)]
pub struct TemplatesListArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
}

#[derive(Args)]
pub struct TemplatesGetArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    template_id: String,
}

#[derive(Args)]
pub struct TemplatesCreateArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    name: String,
    /// Template data as JSON
    #[arg(long)]
    template_data: Option<String>,
}

#[derive(Args)]
pub struct TemplatesUpdateArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    template_id: String,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    template_data: Option<String>,
}

#[derive(Args)]
pub struct TemplatesDeleteArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    template_id: String,
    /// Required to confirm deletion
    #[arg(long)]
    force: bool,
}

#[derive(Args)]
pub struct TemplatesRevertArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    template_id: String,
}

#[derive(Args)]
pub struct TemplatesImportArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    /// Gallery host (e.g., github.com)
    #[arg(long, default_value = "github.com")]
    host: String,
    /// Repository owner (e.g., user or org name)
    #[arg(long)]
    owner: String,
    /// Repository name
    #[arg(long)]
    repository: String,
    /// Template signature
    #[arg(long)]
    signature: String,
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
    args: TemplatesArgs,
    client: &GtmApiClient,
    format: &OutputFormat,
) -> Result<()> {
    match args.action {
        TemplatesAction::List(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let result = client.get_all(&format!("{base}/templates")).await?;
            print_resource(&result, format, "templates");
        }
        TemplatesAction::Get(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let result = client
                .get(&format!("{base}/templates/{}", a.template_id))
                .await?;
            print_resource(&result, format, "template");
        }
        TemplatesAction::Create(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let mut body = json!({ "name": a.name });
            if let Some(data) = a.template_data {
                let parsed: serde_json::Value = serde_json::from_str(&data)
                    .map_err(|_| crate::error::GtmError::InvalidParams(data))?;
                // Merge template data into body
                if let Some(obj) = parsed.as_object() {
                    for (k, v) in obj {
                        body[k] = v.clone();
                    }
                }
            }
            let result = client.post(&format!("{base}/templates"), &body).await?;
            print_resource(&result, format, "template");
        }
        TemplatesAction::Update(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let path = format!("{base}/templates/{}", a.template_id);
            let mut body = client.get(&path).await?;
            if let Some(name) = a.name {
                body["name"] = json!(name);
            }
            if let Some(data) = a.template_data {
                let parsed: serde_json::Value = serde_json::from_str(&data)
                    .map_err(|_| crate::error::GtmError::InvalidParams(data))?;
                if let Some(obj) = parsed.as_object() {
                    for (k, v) in obj {
                        body[k] = v.clone();
                    }
                }
            }
            let result = client.put(&path, &body).await?;
            print_resource(&result, format, "template");
        }
        TemplatesAction::Delete(a) => {
            if !a.force {
                eprintln!(
                    "WARNING: This will permanently delete template '{}'.",
                    a.template_id
                );
                eprintln!("Run the same command with --force to confirm.");
                return Ok(());
            }
            let base = workspace_path(&a.ws, client).await?;
            client
                .delete(&format!("{base}/templates/{}", a.template_id))
                .await?;
            crate::output::formatter::print_deleted("template", &a.template_id);
        }
        TemplatesAction::Revert(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let result = client
                .post(
                    &format!("{base}/templates/{}:revert", a.template_id),
                    &json!({}),
                )
                .await?;
            print_resource(&result, format, "template");
        }
        TemplatesAction::Import(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let body = json!({
                "galleryReference": {
                    "host": a.host,
                    "owner": a.owner,
                    "repository": a.repository,
                    "signature": a.signature,
                }
            });
            let path = format!("{base}/templates:importFromGallery");
            let result = client.post(&path, &body).await?;
            print_resource(&result, format, "template");
        }
    }
    Ok(())
}
