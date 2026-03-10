mod api;
mod auth;
mod commands;
mod config;
mod error;
mod output;

use clap::{Parser, Subcommand};

use api::client::GtmApiClient;
use config::Config;
use output::formatter::OutputFormat;

#[derive(Parser)]
#[command(name = "gtm", version, about = "Google Tag Manager CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output format
    #[arg(long, global = true, default_value = "json")]
    format: OutputFormat,
}

#[derive(Subcommand)]
enum Commands {
    /// Authenticate with Google
    Auth(commands::auth::AuthArgs),
    /// Manage GTM accounts
    Accounts(commands::accounts::AccountsArgs),
    /// Manage GTM containers
    Containers(commands::containers::ContainersArgs),
    /// Manage GTM workspaces
    Workspaces(commands::workspaces::WorkspacesArgs),
    /// Manage GTM tags
    Tags(commands::tags::TagsArgs),
    /// Manage GTM triggers
    Triggers(commands::triggers::TriggersArgs),
    /// Manage GTM variables
    Variables(commands::variables::VariablesArgs),
    /// Manage GTM folders
    Folders(commands::folders::FoldersArgs),
    /// Manage GTM templates
    Templates(commands::templates::TemplatesArgs),
    /// Manage container versions
    Versions(commands::versions::VersionsArgs),
    /// Manage version headers
    VersionHeaders(commands::version_headers::VersionHeadersArgs),
    /// Manage environments
    Environments(commands::environments::EnvironmentsArgs),
    /// Manage user permissions
    Permissions(commands::permissions::PermissionsArgs),
    /// Manage clients (server-side)
    Clients(commands::clients::ClientsArgs),
    /// Manage Google Tag configs
    GtagConfigs(commands::gtag_configs::GtagConfigsArgs),
    /// Manage transformations (server-side)
    Transformations(commands::transformations::TransformationsArgs),
    /// Manage zones (server-side)
    Zones(commands::zones::ZonesArgs),
    /// Manage built-in variables
    BuiltinVariables(commands::builtin_variables::BuiltinVariablesArgs),
    /// Quick setup workflows (GA4, Facebook Pixel, etc.)
    Setup(commands::setup::SetupArgs),
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let config = Config::load();

    let result = match cli.command {
        Commands::Auth(args) => commands::auth::handle(args, &config).await,
        _ => {
            let client = GtmApiClient::new(config);
            match cli.command {
                Commands::Auth(_) => unreachable!(),
                Commands::Accounts(args) => {
                    commands::accounts::handle(args, &client, &cli.format).await
                }
                Commands::Containers(args) => {
                    commands::containers::handle(args, &client, &cli.format).await
                }
                Commands::Workspaces(args) => {
                    commands::workspaces::handle(args, &client, &cli.format).await
                }
                Commands::Tags(args) => {
                    commands::tags::handle(args, &client, &cli.format).await
                }
                Commands::Triggers(args) => {
                    commands::triggers::handle(args, &client, &cli.format).await
                }
                Commands::Variables(args) => {
                    commands::variables::handle(args, &client, &cli.format).await
                }
                Commands::Folders(args) => {
                    commands::folders::handle(args, &client, &cli.format).await
                }
                Commands::Templates(args) => {
                    commands::templates::handle(args, &client, &cli.format).await
                }
                Commands::Versions(args) => {
                    commands::versions::handle(args, &client, &cli.format).await
                }
                Commands::VersionHeaders(args) => {
                    commands::version_headers::handle(args, &client, &cli.format).await
                }
                Commands::Environments(args) => {
                    commands::environments::handle(args, &client, &cli.format).await
                }
                Commands::Permissions(args) => {
                    commands::permissions::handle(args, &client, &cli.format).await
                }
                Commands::Clients(args) => {
                    commands::clients::handle(args, &client, &cli.format).await
                }
                Commands::GtagConfigs(args) => {
                    commands::gtag_configs::handle(args, &client, &cli.format).await
                }
                Commands::Transformations(args) => {
                    commands::transformations::handle(args, &client, &cli.format).await
                }
                Commands::Zones(args) => {
                    commands::zones::handle(args, &client, &cli.format).await
                }
                Commands::BuiltinVariables(args) => {
                    commands::builtin_variables::handle(args, &client, &cli.format).await
                }
                Commands::Setup(args) => {
                    commands::setup::handle(args, &client, &cli.format).await
                }
            }
        }
    };

    if let Err(e) = result {
        e.exit_with_message();
    }
}
