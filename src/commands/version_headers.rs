use clap::{Args, Subcommand};

use crate::api::client::GtmApiClient;
use crate::error::Result;
use crate::output::formatter::{print_resource, OutputFormat};

#[derive(Args)]
pub struct VersionHeadersArgs {
    #[command(subcommand)]
    pub action: VersionHeadersAction,
}

#[derive(Args)]
struct ContainerFlags {
    #[arg(long, env = "GTM_ACCOUNT_ID")]
    account_id: String,
    #[arg(long, env = "GTM_CONTAINER_ID")]
    container_id: String,
}

#[derive(Subcommand)]
pub enum VersionHeadersAction {
    /// List version headers
    List(VersionHeadersListArgs),
    /// Get the latest version header
    Latest(VersionHeadersLatestArgs),
}

#[derive(Args)]
pub struct VersionHeadersListArgs {
    #[command(flatten)]
    c: ContainerFlags,
}

#[derive(Args)]
pub struct VersionHeadersLatestArgs {
    #[command(flatten)]
    c: ContainerFlags,
}

pub async fn handle(
    args: VersionHeadersArgs,
    client: &GtmApiClient,
    format: &OutputFormat,
) -> Result<()> {
    match args.action {
        VersionHeadersAction::List(a) => {
            let path = format!(
                "accounts/{}/containers/{}/version_headers",
                a.c.account_id, a.c.container_id
            );
            let result = client.get_all(&path).await?;
            print_resource(&result, format, "version_headers");
        }
        VersionHeadersAction::Latest(a) => {
            let path = format!(
                "accounts/{}/containers/{}/version_headers:latest",
                a.c.account_id, a.c.container_id
            );
            let result = client.get(&path).await?;
            print_resource(&result, format, "version_headers");
        }
    }
    Ok(())
}
