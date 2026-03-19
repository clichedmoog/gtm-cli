use clap::{Args, Subcommand};
use serde_json::json;

use crate::api::client::GtmApiClient;
use crate::error::Result;
use crate::output::formatter::{print_resource, OutputFormat};

#[derive(Args)]
pub struct DestinationsArgs {
    #[command(subcommand)]
    pub action: DestinationsAction,
}

#[derive(Args)]
struct ContainerFlags {
    #[arg(long, env = "GTM_ACCOUNT_ID")]
    account_id: String,
    #[arg(long, env = "GTM_CONTAINER_ID")]
    container_id: String,
}

#[derive(Subcommand)]
pub enum DestinationsAction {
    /// List destinations for a container
    List(DestListArgs),
    /// Get destination details
    Get(DestGetArgs),
    /// Link a destination to a container
    Link(DestLinkArgs),
}

#[derive(Args)]
pub struct DestListArgs {
    #[command(flatten)]
    c: ContainerFlags,
}

#[derive(Args)]
pub struct DestGetArgs {
    #[command(flatten)]
    c: ContainerFlags,
    /// Destination ID (e.g., AW-123456789)
    #[arg(long)]
    destination_id: String,
}

#[derive(Args)]
pub struct DestLinkArgs {
    #[command(flatten)]
    c: ContainerFlags,
    /// Destination ID to link (e.g., AW-123456789)
    #[arg(long)]
    destination_id: String,
}

pub async fn handle(
    args: DestinationsArgs,
    client: &GtmApiClient,
    format: &OutputFormat,
) -> Result<()> {
    match args.action {
        DestinationsAction::List(a) => {
            let path = format!(
                "accounts/{}/containers/{}/destinations",
                a.c.account_id, a.c.container_id
            );
            let result = client.get(&path).await?;
            print_resource(&result, format, "destinations");
        }
        DestinationsAction::Get(a) => {
            let path = format!(
                "accounts/{}/containers/{}/destinations/{}",
                a.c.account_id, a.c.container_id, a.destination_id
            );
            let result = client.get(&path).await?;
            print_resource(&result, format, "destination");
        }
        DestinationsAction::Link(a) => {
            let path = format!(
                "accounts/{}/containers/{}/destinations:link",
                a.c.account_id, a.c.container_id
            );
            let query = [("destinationId", a.destination_id.as_str())];
            let result = client.post_with_query(&path, &query, &json!({})).await?;
            print_resource(&result, format, "destination");
        }
    }
    Ok(())
}
