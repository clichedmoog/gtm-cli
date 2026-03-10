mod api;
mod app_config;
mod auth;
mod commands;
mod config;
mod error;
mod output;

use clap::{Parser, Subcommand};

use api::client::GtmApiClient;
use app_config::AppConfig;
use config::Config;
use output::formatter::OutputFormat;

#[derive(Parser)]
#[command(name = "gtm", version, about = "Google Tag Manager CLI")]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output format
    #[arg(long, global = true)]
    format: Option<OutputFormat>,

    /// Show what would be done without making changes
    #[arg(long, global = true)]
    dry_run: bool,

    /// Suppress non-essential output
    #[arg(long, short, global = true)]
    quiet: bool,

    /// Disable colored output
    #[arg(long, global = true)]
    no_color: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// AI agent guide and documentation
    Agent(commands::agent::AgentArgs),
    /// Authenticate with Google
    Auth(commands::auth::AuthArgs),
    /// Manage configuration defaults
    Config(commands::config::ConfigArgs),
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
    /// Manage GTM destinations
    Destinations(commands::destinations::DestinationsArgs),
    /// Quick setup workflows (GA4, Facebook Pixel, etc.)
    Setup(commands::setup::SetupArgs),
    /// Upgrade to the latest version
    Upgrade(commands::upgrade::UpgradeArgs),
    /// Generate shell completions
    Completions(commands::completions::CompletionsArgs),
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let config = Config::load();

    // Set NO_COLOR env var if --no-color flag is used (respected by comfy-table and others)
    if cli.no_color || std::env::var("NO_COLOR").is_ok() {
        std::env::set_var("NO_COLOR", "1");
    }

    // Set quiet mode via env var so handlers can check it
    if cli.quiet {
        std::env::set_var("GTM_QUIET", "1");
    }

    // Resolve output format: CLI flag > app_config > auto-detect
    // When piped (non-TTY stdout), default to JSON for scripting
    use std::io::IsTerminal;
    let app_config = AppConfig::load(&Config::config_dir().join("config.json"));
    let format = cli.format.unwrap_or_else(|| {
        app_config
            .output_format
            .as_deref()
            .and_then(|s| match s {
                "json" => Some(OutputFormat::Json),
                "table" => Some(OutputFormat::Table),
                "compact" => Some(OutputFormat::Compact),
                _ => None,
            })
            .unwrap_or_else(|| {
                if std::io::stdout().is_terminal() {
                    OutputFormat::Table
                } else {
                    OutputFormat::Json
                }
            })
    });

    let result = match cli.command {
        Commands::Agent(args) => commands::agent::handle(args),
        Commands::Auth(args) => commands::auth::handle(args, &config).await,
        Commands::Upgrade(args) => commands::upgrade::handle(args).await,
        Commands::Completions(args) => commands::completions::handle(args),
        Commands::Config(args) => {
            // Setup needs a client; get/set/unset don't
            let needs_client = matches!(args.action, commands::config::ConfigAction::Setup);
            if needs_client {
                let client = GtmApiClient::new(config.clone(), cli.dry_run);
                commands::config::handle(args, Some(&client), &config, &format).await
            } else {
                commands::config::handle(args, None, &config, &format).await
            }
        }
        _ => {
            let client = GtmApiClient::new(config, cli.dry_run);
            match cli.command {
                Commands::Agent(_)
                | Commands::Auth(_)
                | Commands::Upgrade(_)
                | Commands::Completions(_)
                | Commands::Config(_) => {
                    unreachable!()
                }
                Commands::Accounts(args) => {
                    commands::accounts::handle(args, &client, &format).await
                }
                Commands::Containers(args) => {
                    commands::containers::handle(args, &client, &format).await
                }
                Commands::Workspaces(args) => {
                    commands::workspaces::handle(args, &client, &format).await
                }
                Commands::Tags(args) => commands::tags::handle(args, &client, &format).await,
                Commands::Triggers(args) => {
                    commands::triggers::handle(args, &client, &format).await
                }
                Commands::Variables(args) => {
                    commands::variables::handle(args, &client, &format).await
                }
                Commands::Folders(args) => commands::folders::handle(args, &client, &format).await,
                Commands::Templates(args) => {
                    commands::templates::handle(args, &client, &format).await
                }
                Commands::Versions(args) => {
                    commands::versions::handle(args, &client, &format).await
                }
                Commands::VersionHeaders(args) => {
                    commands::version_headers::handle(args, &client, &format).await
                }
                Commands::Environments(args) => {
                    commands::environments::handle(args, &client, &format).await
                }
                Commands::Permissions(args) => {
                    commands::permissions::handle(args, &client, &format).await
                }
                Commands::Clients(args) => commands::clients::handle(args, &client, &format).await,
                Commands::GtagConfigs(args) => {
                    commands::gtag_configs::handle(args, &client, &format).await
                }
                Commands::Transformations(args) => {
                    commands::transformations::handle(args, &client, &format).await
                }
                Commands::Zones(args) => commands::zones::handle(args, &client, &format).await,
                Commands::BuiltinVariables(args) => {
                    commands::builtin_variables::handle(args, &client, &format).await
                }
                Commands::Destinations(args) => {
                    commands::destinations::handle(args, &client, &format).await
                }
                Commands::Setup(args) => commands::setup::handle(args, &client, &format).await,
            }
        }
    };

    if let Err(e) = result {
        e.exit_with_message();
    }
}
