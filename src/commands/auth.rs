use std::path::PathBuf;

use clap::{Args, Subcommand};

use crate::auth::{self, oauth, service_account, token_store, AuthMethod};
use crate::config::Config;
use crate::error::Result;

#[derive(Args)]
pub struct AuthArgs {
    #[command(subcommand)]
    pub action: AuthAction,
}

#[derive(Subcommand)]
pub enum AuthAction {
    /// Authenticate with Google (opens browser or use service account)
    Login(LoginArgs),
    /// Remove saved credentials
    Logout,
    /// Show authentication status
    Status,
}

#[derive(Args)]
pub struct LoginArgs {
    /// Path to service account JSON key file
    #[arg(long)]
    pub service_account: Option<PathBuf>,
}

pub async fn handle(args: AuthArgs, config: &Config) -> Result<()> {
    match args.action {
        AuthAction::Login(a) => {
            let config_dir = Config::config_dir();
            if let Some(sa_path) = a.service_account {
                // Validate key file exists and is parseable
                service_account::load_key(&sa_path)?;
                service_account::login(&sa_path, &config.token_path).await?;
                let method = AuthMethod::ServiceAccount {
                    key_path: sa_path.display().to_string(),
                };
                auth::save_auth_method(&config_dir, &method)?;
            } else {
                oauth::login(config).await?;
                auth::save_auth_method(&config_dir, &AuthMethod::OAuth)?;
            }
        }
        AuthAction::Logout => {
            if config.token_path.exists() {
                std::fs::remove_file(&config.token_path)?;
                eprintln!("Token removed.");
            } else {
                eprintln!("No token found.");
            }
            // Also remove auth method
            let method_path = Config::config_dir().join("auth_method.json");
            if method_path.exists() {
                std::fs::remove_file(&method_path)?;
            }
        }
        AuthAction::Status => {
            let config_dir = Config::config_dir();

            // Show auth method
            if let Ok(sa_path) = std::env::var("GOOGLE_APPLICATION_CREDENTIALS") {
                eprintln!("Method: service_account (via GOOGLE_APPLICATION_CREDENTIALS)");
                eprintln!("Key: {sa_path}");
            } else if let Some(method) = auth::load_auth_method(&config_dir) {
                match &method {
                    AuthMethod::OAuth => eprintln!("Method: oauth"),
                    AuthMethod::ServiceAccount { key_path } => {
                        eprintln!("Method: service_account");
                        eprintln!("Key: {key_path}");
                    }
                }
            } else {
                eprintln!("Method: oauth (default)");
            }

            // Show token status
            match token_store::load_token(&config.token_path)? {
                Some(token) => {
                    if token.is_expired() {
                        eprintln!("Status: Token expired (will refresh on next request)");
                    } else {
                        eprintln!("Status: Authenticated");
                    }
                    if let Some(expires_at) = token.expires_at {
                        eprintln!("Expires: {expires_at}");
                    }
                    eprintln!("Has refresh token: {}", token.refresh_token.is_some());
                }
                None => {
                    eprintln!("Status: Not authenticated");
                    eprintln!("Run `gtm auth login` to authenticate.");
                }
            }
        }
    }
    Ok(())
}
