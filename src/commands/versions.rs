use clap::{Args, Subcommand};
use serde_json::json;

use crate::api::client::GtmApiClient;
use crate::error::Result;
use crate::output::formatter::{print_resource, OutputFormat};

#[derive(Args)]
pub struct VersionsArgs {
    #[command(subcommand)]
    pub action: VersionsAction,
}

#[derive(Args)]
struct ContainerFlags {
    #[arg(long, env = "GTM_ACCOUNT_ID")]
    account_id: String,
    #[arg(long, env = "GTM_CONTAINER_ID")]
    container_id: String,
}

#[derive(Args)]
struct VersionFlags {
    #[arg(long, env = "GTM_ACCOUNT_ID")]
    account_id: String,
    #[arg(long, env = "GTM_CONTAINER_ID")]
    container_id: String,
    #[arg(long)]
    version_id: String,
}

#[derive(Subcommand)]
pub enum VersionsAction {
    /// List container versions
    List(VersionsListArgs),
    /// Get version details
    Get(VersionsGetArgs),
    /// Update version name/notes
    Update(VersionsUpdateArgs),
    /// Delete a version
    Delete(VersionsDeleteArgs),
    /// Restore a deleted version
    Undelete(VersionsUndeleteArgs),
    /// Set a version as the latest
    SetLatest(VersionsSetLatestArgs),
    /// Get the live (published) version
    Live(VersionsLiveArgs),
    /// Publish a version
    Publish(VersionsPublishArgs),
}

#[derive(Args)]
pub struct VersionsListArgs {
    #[command(flatten)]
    c: ContainerFlags,
}

#[derive(Args)]
pub struct VersionsGetArgs {
    #[command(flatten)]
    v: VersionFlags,
}

#[derive(Args)]
pub struct VersionsUpdateArgs {
    #[command(flatten)]
    v: VersionFlags,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    notes: Option<String>,
}

#[derive(Args)]
pub struct VersionsDeleteArgs {
    #[command(flatten)]
    v: VersionFlags,
    /// Required to confirm deletion
    #[arg(long)]
    force: bool,
}

#[derive(Args)]
pub struct VersionsUndeleteArgs {
    #[command(flatten)]
    v: VersionFlags,
}

#[derive(Args)]
pub struct VersionsSetLatestArgs {
    #[command(flatten)]
    v: VersionFlags,
}

#[derive(Args)]
pub struct VersionsLiveArgs {
    #[command(flatten)]
    c: ContainerFlags,
}

#[derive(Args)]
pub struct VersionsPublishArgs {
    #[command(flatten)]
    v: VersionFlags,
}

fn container_path(c: &ContainerFlags) -> String {
    format!("accounts/{}/containers/{}", c.account_id, c.container_id)
}

fn version_path(v: &VersionFlags) -> String {
    format!(
        "accounts/{}/containers/{}/versions/{}",
        v.account_id, v.container_id, v.version_id
    )
}

pub async fn handle(
    args: VersionsArgs,
    client: &GtmApiClient,
    format: &OutputFormat,
) -> Result<()> {
    match args.action {
        VersionsAction::List(a) => {
            let path = format!("{}/versions", container_path(&a.c));
            let result = client.get_all(&path).await?;
            print_resource(&result, format, "versions");
        }
        VersionsAction::Get(a) => {
            let result = client.get(&version_path(&a.v)).await?;
            print_resource(&result, format, "version");
        }
        VersionsAction::Update(a) => {
            let path = version_path(&a.v);
            let mut body = client.get(&path).await?;
            if let Some(name) = a.name {
                body["name"] = json!(name);
            }
            if let Some(notes) = a.notes {
                body["notes"] = json!(notes);
            }
            let result = client.put(&path, &body).await?;
            print_resource(&result, format, "version");
        }
        VersionsAction::Delete(a) => {
            if !a.force {
                eprintln!(
                    "WARNING: This will permanently delete version '{}'.",
                    a.v.version_id
                );
                eprintln!("Run the same command with --force to confirm.");
                return Ok(());
            }
            client.delete(&version_path(&a.v)).await?;
            crate::output::formatter::print_deleted("version", &a.v.version_id);
        }
        VersionsAction::Undelete(a) => {
            let path = format!("{}:undelete", version_path(&a.v));
            let result = client.post(&path, &json!({})).await?;
            print_resource(&result, format, "version");
        }
        VersionsAction::SetLatest(a) => {
            let path = format!("{}:set_latest", version_path(&a.v));
            let result = client.post(&path, &json!({})).await?;
            print_resource(&result, format, "version");
        }
        VersionsAction::Live(a) => {
            let path = format!("{}/versions:live", container_path(&a.c));
            let result = client.get(&path).await?;
            print_resource(&result, format, "version");
        }
        VersionsAction::Publish(a) => {
            let path = format!("{}:publish", version_path(&a.v));
            let result = client.post(&path, &json!({})).await?;
            print_resource(&result, format, "version");
        }
    }
    Ok(())
}
