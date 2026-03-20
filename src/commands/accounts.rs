use clap::{Args, Subcommand};
use serde_json::json;

use crate::api::client::GtmApiClient;
use crate::error::Result;
use crate::output::formatter::{print_resource, OutputFormat};

#[derive(Args)]
pub struct AccountsArgs {
    #[command(subcommand)]
    pub action: AccountsAction,
}

#[derive(Subcommand)]
pub enum AccountsAction {
    /// List all GTM accounts
    List,
    /// Get account details
    Get(AccountGetArgs),
    /// Update account settings
    Update(AccountUpdateArgs),
}

#[derive(Args)]
pub struct AccountGetArgs {
    /// GTM account ID
    #[arg(long, env = "GTM_ACCOUNT_ID")]
    pub account_id: String,
}

#[derive(Args)]
pub struct AccountUpdateArgs {
    /// GTM account ID
    #[arg(long, env = "GTM_ACCOUNT_ID")]
    pub account_id: String,
    /// Account name
    #[arg(long)]
    pub name: Option<String>,
    /// Share data with Google
    #[arg(long)]
    pub share_data: Option<bool>,
}

pub async fn handle(
    args: AccountsArgs,
    client: &GtmApiClient,
    format: &OutputFormat,
) -> Result<()> {
    match args.action {
        AccountsAction::List => {
            let result = client.get_all("accounts").await?;
            print_resource(&result, format, "accounts");
        }
        AccountsAction::Get(a) => {
            let path = format!("accounts/{}", a.account_id);
            let result = client.get(&path).await?;
            print_resource(&result, format, "account");
        }
        AccountsAction::Update(a) => {
            let path = format!("accounts/{}", a.account_id);
            let mut body = client.get(&path).await?;
            if let Some(name) = a.name {
                body["name"] = json!(name);
            }
            if let Some(share) = a.share_data {
                body["shareData"] = json!(share);
            }
            let result = client.put(&path, &body).await?;
            print_resource(&result, format, "account");
        }
    }
    Ok(())
}
