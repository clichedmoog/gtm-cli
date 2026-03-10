use clap::{Args, Subcommand};
use serde_json::json;

use crate::api::client::GtmApiClient;
use crate::error::{GtmError, Result};
use crate::output::formatter::{print_resource, OutputFormat};

#[derive(Args)]
pub struct PermissionsArgs {
    #[command(subcommand)]
    pub action: PermissionsAction,
}

#[derive(Subcommand)]
pub enum PermissionsAction {
    /// List user permissions
    List(PermissionsListArgs),
    /// Get permission details
    Get(PermissionsGetArgs),
    /// Create a user permission
    Create(PermissionsCreateArgs),
    /// Update a user permission
    Update(PermissionsUpdateArgs),
    /// Delete a user permission
    Delete(PermissionsDeleteArgs),
}

#[derive(Args)]
pub struct PermissionsListArgs {
    #[arg(long, env = "GTM_ACCOUNT_ID")]
    account_id: String,
}

#[derive(Args)]
pub struct PermissionsGetArgs {
    #[arg(long, env = "GTM_ACCOUNT_ID")]
    account_id: String,
    #[arg(long)]
    permission_id: String,
}

#[derive(Args)]
pub struct PermissionsCreateArgs {
    #[arg(long, env = "GTM_ACCOUNT_ID")]
    account_id: String,
    /// User email address
    #[arg(long)]
    email: String,
    /// Account-level access: noAccess, user, admin
    #[arg(long, default_value = "user")]
    account_access: String,
    /// Container permissions as JSON array
    /// e.g., '[{"containerId":"123","permission":"publish"}]'
    #[arg(long)]
    container_access: Option<String>,
}

#[derive(Args)]
pub struct PermissionsUpdateArgs {
    #[arg(long, env = "GTM_ACCOUNT_ID")]
    account_id: String,
    #[arg(long)]
    permission_id: String,
    #[arg(long)]
    account_access: Option<String>,
    #[arg(long)]
    container_access: Option<String>,
}

#[derive(Args)]
pub struct PermissionsDeleteArgs {
    #[arg(long, env = "GTM_ACCOUNT_ID")]
    account_id: String,
    #[arg(long)]
    permission_id: String,
    /// Required to confirm deletion
    #[arg(long)]
    force: bool,
}

pub async fn handle(
    args: PermissionsArgs,
    client: &GtmApiClient,
    format: &OutputFormat,
) -> Result<()> {
    match args.action {
        PermissionsAction::List(a) => {
            let path = format!("accounts/{}/user_permissions", a.account_id);
            let result = client.get_all(&path).await?;
            print_resource(&result, format, "permissions");
        }
        PermissionsAction::Get(a) => {
            let path = format!(
                "accounts/{}/user_permissions/{}",
                a.account_id, a.permission_id
            );
            let result = client.get(&path).await?;
            print_resource(&result, format, "permission");
        }
        PermissionsAction::Create(a) => {
            let path = format!("accounts/{}/user_permissions", a.account_id);
            let mut body = json!({
                "emailAddress": a.email,
                "accountAccess": { "permission": a.account_access },
            });
            if let Some(ca) = a.container_access {
                let parsed: serde_json::Value =
                    serde_json::from_str(&ca).map_err(|_| GtmError::InvalidParams(ca))?;
                body["containerAccess"] = parsed;
            }
            let result = client.post(&path, &body).await?;
            print_resource(&result, format, "permission");
        }
        PermissionsAction::Update(a) => {
            let path = format!(
                "accounts/{}/user_permissions/{}",
                a.account_id, a.permission_id
            );
            let mut body = client.get(&path).await?;
            if let Some(aa) = a.account_access {
                body["accountAccess"] = json!({ "permission": aa });
            }
            if let Some(ca) = a.container_access {
                let parsed: serde_json::Value =
                    serde_json::from_str(&ca).map_err(|_| GtmError::InvalidParams(ca))?;
                body["containerAccess"] = parsed;
            }
            let result = client.put(&path, &body).await?;
            print_resource(&result, format, "permission");
        }
        PermissionsAction::Delete(a) => {
            if !a.force {
                eprintln!(
                    "WARNING: This will permanently delete permission '{}'.",
                    a.permission_id
                );
                eprintln!("Run the same command with --force to confirm.");
                return Ok(());
            }
            let path = format!(
                "accounts/{}/user_permissions/{}",
                a.account_id, a.permission_id
            );
            client.delete(&path).await?;
            crate::output::formatter::print_deleted("permission", &a.permission_id);
        }
    }
    Ok(())
}
