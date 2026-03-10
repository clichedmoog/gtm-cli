use std::collections::HashMap;

use clap::{Args, Subcommand};
use serde_json::{json, Value};

use crate::api::client::GtmApiClient;
use crate::api::workspace::resolve_workspace;
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
    /// Export workspace (tags, triggers, variables, folders) to JSON
    Export(WorkspaceExportArgs),
    /// Import entities from a JSON export file
    Import(WorkspaceImportArgs),
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
    /// Required to confirm deletion
    #[arg(long)]
    force: bool,
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

#[derive(Args)]
pub struct WorkspaceExportArgs {
    #[arg(long, env = "GTM_ACCOUNT_ID")]
    account_id: String,
    #[arg(long, env = "GTM_CONTAINER_ID")]
    container_id: String,
    #[arg(long, env = "GTM_WORKSPACE_ID")]
    workspace_id: Option<String>,
    /// Output file (default: stdout)
    #[arg(long, short)]
    output: Option<String>,
}

#[derive(Args)]
pub struct WorkspaceImportArgs {
    #[arg(long, env = "GTM_ACCOUNT_ID")]
    account_id: String,
    #[arg(long, env = "GTM_CONTAINER_ID")]
    container_id: String,
    #[arg(long, env = "GTM_WORKSPACE_ID")]
    workspace_id: Option<String>,
    /// Input file with exported workspace JSON
    #[arg(long, short)]
    input: String,
}

