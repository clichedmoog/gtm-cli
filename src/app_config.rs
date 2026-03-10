use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::error::{GtmError, Result};

const VALID_KEYS: &[&str] = &[
    "defaultAccountId",
    "defaultContainerId",
    "defaultWorkspaceId",
    "outputFormat",
];

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_account_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_container_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_workspace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,
}

impl AppConfig {
    pub fn load(path: &Path) -> Self {
        if !path.exists() {
            return Self::default();
        }
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        match key {
            "defaultAccountId" => self.default_account_id.as_deref(),
            "defaultContainerId" => self.default_container_id.as_deref(),
            "defaultWorkspaceId" => self.default_workspace_id.as_deref(),
            "outputFormat" => self.output_format.as_deref(),
            _ => None,
        }
    }

    pub fn set(&mut self, key: &str, value: String) -> Result<()> {
        if !VALID_KEYS.contains(&key) {
            return Err(GtmError::InvalidParams(format!(
                "Unknown config key '{key}'. Valid keys: {}",
                VALID_KEYS.join(", ")
            )));
        }
        if key == "outputFormat" && !["json", "table", "compact"].contains(&value.as_str()) {
            return Err(GtmError::InvalidParams(
                "outputFormat must be 'json', 'table', or 'compact'".into(),
            ));
        }
        match key {
            "defaultAccountId" => self.default_account_id = Some(value),
            "defaultContainerId" => self.default_container_id = Some(value),
            "defaultWorkspaceId" => self.default_workspace_id = Some(value),
            "outputFormat" => self.output_format = Some(value),
            _ => {}
        }
        Ok(())
    }

    pub fn unset(&mut self, key: &str) -> Result<()> {
        if !VALID_KEYS.contains(&key) {
            return Err(GtmError::InvalidParams(format!(
                "Unknown config key '{key}'. Valid keys: {}",
                VALID_KEYS.join(", ")
            )));
        }
        match key {
            "defaultAccountId" => self.default_account_id = None,
            "defaultContainerId" => self.default_container_id = None,
            "defaultWorkspaceId" => self.default_workspace_id = None,
            "outputFormat" => self.output_format = None,
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_load_missing() {
        let dir = TempDir::new().unwrap();
        let config = AppConfig::load(&dir.path().join("missing.json"));
        assert!(config.default_account_id.is_none());
    }

    #[test]
    fn test_save_and_load() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.json");
        let mut config = AppConfig::default();
        config.default_account_id = Some("123".into());
        config.save(&path).unwrap();

        let loaded = AppConfig::load(&path);
        assert_eq!(loaded.default_account_id.as_deref(), Some("123"));
    }

    #[test]
    fn test_set_invalid_key() {
        let mut config = AppConfig::default();
        assert!(config.set("invalidKey", "val".into()).is_err());
    }

    #[test]
    fn test_set_invalid_format() {
        let mut config = AppConfig::default();
        assert!(config.set("outputFormat", "xml".into()).is_err());
    }

    #[test]
    fn test_set_and_get() {
        let mut config = AppConfig::default();
        config.set("defaultAccountId", "456".into()).unwrap();
        assert_eq!(config.get("defaultAccountId"), Some("456"));
    }

    #[test]
    fn test_unset() {
        let mut config = AppConfig::default();
        config.default_account_id = Some("123".into());
        config.unset("defaultAccountId").unwrap();
        assert!(config.default_account_id.is_none());
    }
}
