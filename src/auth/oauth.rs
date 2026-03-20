use chrono::Utc;
use serde::Deserialize;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use url::Url;

use crate::config::{Config, OAUTH_AUTH_URL, OAUTH_TOKEN_URL, SCOPES};
use crate::error::{GtmError, Result};

use super::token_store::{self, Credentials, TokenData};

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
}

/// Run the full OAuth2 login flow: open browser, receive code, exchange for tokens.
pub async fn login(config: &Config) -> Result<TokenData> {
    let creds = token_store::load_credentials(&config.credentials_path)?;

    // Bind to a random port for the redirect
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let redirect_uri = format!("http://127.0.0.1:{port}");

    let auth_url = build_auth_url(&creds, &redirect_uri);

    eprintln!("Opening browser for authentication...");
    eprintln!("If the browser doesn't open, visit:\n{auth_url}\n");

    if open::that(&auth_url).is_err() {
        eprintln!("Could not open browser automatically.");
    }

    // Wait for the redirect with the auth code
    let code = receive_auth_code(listener)?;

    // Exchange code for tokens
    let token = exchange_code(&creds, &code, &redirect_uri).await?;

    // Save tokens
    token_store::save_token(&config.token_path, &token)?;

    eprintln!("Authentication successful! Token saved.");
    Ok(token)
}

/// Ensure we have a valid access token, refreshing if needed.
pub async fn ensure_valid_token(config: &Config) -> Result<String> {
    let token = token_store::load_token(&config.token_path)?.ok_or(GtmError::AuthRequired)?;

    if !token.is_expired() {
        return Ok(token.access_token);
    }

    // Try to refresh
    let refresh_token = token
        .refresh_token
        .as_deref()
        .ok_or(GtmError::AuthRequired)?;

    let creds = token_store::load_credentials(&config.credentials_path)?;
    let new_token = refresh_access_token(&creds, refresh_token).await?;

    token_store::save_token(&config.token_path, &new_token)?;
    Ok(new_token.access_token)
}

fn build_auth_url(creds: &Credentials, redirect_uri: &str) -> String {
    let scope = SCOPES.join(" ");
    let mut url = Url::parse(OAUTH_AUTH_URL).unwrap();
    url.query_pairs_mut()
        .append_pair("client_id", &creds.installed.client_id)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", &scope)
        .append_pair("access_type", "offline")
        .append_pair("prompt", "consent");
    url.to_string()
}

fn receive_auth_code(listener: TcpListener) -> Result<String> {
    let (mut stream, _) = listener.accept()?;
    let mut reader = BufReader::new(&stream);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;

    // Parse: GET /?code=AUTH_CODE&scope=... HTTP/1.1
    let code = request_line
        .split_whitespace()
        .nth(1)
        .and_then(|path| Url::parse(&format!("http://localhost{path}")).ok())
        .and_then(|url| {
            url.query_pairs()
                .find(|(k, _)| k == "code")
                .map(|(_, v)| v.to_string())
        })
        .ok_or_else(|| GtmError::TokenRefreshFailed("No auth code received".into()))?;

    // Send a styled response to the browser
    let html = include_str!("../../assets/auth_success.html");
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\r\n{}",
        html.len(),
        html
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()?;

    Ok(code)
}

async fn exchange_code(creds: &Credentials, code: &str, redirect_uri: &str) -> Result<TokenData> {
    let client = reqwest::Client::new();
    let resp = client
        .post(OAUTH_TOKEN_URL)
        .form(&[
            ("client_id", creds.installed.client_id.as_str()),
            ("client_secret", creds.installed.client_secret.as_str()),
            ("code", code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect_uri),
        ])
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        return Err(GtmError::ApiError {
            status,
            message: body,
        });
    }

    let token_resp: TokenResponse = resp.json().await?;
    Ok(TokenData {
        access_token: token_resp.access_token,
        refresh_token: token_resp.refresh_token,
        expires_at: token_resp
            .expires_in
            .map(|s| Utc::now() + chrono::Duration::seconds(s)),
        expiry_date: None,
    })
}

async fn refresh_access_token(creds: &Credentials, refresh_token: &str) -> Result<TokenData> {
    let client = reqwest::Client::new();
    let resp = client
        .post(OAUTH_TOKEN_URL)
        .form(&[
            ("client_id", creds.installed.client_id.as_str()),
            ("client_secret", creds.installed.client_secret.as_str()),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(GtmError::TokenRefreshFailed(body));
    }

    let token_resp: TokenResponse = resp.json().await?;
    Ok(TokenData {
        access_token: token_resp.access_token,
        // Keep the existing refresh token if not returned
        refresh_token: token_resp
            .refresh_token
            .or_else(|| Some(refresh_token.to_string())),
        expires_at: token_resp
            .expires_in
            .map(|s| Utc::now() + chrono::Duration::seconds(s)),
        expiry_date: None,
    })
}