pub async fn handle(
    args: WorkspacesArgs,
    client: &GtmApiClient,
    format: &OutputFormat,
) -> Result<()> {
    match args.action {
        WorkspacesAction::List(a) => {
            let path = format!(
                "accounts/{}/containers/{}/workspaces",
                a.c.account_id, a.c.container_id
            );
            let result = client.get_all(&path).await?;
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
            let path = format!(
                "accounts/{}/containers/{}/workspaces",
                a.c.account_id, a.c.container_id
            );
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
            let mut body = client.get(&path).await?;
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
            if !a.force {
                eprintln!(
                    "WARNING: This will permanently delete workspace '{}'.",
                    a.ws.workspace_id
                );
                eprintln!("Run the same command with --force to confirm.");
                return Ok(());
            }
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
        WorkspacesAction::Export(a) => {
            let ws_id = resolve_workspace(
                client,
                &a.account_id,
                &a.container_id,
                a.workspace_id.as_deref(),
            )
            .await?;
            let base = format!(
                "accounts/{}/containers/{}/workspaces/{}",
                a.account_id, a.container_id, ws_id
            );

            let (tags, triggers, variables, folders) = tokio::join!(
                async {
                    client
                        .get(&format!("{base}/tags"))
                        .await
                        .unwrap_or(json!({}))
                },
                async {
                    client
                        .get(&format!("{base}/triggers"))
                        .await
                        .unwrap_or(json!({}))
                },
                async {
                    client
                        .get(&format!("{base}/variables"))
                        .await
                        .unwrap_or(json!({}))
                },
                async {
                    client
                        .get(&format!("{base}/folders"))
                        .await
                        .unwrap_or(json!({}))
                },
            );

            let export = json!({
                "exportVersion": "1",
                "accountId": a.account_id,
                "containerId": a.container_id,
                "workspaceId": ws_id,
                "tags": tags.get("tag").unwrap_or(&json!([])),
                "triggers": triggers.get("trigger").unwrap_or(&json!([])),
                "variables": variables.get("variable").unwrap_or(&json!([])),
                "folders": folders.get("folder").unwrap_or(&json!([])),
            });

            let output = serde_json::to_string_pretty(&export).unwrap();
            if let Some(path) = a.output {
                std::fs::write(&path, &output).map_err(crate::error::GtmError::Io)?;
                eprintln!("Exported to {path}");
            } else {
                println!("{output}");
            }
        }
        WorkspacesAction::Import(a) => {
            let ws_id = resolve_workspace(
                client,
                &a.account_id,
                &a.container_id,
                a.workspace_id.as_deref(),
            )
            .await?;
            let base = format!(
                "accounts/{}/containers/{}/workspaces/{}",
                a.account_id, a.container_id, ws_id
            );

            let data = std::fs::read_to_string(&a.input).map_err(crate::error::GtmError::Io)?;
            let export: Value = serde_json::from_str(&data)
                .map_err(|_| crate::error::GtmError::InvalidParams(a.input.clone()))?;

            // Create folders first
            let mut folder_id_map: HashMap<String, String> = HashMap::new();
            if let Some(folders) = export["folders"].as_array() {
                for folder in folders {
                    let body = json!({
                        "name": folder["name"],
                        "notes": folder.get("notes").unwrap_or(&json!(null)),
                    });
                    let result = client.post(&format!("{base}/folders"), &body).await?;
                    if let (Some(old_id), Some(new_id)) =
                        (folder["folderId"].as_str(), result["folderId"].as_str())
                    {
                        folder_id_map.insert(old_id.to_string(), new_id.to_string());
                    }
                    eprintln!("Created folder: {}", folder["name"].as_str().unwrap_or("?"));
                }
            }

            // Create triggers
            let mut trigger_id_map: HashMap<String, String> = HashMap::new();
            if let Some(triggers) = export["triggers"].as_array() {
                for trigger in triggers {
                    let mut body = json!({
                        "name": trigger["name"],
                        "type": trigger["type"],
                    });
                    // Copy relevant fields
                    for key in [
                        "customEventFilter",
                        "filter",
                        "autoEventFilter",
                        "waitForTags",
                        "checkValidation",
                        "waitForTagsTimeout",
                        "uniqueTriggerId",
                        "parameter",
                    ] {
                        if trigger.get(key).is_some() {
                            body[key] = trigger[key].clone();
                        }
                    }
                    let result = client.post(&format!("{base}/triggers"), &body).await?;
                    if let (Some(old_id), Some(new_id)) =
                        (trigger["triggerId"].as_str(), result["triggerId"].as_str())
                    {
                        trigger_id_map.insert(old_id.to_string(), new_id.to_string());
                    }
                    eprintln!(
                        "Created trigger: {}",
                        trigger["name"].as_str().unwrap_or("?")
                    );
                }
            }

            // Helper: remap parentFolderId using folder_id_map
            let remap_folder = |entity: &Value, body: &mut Value, map: &HashMap<String, String>| {
                if let Some(old_folder) = entity["parentFolderId"].as_str() {
                    if let Some(new_folder) = map.get(old_folder) {
                        body["parentFolderId"] = json!(new_folder);
                    }
                }
            };

            // Create variables
            if let Some(variables) = export["variables"].as_array() {
                for variable in variables {
                    let mut body = json!({
                        "name": variable["name"],
                        "type": variable["type"],
                    });
                    if variable.get("parameter").is_some() {
                        body["parameter"] = variable["parameter"].clone();
                    }
                    remap_folder(variable, &mut body, &folder_id_map);
                    client.post(&format!("{base}/variables"), &body).await?;
                    eprintln!(
                        "Created variable: {}",
                        variable["name"].as_str().unwrap_or("?")
                    );
                }
            }

            // Create tags (with remapped trigger IDs)
            if let Some(tags) = export["tags"].as_array() {
                for tag in tags {
                    let mut body = json!({
                        "name": tag["name"],
                        "type": tag["type"],
                    });
                    if tag.get("parameter").is_some() {
                        body["parameter"] = tag["parameter"].clone();
                    }
                    // Remap trigger IDs, preserving unmapped ones (e.g. built-in triggers)
                    for field in ["firingTriggerId", "blockingTriggerId"] {
                        if let Some(ids) = tag[field].as_array() {
                            let new_ids: Vec<&str> = ids
                                .iter()
                                .filter_map(|id| id.as_str())
                                .map(|old| {
                                    trigger_id_map.get(old).map(|s| s.as_str()).unwrap_or(old)
                                })
                                .collect();
                            if !new_ids.is_empty() {
                                body[field] = json!(new_ids);
                            }
                        }
                    }
                    remap_folder(tag, &mut body, &folder_id_map);
                    client.post(&format!("{base}/tags"), &body).await?;
                    eprintln!("Created tag: {}", tag["name"].as_str().unwrap_or("?"));
                }
            }

            eprintln!("Import complete.");
        }
    }
    Ok(())
}
