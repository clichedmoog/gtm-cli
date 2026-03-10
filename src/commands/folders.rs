use clap::{Args, Subcommand};
use serde_json::json;

use crate::api::client::GtmApiClient;
use crate::api::workspace::resolve_workspace;
use crate::error::Result;
use crate::output::formatter::{print_resource, OutputFormat};

#[derive(Args)]
pub struct FoldersArgs {
    #[command(subcommand)]
    pub action: FoldersAction,
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
pub enum FoldersAction {
    /// List all folders
    List(FoldersListArgs),
    /// Get folder details
    Get(FoldersGetArgs),
    /// Create a new folder
    Create(FoldersCreateArgs),
    /// Update a folder
    Update(FoldersUpdateArgs),
    /// Delete a folder
    Delete(FoldersDeleteArgs),
    /// Revert folder changes
    Revert(FoldersRevertArgs),
    /// Move tags, triggers, or variables into a folder
    MoveEntities(FoldersMoveArgs),
    /// Get all entities in a folder
    Entities(FoldersEntitiesArgs),
}

#[derive(Args)]
pub struct FoldersListArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
}

#[derive(Args)]
pub struct FoldersGetArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    folder_id: String,
}

#[derive(Args)]
pub struct FoldersCreateArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    name: String,
    #[arg(long)]
    notes: Option<String>,
}

#[derive(Args)]
pub struct FoldersUpdateArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    folder_id: String,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    notes: Option<String>,
}

#[derive(Args)]
pub struct FoldersDeleteArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    folder_id: String,
    /// Required to confirm deletion
    #[arg(long)]
    force: bool,
}

#[derive(Args)]
pub struct FoldersRevertArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    folder_id: String,
}

#[derive(Args)]
pub struct FoldersMoveArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    /// Folder ID to move entities into
    #[arg(long)]
    folder_id: String,
    /// Tag IDs to move (comma-separated)
    #[arg(long, value_delimiter = ',')]
    tag_id: Vec<String>,
    /// Trigger IDs to move (comma-separated)
    #[arg(long, value_delimiter = ',')]
    trigger_id: Vec<String>,
    /// Variable IDs to move (comma-separated)
    #[arg(long, value_delimiter = ',')]
    variable_id: Vec<String>,
}

#[derive(Args)]
pub struct FoldersEntitiesArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    #[arg(long)]
    folder_id: String,
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

pub async fn handle(args: FoldersArgs, client: &GtmApiClient, format: &OutputFormat) -> Result<()> {
    match args.action {
        FoldersAction::List(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let result = client.get_all(&format!("{base}/folders")).await?;
            print_resource(&result, format, "folders");
        }
        FoldersAction::Get(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let result = client
                .get(&format!("{base}/folders/{}", a.folder_id))
                .await?;
            print_resource(&result, format, "folder");
        }
        FoldersAction::Create(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let mut body = json!({ "name": a.name });
            if let Some(notes) = a.notes {
                body["notes"] = json!(notes);
            }
            let result = client.post(&format!("{base}/folders"), &body).await?;
            print_resource(&result, format, "folder");
        }
        FoldersAction::Update(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let path = format!("{base}/folders/{}", a.folder_id);
            let mut body = client.get(&path).await?;
            if let Some(name) = a.name {
                body["name"] = json!(name);
            }
            if let Some(notes) = a.notes {
                body["notes"] = json!(notes);
            }
            let result = client.put(&path, &body).await?;
            print_resource(&result, format, "folder");
        }
        FoldersAction::Delete(a) => {
            if !a.force {
                eprintln!(
                    "WARNING: This will permanently delete folder '{}'.",
                    a.folder_id
                );
                eprintln!("Run the same command with --force to confirm.");
                return Ok(());
            }
            let base = workspace_path(&a.ws, client).await?;
            client
                .delete(&format!("{base}/folders/{}", a.folder_id))
                .await?;
            crate::output::formatter::print_deleted("folder", &a.folder_id);
        }
        FoldersAction::Revert(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let result = client
                .post(
                    &format!("{base}/folders/{}:revert", a.folder_id),
                    &json!({}),
                )
                .await?;
            print_resource(&result, format, "folder");
        }
        FoldersAction::MoveEntities(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let path = format!("{base}/folders/{}:move_entities_to_folder", a.folder_id);

            // tagId, triggerId, variableId are query parameters, not body
            let mut query: Vec<(&str, &str)> = Vec::new();
            for id in &a.tag_id {
                query.push(("tagId", id.as_str()));
            }
            for id in &a.trigger_id {
                query.push(("triggerId", id.as_str()));
            }
            for id in &a.variable_id {
                query.push(("variableId", id.as_str()));
            }

            // Request body is a Folder resource (can be empty)
            let body = json!({});
            let result = client.post_with_query(&path, &query, &body).await?;
            print_resource(&result, format, "folder");
        }
        FoldersAction::Entities(a) => {
            let base = workspace_path(&a.ws, client).await?;
            let result = client
                .post(
                    &format!("{base}/folders/{}:entities", a.folder_id),
                    &json!({}),
                )
                .await?;
            print_resource(&result, format, "folder_entities");
        }
    }
    Ok(())
}
