pub mod oauth;
pub mod service_account;
pub mod token_store;

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum AuthMethod {
    #[serde(rename = "oauth")]
    OAuth,
    #[serde(rename = "service_account")]
    ServiceAccount { key_path: String },
}

pub fn load_auth_method(config_dir: &Path) -> Option<AuthMethod> {
    let path = config_dir.join("auth_method.json");
    if !path.exists() {
        return None;
    }
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
}

pub fn save_auth_method(config_dir: &Path, method: &AuthMethod) -> Result<()> {
    std::fs::create_dir_all(config_dir)?;
    let path = config_dir.join("auth_method.json");
    let content = serde_json::to_string_pretty(method)?;
    std::fs::write(path, content)?;
    Ok(())
}

/// Resolve a valid access token using the best available auth method.
/// Priority: GOOGLE_APPLICATION_CREDENTIALS env var > saved auth method > OAuth
pub async fn ensure_valid_token(config: &Config) -> Result<String> {
    // 1. Check GOOGLE_APPLICATION_CREDENTIALS env var
    if let Ok(sa_path) = std::env::var("GOOGLE_APPLICATION_CREDENTIALS") {
        return service_account::ensure_valid_token(config, &PathBuf::from(sa_path)).await;
    }

    // 2. Check saved auth method
    let config_dir = Config::config_dir();
    if let Some(method) = load_auth_method(&config_dir) {
        return match method {
            AuthMethod::ServiceAccount { key_path } => {
                service_account::ensure_valid_token(config, &PathBuf::from(key_path)).await
            }
            AuthMethod::OAuth => oauth::ensure_valid_token(config).await,
        };
    }

    // 3. Fall back to OAuth
    oauth::ensure_valid_token(config).await
}
