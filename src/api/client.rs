use reqwest::RequestBuilder;
use serde_json::Value;

use crate::auth;
use crate::config::{Config, API_BASE};
use crate::error::{GtmError, Result};

const MAX_RETRIES: u32 = 3;
const RETRY_BASE_SECS: u64 = 5;

pub struct GtmApiClient {
    http: reqwest::Client,
    config: Config,
    dry_run: bool,
}

impl GtmApiClient {
    pub fn new(config: Config, dry_run: bool) -> Self {
        Self {
            http: reqwest::Client::new(),
            config,
            dry_run,
        }
    }

    async fn auth_header(&self) -> Result<String> {
        let token = auth::ensure_valid_token(&self.config).await?;
        Ok(format!("Bearer {token}"))
    }

    pub async fn get(&self, path: &str) -> Result<Value> {
        let url = format!("{API_BASE}/{path}");
        let auth = self.auth_header().await?;
        self.send_with_retry(|| self.http.get(&url).header("Authorization", &auth))
            .await
    }

    /// GET with automatic pagination. Collects all pages into a single response.
    /// The GTM API uses `nextPageToken` / `pageToken` for pagination.
    pub async fn get_all(&self, path: &str) -> Result<Value> {
        let auth = self.auth_header().await?;
        let mut all_results: Option<Value> = None;
        let mut page_token: Option<String> = None;

        loop {
            let mut url = format!("{API_BASE}/{path}");
            if let Some(ref token) = page_token {
                let sep = if url.contains('?') { '&' } else { '?' };
                url = format!("{url}{sep}pageToken={token}");
            }

            let result = self
                .send_with_retry(|| self.http.get(&url).header("Authorization", &auth))
                .await?;

            // Extract next page token
            page_token = result
                .get("nextPageToken")
                .and_then(|v| v.as_str())
                .map(String::from);

            match &mut all_results {
                None => {
                    all_results = Some(result);
                }
                Some(existing) => {
                    // Merge arrays from the response into the accumulated result
                    if let (Some(existing_obj), Some(new_obj)) =
                        (existing.as_object_mut(), result.as_object())
                    {
                        for (key, new_val) in new_obj {
                            if key == "nextPageToken" {
                                continue;
                            }
                            if let Some(new_arr) = new_val.as_array() {
                                if let Some(existing_arr) =
                                    existing_obj.get_mut(key).and_then(|v| v.as_array_mut())
                                {
                                    existing_arr.extend(new_arr.clone());
                                } else {
                                    existing_obj.insert(key.clone(), new_val.clone());
                                }
                            }
                        }
                    }
                }
            }

            if page_token.is_none() {
                break;
            }
        }

        // Remove nextPageToken from final result
        if let Some(ref mut result) = all_results {
            if let Some(obj) = result.as_object_mut() {
                obj.remove("nextPageToken");
            }
        }

        Ok(all_results.unwrap_or(Value::Object(serde_json::Map::new())))
    }

    pub async fn post(&self, path: &str, body: &Value) -> Result<Value> {
        if self.dry_run {
            self.print_dry_run("POST", path, Some(body));
            return Ok(body.clone());
        }
        let url = format!("{API_BASE}/{path}");
        let auth = self.auth_header().await?;
        let body = body.clone();
        self.send_with_retry(|| {
            self.http
                .post(&url)
                .header("Authorization", &auth)
                .json(&body)
        })
        .await
    }

    /// POST with query parameters (e.g., for move_entities_to_folder)
    pub async fn post_with_query(
        &self,
        path: &str,
        query: &[(&str, &str)],
        body: &Value,
    ) -> Result<Value> {
        if self.dry_run {
            self.print_dry_run("POST", path, Some(body));
            return Ok(body.clone());
        }
        let url = format!("{API_BASE}/{path}");
        let auth = self.auth_header().await?;
        let body = body.clone();
        let query: Vec<(String, String)> = query
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        self.send_with_retry(|| {
            self.http
                .post(&url)
                .header("Authorization", &auth)
                .query(&query)
                .json(&body)
        })
        .await
    }

    pub async fn put(&self, path: &str, body: &Value) -> Result<Value> {
        if self.dry_run {
            self.print_dry_run("PUT", path, Some(body));
            return Ok(body.clone());
        }
        let url = format!("{API_BASE}/{path}");
        let auth = self.auth_header().await?;
        let body = body.clone();
        self.send_with_retry(|| {
            self.http
                .put(&url)
                .header("Authorization", &auth)
                .json(&body)
        })
        .await
    }

    /// DELETE with query parameters
    pub async fn delete_with_query(&self, path: &str, query: &[(&str, &str)]) -> Result<()> {
        if self.dry_run {
            self.print_dry_run("DELETE", path, None);
            return Ok(());
        }
        let url = format!("{API_BASE}/{path}");
        let auth = self.auth_header().await?;
        let query: Vec<(String, String)> = query
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        self.send_delete_with_retry(|| {
            self.http
                .delete(&url)
                .header("Authorization", &auth)
                .query(&query)
        })
        .await
    }

    pub async fn delete(&self, path: &str) -> Result<()> {
        if self.dry_run {
            self.print_dry_run("DELETE", path, None);
            return Ok(());
        }
        let url = format!("{API_BASE}/{path}");
        let auth = self.auth_header().await?;
        self.send_delete_with_retry(|| self.http.delete(&url).header("Authorization", &auth))
            .await
    }

    fn print_dry_run(&self, method: &str, path: &str, body: Option<&Value>) {
        eprintln!("[dry-run] {method} {API_BASE}/{path}");
        if let Some(b) = body {
            if let Ok(pretty) = serde_json::to_string_pretty(b) {
                eprintln!("[dry-run] Body: {pretty}");
            }
        }
    }

    /// Send a request and parse JSON response, retrying on 429.
    async fn send_with_retry<F>(&self, build: F) -> Result<Value>
    where
        F: Fn() -> RequestBuilder,
    {
        for attempt in 0..=MAX_RETRIES {
            let resp = build().send().await?;
            if resp.status().as_u16() == 429 && attempt < MAX_RETRIES {
                let wait = RETRY_BASE_SECS * 2u64.pow(attempt);
                eprintln!("Rate limited, retrying in {wait}s...");
                tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
                continue;
            }
            return Self::handle_response(resp).await;
        }
        unreachable!()
    }

    /// Send a DELETE request, retrying on 429.
    async fn send_delete_with_retry<F>(&self, build: F) -> Result<()>
    where
        F: Fn() -> RequestBuilder,
    {
        for attempt in 0..=MAX_RETRIES {
            let resp = build().send().await?;
            if resp.status().as_u16() == 429 && attempt < MAX_RETRIES {
                let wait = RETRY_BASE_SECS * 2u64.pow(attempt);
                eprintln!("Rate limited, retrying in {wait}s...");
                tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
                continue;
            }
            if !resp.status().is_success() {
                let status = resp.status().as_u16();
                let message = Self::extract_error_message(resp).await;
                return Err(GtmError::ApiError { status, message });
            }
            return Ok(());
        }
        unreachable!()
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
