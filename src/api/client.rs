use serde_json::Value;

use crate::auth::oauth;
use crate::config::{Config, API_BASE};
use crate::error::{GtmError, Result};

pub struct GtmApiClient {
    http: reqwest::Client,
    config: Config,
}

impl GtmApiClient {
    pub fn new(config: Config) -> Self {
        Self {
            http: reqwest::Client::new(),
            config,
        }
    }

    async fn auth_header(&self) -> Result<String> {
        let token = oauth::ensure_valid_token(&self.config).await?;
        Ok(format!("Bearer {token}"))
    }

    pub async fn get(&self, path: &str) -> Result<Value> {
        let url = format!("{API_BASE}/{path}");
        let auth = self.auth_header().await?;
        let resp = self.http.get(&url).header("Authorization", &auth).send().await?;
        Self::handle_response(resp).await
    }

    pub async fn post(&self, path: &str, body: &Value) -> Result<Value> {
        let url = format!("{API_BASE}/{path}");
        let auth = self.auth_header().await?;
        let resp = self
            .http
            .post(&url)
            .header("Authorization", &auth)
            .json(body)
            .send()
            .await?;
        Self::handle_response(resp).await
    }

    /// POST with query parameters (e.g., for move_entities_to_folder)
    pub async fn post_with_query(
        &self,
        path: &str,
        query: &[(&str, &str)],
        body: &Value,
    ) -> Result<Value> {
        let url = format!("{API_BASE}/{path}");
        let auth = self.auth_header().await?;
        let resp = self
            .http
            .post(&url)
            .header("Authorization", &auth)
            .query(query)
            .json(body)
            .send()
            .await?;
        Self::handle_response(resp).await
    }

    pub async fn put(&self, path: &str, body: &Value) -> Result<Value> {
        let url = format!("{API_BASE}/{path}");
        let auth = self.auth_header().await?;
        let resp = self
            .http
            .put(&url)
            .header("Authorization", &auth)
            .json(body)
            .send()
            .await?;
        Self::handle_response(resp).await
    }

    /// DELETE with query parameters
    pub async fn delete_with_query(&self, path: &str, query: &[(&str, &str)]) -> Result<()> {
        let url = format!("{API_BASE}/{path}");
        let auth = self.auth_header().await?;
        let resp = self
            .http
            .delete(&url)
            .header("Authorization", &auth)
            .query(query)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = Self::extract_error_message(resp).await;
            return Err(GtmError::ApiError { status, message });
        }
        Ok(())
    }

    pub async fn delete(&self, path: &str) -> Result<()> {
        let url = format!("{API_BASE}/{path}");
        let auth = self.auth_header().await?;
        let resp = self
            .http
            .delete(&url)
            .header("Authorization", &auth)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = Self::extract_error_message(resp).await;
            return Err(GtmError::ApiError { status, message });
        }
        Ok(())
    }

    async fn handle_response(resp: reqwest::Response) -> Result<Value> {
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = Self::extract_error_message(resp).await;
            return Err(GtmError::ApiError { status, message });
        }
        let body = resp.json::<Value>().await?;
        Ok(body)
    }

    async fn extract_error_message(resp: reqwest::Response) -> String {
        resp.json::<Value>()
            .await
            .ok()
            .and_then(|v| {
                v.get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                    .map(String::from)
            })
            .unwrap_or_else(|| "Unknown error".into())
    }
}
