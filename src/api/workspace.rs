use serde_json::json;

use super::client::GtmApiClient;
use crate::error::Result;

/// Resolve workspace ID. If provided, use it directly.
/// Otherwise, list workspaces and return the first one.
/// If no workspaces exist, create a "Default Workspace".
pub async fn resolve_workspace(
    client: &GtmApiClient,
    account_id: &str,
    container_id: &str,
    workspace_id: Option<&str>,
) -> Result<String> {
    if let Some(id) = workspace_id {
        return Ok(id.to_string());
    }

    let path = format!("accounts/{account_id}/containers/{container_id}/workspaces");
    let result = client.get(&path).await?;

    if let Some(workspaces) = result.get("workspace").and_then(|w| w.as_array()) {
        if let Some(first) = workspaces.first() {
            if let Some(id) = first.get("workspaceId").and_then(|v| v.as_str()) {
                return Ok(id.to_string());
            }
        }
    }

    // No workspaces found, create one
    let body = json!({
        "name": "Default Workspace",
        "description": "Created by gtm CLI"
    });
    let created = client.post(&path, &body).await?;
    let id = created
        .get("workspaceId")
        .and_then(|v| v.as_str())
        .unwrap_or("1")
        .to_string();

    Ok(id)
}
