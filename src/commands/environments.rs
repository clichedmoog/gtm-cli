use clap::{Args, Subcommand};
use serde_json::json;

use crate::api::client::GtmApiClient;
use crate::error::Result;
use crate::output::formatter::{print_resource, OutputFormat};

#[derive(Args)]
pub struct EnvironmentsArgs {
    #[command(subcommand)]
    pub action: EnvironmentsAction,
}

#[derive(Args)]
struct ContainerFlags {
    #[arg(long, env = "GTM_ACCOUNT_ID")]
    account_id: String,
    #[arg(long, env = "GTM_CONTAINER_ID")]
    container_id: String,
}

#[derive(Args)]
struct EnvFlags {
    #[arg(long, env = "GTM_ACCOUNT_ID")]
    account_id: String,
    #[arg(long, env = "GTM_CONTAINER_ID")]
    container_id: String,
    #[arg(long)]
    environment_id: String,
}

#[derive(Subcommand)]
pub enum EnvironmentsAction {
    /// List environments
    List(EnvListArgs),
    /// Get environment details
    Get(EnvGetArgs),
    /// Create an environment
    Create(EnvCreateArgs),
    /// Update an environment
    Update(EnvUpdateArgs),
    /// Delete an environment
    Delete(EnvDeleteArgs),
    /// Reauthorize an environment
    Reauthorize(EnvReauthorizeArgs),
}

#[derive(Args)]
pub struct EnvListArgs {
    #[command(flatten)]
    c: ContainerFlags,
}

#[derive(Args)]
pub struct EnvGetArgs {
    #[command(flatten)]
    e: EnvFlags,
}

#[derive(Args)]
pub struct EnvCreateArgs {
    #[command(flatten)]
    c: ContainerFlags,
    #[arg(long)]
    name: String,
    #[arg(long)]
    description: Option<String>,
    /// Enable debug mode
    #[arg(long)]
    enable_debug: Option<bool>,
}

#[derive(Args)]
pub struct EnvUpdateArgs {
    #[command(flatten)]
    e: EnvFlags,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    description: Option<String>,
    #[arg(long)]
    enable_debug: Option<bool>,
}

#[derive(Args)]
pub struct EnvDeleteArgs {
    #[command(flatten)]
    e: EnvFlags,
    /// Required to confirm deletion
    #[arg(long)]
    force: bool,
}

#[derive(Args)]
pub struct EnvReauthorizeArgs {
    #[command(flatten)]
    e: EnvFlags,
}

fn env_path(e: &EnvFlags) -> String {
    format!(
        "accounts/{}/containers/{}/environments/{}",
        e.account_id, e.container_id, e.environment_id
    )
}

pub async fn handle(
    args: EnvironmentsArgs,
    client: &GtmApiClient,
    format: &OutputFormat,
) -> Result<()> {
    match args.action {
        EnvironmentsAction::List(a) => {
            let path = format!(
                "accounts/{}/containers/{}/environments",
                a.c.account_id, a.c.container_id
            );
            let result = client.get_all(&path).await?;
            print_resource(&result, format, "environments");
        }
        EnvironmentsAction::Get(a) => {
            let result = client.get(&env_path(&a.e)).await?;
            print_resource(&result, format, "environment");
        }
        EnvironmentsAction::Create(a) => {
            let path = format!(
                "accounts/{}/containers/{}/environments",
                a.c.account_id, a.c.container_id
            );
            let mut body = json!({ "name": a.name });
            if let Some(desc) = a.description {
                body["description"] = json!(desc);
            }
            if let Some(debug) = a.enable_debug {
                body["enableDebug"] = json!(debug);
            }
            let result = client.post(&path, &body).await?;
            print_resource(&result, format, "environment");
        }
        EnvironmentsAction::Update(a) => {
            let path = env_path(&a.e);
            let mut body = client.get(&path).await?;
            if let Some(name) = a.name {
                body["name"] = json!(name);
            }
            if let Some(desc) = a.description {
                body["description"] = json!(desc);
            }
            if let Some(debug) = a.enable_debug {
                body["enableDebug"] = json!(debug);
            }
            let result = client.put(&path, &body).await?;
            print_resource(&result, format, "environment");
        }
        EnvironmentsAction::Delete(a) => {
            if !a.force {
                eprintln!(
                    "WARNING: This will permanently delete environment '{}'.",
                    a.e.environment_id
                );
                eprintln!("Run the same command with --force to confirm.");
                return Ok(());
            }
            client.delete(&env_path(&a.e)).await?;
            crate::output::formatter::print_deleted("environment", &a.e.environment_id);
        }
        EnvironmentsAction::Reauthorize(a) => {
            let path = format!("{}:reauthorize", env_path(&a.e));
            let result = client.post(&path, &json!({})).await?;
            print_resource(&result, format, "environment");
        }
    }
    Ok(())
}
