use clap::{Args, Subcommand};
use serde_json::json;

use crate::api::client::GtmApiClient;
use crate::error::Result;
use crate::output::formatter::{print_resource, OutputFormat};

#[derive(Args)]
pub struct WorkspacesArgs {
    #[command(subcommand)]
    pub action: WorkspacesAction,
}

#[derive(Args)]
struct ContainerFlags {
    #[arg(long, env = "GTM_ACCOUNT_ID")]
    account_id: String,
    #[arg(long, env = "GTM_CONTAINER_ID")]
    container_id: String,
}

#[derive(Args)]
struct WorkspaceFlags {
    #[arg(long, env = "GTM_ACCOUNT_ID")]
    account_id: String,
    #[arg(long, env = "GTM_CONTAINER_ID")]
    container_id: String,
    #[arg(long, env = "GTM_WORKSPACE_ID")]
    workspace_id: String,
}

#[derive(Subcommand)]
pub enum WorkspacesAction {
    /// List workspaces
    List(WorkspaceListArgs),
    /// Get workspace details
    Get(WorkspaceGetArgs),
    /// Create a new workspace
    Create(WorkspaceCreateArgs),
    /// Update workspace
    Update(WorkspaceUpdateArgs),
    /// Delete workspace
    Delete(WorkspaceDeleteArgs),
    /// Get workspace status (changed entities)
    Status(WorkspaceStatusArgs),
    /// Sync workspace with latest version
    Sync(WorkspaceSyncArgs),
    /// Create a version from workspace
    CreateVersion(WorkspaceCreateVersionArgs),
    /// Quick preview workspace
    QuickPreview(WorkspaceQuickPreviewArgs),
}

#[derive(Args)]
pub struct WorkspaceListArgs {
    #[command(flatten)]
    c: ContainerFlags,
}

#[derive(Args)]
pub struct WorkspaceGetArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
}

#[derive(Args)]
pub struct WorkspaceCreateArgs {
    #[command(flatten)]
    c: ContainerFlags,
    #[arg(long)]
    name: String,
    #[arg(long)]
    description: Option<String>,
}

#[derive(Args)]
pub struct WorkspaceUpdateArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    description: Option<String>,
}

#[derive(Args)]
pub struct WorkspaceDeleteArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
}

#[derive(Args)]
pub struct WorkspaceStatusArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
}

#[derive(Args)]
pub struct WorkspaceSyncArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
}

#[derive(Args)]
pub struct WorkspaceCreateVersionArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    notes: Option<String>,
}

#[derive(Args)]
pub struct WorkspaceQuickPreviewArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
}

pub async fn handle(args: WorkspacesArgs, client: &GtmApiClient, format: &OutputFormat) -> Result<()> {
    match args.action {
        WorkspacesAction::List(a) => {
            let path = format!("accounts/{}/containers/{}/workspaces", a.c.account_id, a.c.container_id);
            let result = client.get(&path).await?;
            print_resource(&result, format, "workspaces");
        }
        WorkspacesAction::Get(a) => {
            let path = format!(
                "accounts/{}/containers/{}/workspaces/{}",
                a.ws.account_id, a.ws.container_id, a.ws.workspace_id
            );
            let result = client.get(&path).await?;
            print_resource(&result, format, "workspace");
        }
        WorkspacesAction::Create(a) => {
            let path = format!("accounts/{}/containers/{}/workspaces", a.c.account_id, a.c.container_id);
            let mut body = json!({ "name": a.name });
            if let Some(desc) = a.description {
                body["description"] = json!(desc);
            }
            let result = client.post(&path, &body).await?;
            print_resource(&result, format, "workspace");
        }
        WorkspacesAction::Update(a) => {
            let path = format!(
                "accounts/{}/containers/{}/workspaces/{}",
                a.ws.account_id, a.ws.container_id, a.ws.workspace_id
            );
            let mut body = json!({});
            if let Some(name) = a.name {
                body["name"] = json!(name);
            }
            if let Some(desc) = a.description {
                body["description"] = json!(desc);
            }
            let result = client.put(&path, &body).await?;
            print_resource(&result, format, "workspace");
        }
        WorkspacesAction::Delete(a) => {
            let path = format!(
                "accounts/{}/containers/{}/workspaces/{}",
                a.ws.account_id, a.ws.container_id, a.ws.workspace_id
            );
            client.delete(&path).await?;
            crate::output::formatter::print_deleted("workspace", &a.ws.workspace_id);
        }
        WorkspacesAction::Status(a) => {
            let path = format!(
                "accounts/{}/containers/{}/workspaces/{}/status",
                a.ws.account_id, a.ws.container_id, a.ws.workspace_id
            );
            let result = client.get(&path).await?;
            print_resource(&result, format, "workspace");
        }
        WorkspacesAction::Sync(a) => {
            let path = format!(
                "accounts/{}/containers/{}/workspaces/{}:sync",
                a.ws.account_id, a.ws.container_id, a.ws.workspace_id
            );
            let result = client.post(&path, &json!({})).await?;
            print_resource(&result, format, "workspace");
        }
        WorkspacesAction::CreateVersion(a) => {
            let path = format!(
                "accounts/{}/containers/{}/workspaces/{}:create_version",
                a.ws.account_id, a.ws.container_id, a.ws.workspace_id
            );
            let mut body = json!({});
            if let Some(name) = a.name {
                body["name"] = json!(name);
            }
            if let Some(notes) = a.notes {
                body["notes"] = json!(notes);
            }
            let result = client.post(&path, &body).await?;
            print_resource(&result, format, "workspace");
        }
        WorkspacesAction::QuickPreview(a) => {
            let path = format!(
                "accounts/{}/containers/{}/workspaces/{}:quick_preview",
                a.ws.account_id, a.ws.container_id, a.ws.workspace_id
            );
            let result = client.post(&path, &json!({})).await?;
            print_resource(&result, format, "workspace");
        }
    }
    Ok(())
}
