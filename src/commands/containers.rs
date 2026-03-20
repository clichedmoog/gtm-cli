use clap::{Args, Subcommand};
use serde_json::json;

use crate::api::client::GtmApiClient;
use crate::error::Result;
use crate::output::formatter::{print_resource, OutputFormat};

#[derive(Args)]
pub struct ContainersArgs {
    #[command(subcommand)]
    pub action: ContainersAction,
}

#[derive(Subcommand)]
pub enum ContainersAction {
    /// List containers in an account
    List(ContainerAccountArgs),
    /// Get container details
    Get(ContainerIdArgs),
    /// Create a new container
    Create(ContainerCreateArgs),
    /// Update a container
    Update(ContainerUpdateArgs),
    /// Delete a container
    Delete(ContainerDeleteArgs),
    /// Get container installation snippet
    Snippet(ContainerIdArgs),
    /// Lookup container by public ID (GTM-XXXXX)
    Lookup(ContainerLookupArgs),
    /// Combine (merge) two containers
    Combine(ContainerCombineArgs),
    /// Move a Tag ID to another container
    MoveTagId(ContainerMoveTagIdArgs),
}

#[derive(Args)]
pub struct ContainerAccountArgs {
    #[arg(long, env = "GTM_ACCOUNT_ID")]
    pub account_id: String,
}

#[derive(Args)]
pub struct ContainerIdArgs {
    #[arg(long, env = "GTM_ACCOUNT_ID")]
    pub account_id: String,
    #[arg(long, env = "GTM_CONTAINER_ID")]
    pub container_id: String,
}

#[derive(Args)]
pub struct ContainerCreateArgs {
    #[arg(long, env = "GTM_ACCOUNT_ID")]
    pub account_id: String,
    /// Container name
    #[arg(long)]
    pub name: String,
    /// Usage context: web, android, ios, amp
    #[arg(long, value_delimiter = ',')]
    pub usage_context: Vec<String>,
}

#[derive(Args)]
pub struct ContainerUpdateArgs {
    #[arg(long, env = "GTM_ACCOUNT_ID")]
    pub account_id: String,
    #[arg(long, env = "GTM_CONTAINER_ID")]
    pub container_id: String,
    #[arg(long)]
    pub name: Option<String>,
}

#[derive(Args)]
pub struct ContainerDeleteArgs {
    #[arg(long, env = "GTM_ACCOUNT_ID")]
    pub account_id: String,
    #[arg(long, env = "GTM_CONTAINER_ID")]
    pub container_id: String,
    /// Required to confirm deletion
    #[arg(long)]
    pub force: bool,
}

#[derive(Args)]
pub struct ContainerLookupArgs {
    /// Public container ID (e.g., GTM-XXXXX)
    #[arg(long)]
    pub public_id: String,
}

#[derive(Args)]
pub struct ContainerCombineArgs {
    #[arg(long, env = "GTM_ACCOUNT_ID")]
    pub account_id: String,
    #[arg(long, env = "GTM_CONTAINER_ID")]
    pub container_id: String,
    /// Allow user permission merging to resolve conflicts
    #[arg(long)]
    pub allow_user_permission_feature_update: bool,
}

#[derive(Args)]
pub struct ContainerMoveTagIdArgs {
    #[arg(long, env = "GTM_ACCOUNT_ID")]
    pub account_id: String,
    #[arg(long, env = "GTM_CONTAINER_ID")]
    pub container_id: String,
    /// Tag name for the destination container
    #[arg(long)]
    pub tag_name: Option<String>,
    /// Tag ID to move
    #[arg(long)]
    pub tag_id: String,
    /// Allow user permission merging to resolve conflicts
    #[arg(long)]
    pub allow_user_permission_feature_update: bool,
    /// Copy tag to destination (instead of move)
    #[arg(long)]
    pub copy_tag: bool,
    /// Copy users to destination container
    #[arg(long)]
    pub copy_users: bool,
    /// Copy container settings to destination
    #[arg(long)]
    pub copy_settings: bool,
}

pub async fn handle(
    args: ContainersArgs,
    client: &GtmApiClient,
    format: &OutputFormat,
) -> Result<()> {
    match args.action {
        ContainersAction::List(a) => {
            let path = format!("accounts/{}/containers", a.account_id);
            let result = client.get_all(&path).await?;
            print_resource(&result, format, "containers");
        }
        ContainersAction::Get(a) => {
            let path = format!("accounts/{}/containers/{}", a.account_id, a.container_id);
            let result = client.get(&path).await?;
            print_resource(&result, format, "container");
        }
        ContainersAction::Create(a) => {
            let path = format!("accounts/{}/containers", a.account_id);
            let body = json!({
                "name": a.name,
                "usageContext": a.usage_context,
            });
            let result = client.post(&path, &body).await?;
            print_resource(&result, format, "container");
        }
        ContainersAction::Update(a) => {
            let path = format!("accounts/{}/containers/{}", a.account_id, a.container_id);
            let mut body = client.get(&path).await?;
            if let Some(name) = a.name {
                body["name"] = json!(name);
            }
            let result = client.put(&path, &body).await?;
            print_resource(&result, format, "container");
        }
        ContainersAction::Delete(a) => {
            if !a.force {
                eprintln!(
                    "WARNING: This will permanently delete container '{}'.",
                    a.container_id
                );
                eprintln!("Run the same command with --force to confirm.");
                return Ok(());
            }
            let path = format!("accounts/{}/containers/{}", a.account_id, a.container_id);
            client.delete(&path).await?;
            crate::output::formatter::print_deleted("container", &a.container_id);
        }
        ContainersAction::Snippet(a) => {
            let path = format!(
                "accounts/{}/containers/{}:snippet",
                a.account_id, a.container_id
            );
            let result = client.get(&path).await?;
            print_resource(&result, format, "container");
        }
        ContainersAction::Lookup(a) => {
            let encoded: String =
                url::form_urlencoded::byte_serialize(a.public_id.as_bytes()).collect();
            let path = format!("accounts/containers:lookup?destinationId={encoded}");
            let result = client.get(&path).await?;
            print_resource(&result, format, "container");
        }
        ContainersAction::Combine(a) => {
            let path = format!(
                "accounts/{}/containers/{}:combine",
                a.account_id, a.container_id
            );
            let mut query: Vec<(&str, &str)> = vec![];
            if a.allow_user_permission_feature_update {
                query.push(("allowUserPermissionFeatureUpdate", "true"));
            }
            let result = client.post_with_query(&path, &query, &json!({})).await?;
            print_resource(&result, format, "container");
        }
        ContainersAction::MoveTagId(a) => {
            let path = format!(
                "accounts/{}/containers/{}:move_tag_id",
                a.account_id, a.container_id
            );
            let mut query: Vec<(&str, String)> = vec![];
            query.push(("tagId", a.tag_id.clone()));
            if let Some(ref name) = a.tag_name {
                query.push(("tagName", name.clone()));
            }
            if a.allow_user_permission_feature_update {
                query.push(("allowUserPermissionFeatureUpdate", "true".into()));
            }
            if a.copy_tag {
                query.push(("copyTag", "true".into()));
            }
            if a.copy_users {
                query.push(("copyUsers", "true".into()));
            }
            if a.copy_settings {
                query.push(("copySettings", "true".into()));
            }
            let query_refs: Vec<(&str, &str)> =
                query.iter().map(|(k, v)| (*k, v.as_str())).collect();
            let result = client
                .post_with_query(&path, &query_refs, &json!({}))
                .await?;
            print_resource(&result, format, "container");
        }
    }
    Ok(())
}
