use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use std::path::Path;

use crate::error::Result;

/// Deserialize expiry_date from either a number or a numeric string.
fn deserialize_expiry_date<'de, D>(deserializer: D) -> std::result::Result<Option<i64>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum NumOrStr {
        Num(i64),
        Str(String),
    }

    Option::<NumOrStr>::deserialize(deserializer).map(|opt| {
        opt.and_then(|v| match v {
            NumOrStr::Num(n) => Some(n),
            NumOrStr::Str(s) => s.parse::<i64>().ok(),
        })
    })
}

/// Token data compatible with both gtm-cli native format and gtm-mcp format.
/// gtm-mcp uses `expiry_date` (epoch ms), gtm-cli uses `expires_at` (ISO 8601).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenData {
    pub access_token: String,
    pub refresh_token: Option<String>,
    /// ISO 8601 expiry (gtm-cli native format)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    /// Epoch milliseconds expiry (gtm-mcp compatibility).
    /// Can be stored as either a number or a string.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_expiry_date"
    )]
    pub expiry_date: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub installed: InstalledCredentials,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledCredentials {
    pub client_id: String,
    pub client_secret: String,
    #[serde(default)]
    pub redirect_uris: Vec<String>,
}

impl TokenData {
    /// Get the effective expiry time, checking both formats.
    pub fn effective_expires_at(&self) -> Option<DateTime<Utc>> {
        self.expires_at
            .or_else(|| self.expiry_date.and_then(DateTime::from_timestamp_millis))
    }

    pub fn is_expired(&self) -> bool {
        match self.effective_expires_at() {
            Some(expires_at) => {
                let buffer = chrono::Duration::seconds(60);
                Utc::now() + buffer >= expires_at
            }
            None => true,
        }
    }
}

pub fn load_token(path: &Path) -> Result<Option<TokenData>> {
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(path)?;
    let token: TokenData = serde_json::from_str(&content)?;
    Ok(Some(token))
}

pub fn save_token(path: &Path, token: &TokenData) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(token)?;
    std::fs::write(path, content)?;
    Ok(())
}

pub fn load_credentials(path: &Path) -> Result<Credentials> {
    // Use user-provided credentials.json if it exists, otherwise fall back to built-in
    if path.exists() {
        let content = std::fs::read_to_string(path)?;
        let creds: Credentials = serde_json::from_str(&content)?;
        Ok(creds)
    } else {
        Ok(builtin_credentials())
    }
}

fn builtin_credentials() -> Credentials {
    use crate::config::{BUILTIN_CLIENT_ID, BUILTIN_CLIENT_SECRET};
    Credentials {
        installed: InstalledCredentials {
            client_id: BUILTIN_CLIENT_ID.to_string(),
            client_secret: BUILTIN_CLIENT_SECRET.to_string(),
            redirect_uris: vec!["http://localhost".to_string()],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_token_not_expired() {
        let token = TokenData {
            access_token: "test".into(),
            refresh_token: Some("refresh".into()),
            expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
            expiry_date: None,
        };
        assert!(!token.is_expired());
    }

    #[test]
    fn test_token_expired() {
        let token = TokenData {
            access_token: "test".into(),
            refresh_token: Some("refresh".into()),
            expires_at: Some(Utc::now() - chrono::Duration::hours(1)),
            expiry_date: None,
        };
        assert!(token.is_expired());
    }

    #[test]
    fn test_token_expired_within_buffer() {
        let token = TokenData {
            access_token: "test".into(),
            refresh_token: None,
            expires_at: Some(Utc::now() + chrono::Duration::seconds(30)),
            expiry_date: None,
        };
        assert!(token.is_expired()); // 30s < 60s buffer
    }

    #[test]
    fn test_token_none_expires_at_is_expired() {
        let token = TokenData {
            access_token: "test".into(),
            refresh_token: None,
            expires_at: None,
            expiry_date: None,
        };
        assert!(token.is_expired());
    }

    #[test]
    fn test_save_and_load_token() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("token.json");

        let token = TokenData {
            access_token: "access123".into(),
            refresh_token: Some("refresh456".into()),
            expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
            expiry_date: None,
        };

        save_token(&path, &token).unwrap();
        let loaded = load_token(&path).unwrap().unwrap();

        assert_eq!(loaded.access_token, "access123");
        assert_eq!(loaded.refresh_token.unwrap(), "refresh456");
    }

    #[test]
    fn test_load_token_missing_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nonexistent.json");
        assert!(load_token(&path).unwrap().is_none());
    }

    #[test]
    fn test_save_creates_parent_dirs() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("sub").join("dir").join("token.json");

        let token = TokenData {
            access_token: "test".into(),
            refresh_token: None,
            expires_at: None,
            expiry_date: None,
        };

        save_token(&path, &token).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn test_expiry_date_compatibility() {
        // gtm-mcp format: expiry_date as epoch milliseconds
        let future_ms = (Utc::now() + chrono::Duration::hours(1)).timestamp_millis();
        let token = TokenData {
            access_token: "test".into(),
            refresh_token: None,
            expires_at: None,
            expiry_date: Some(future_ms),
        };
        assert!(!token.is_expired());

        let past_ms = (Utc::now() - chrono::Duration::hours(1)).timestamp_millis();
        let token_expired = TokenData {
            access_token: "test".into(),
            refresh_token: None,
            expires_at: None,
            expiry_date: Some(past_ms),
        };
        assert!(token_expired.is_expired());
    }

    #[test]
    fn test_load_mcp_format_token() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("token.json");
        let future_ms = (Utc::now() + chrono::Duration::hours(1)).timestamp_millis();
        std::fs::write(
            &path,
            format!(
                r#"{{"access_token":"abc","refresh_token":"def","expiry_date":{future_ms},"scope":"https://www.googleapis.com/auth/tagmanager.edit.containers","token_type":"Bearer"}}"#
            ),
        )
        .unwrap();

        let loaded = load_token(&path).unwrap().unwrap();
        assert_eq!(loaded.access_token, "abc");
        assert!(!loaded.is_expired());
    }

    #[test]
    fn test_load_credentials_missing_uses_builtin() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nope.json");
        let creds = load_credentials(&path).unwrap();
        assert!(!creds.installed.client_id.is_empty());
    }

    #[test]
    fn test_load_credentials_valid() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("creds.json");
        std::fs::write(
            &path,
            r#"{"installed":{"client_id":"id","client_secret":"secret","redirect_uris":["http://localhost"]}}"#,
        )
        .unwrap();

        let creds = load_credentials(&path).unwrap();
        assert_eq!(creds.installed.client_id, "id");
        assert_eq!(creds.installed.client_secret, "secret");
    }
}
