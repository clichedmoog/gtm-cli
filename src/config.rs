use std::path::PathBuf;

pub const API_BASE: &str = "https://tagmanager.googleapis.com/tagmanager/v2";

pub const OAUTH_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const OAUTH_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

pub const BUILTIN_CLIENT_ID: &str =
    "945641231304-odi00pljfurl01pkaog18ppvuoqadvns.apps.googleusercontent.com";
pub const BUILTIN_CLIENT_SECRET: &str = "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";

pub const SCOPES: &[&str] = &[
    "https://www.googleapis.com/auth/tagmanager.edit.containers",
    "https://www.googleapis.com/auth/tagmanager.publish",
];

#[derive(Debug, Clone)]
pub struct Config {
    pub credentials_path: PathBuf,
    pub token_path: PathBuf,
}

impl Config {
    pub fn load() -> Self {
        let config_dir = Self::config_dir();

        let credentials_path = std::env::var("GTM_CREDENTIALS_FILE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| config_dir.join("credentials.json"));

        let token_path = std::env::var("GTM_TOKEN_FILE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| config_dir.join("token.json"));

        Self {
            credentials_path,
            token_path,
        }
    }

    pub fn config_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".config")
            .join("gtm")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_dir_is_under_config() {
        let dir = Config::config_dir();
        assert!(dir.ends_with("gtm"));
    }

    #[test]
    fn test_config_load_defaults() {
        // Clear env vars for test isolation
        std::env::remove_var("GTM_CREDENTIALS_FILE");
        std::env::remove_var("GTM_TOKEN_FILE");

        let config = Config::load();
        assert!(config.credentials_path.ends_with("credentials.json"));
        assert!(config.token_path.ends_with("token.json"));
    }

    #[test]
    fn test_config_load_from_env() {
        std::env::set_var("GTM_CREDENTIALS_FILE", "/tmp/creds.json");
        std::env::set_var("GTM_TOKEN_FILE", "/tmp/tok.json");

        let config = Config::load();
        assert_eq!(config.credentials_path, PathBuf::from("/tmp/creds.json"));
        assert_eq!(config.token_path, PathBuf::from("/tmp/tok.json"));

        // Cleanup
        std::env::remove_var("GTM_CREDENTIALS_FILE");
        std::env::remove_var("GTM_TOKEN_FILE");
    }
}
