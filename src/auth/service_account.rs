use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::Utc;
use rsa::pkcs8::DecodePrivateKey;
use rsa::sha2::Sha256;
use rsa::signature::SignatureEncoding;
use rsa::{pkcs1v15::SigningKey, signature::Signer, RsaPrivateKey};
use serde::Deserialize;
use std::path::Path;

use super::token_store::{self, TokenData};
use crate::config::{Config, SCOPES};
use crate::error::{GtmError, Result};

const TOKEN_URI: &str = "https://oauth2.googleapis.com/token";

#[derive(Debug, Deserialize)]
pub struct ServiceAccountKey {
    pub client_email: String,
    pub private_key: String,
    pub private_key_id: String,
    #[serde(default)]
    pub token_uri: Option<String>,
}

pub fn load_key(path: &Path) -> Result<ServiceAccountKey> {
    if !path.exists() {
        return Err(GtmError::CredentialsNotFound {
            path: path.display().to_string(),
        });
    }
    let content = std::fs::read_to_string(path)?;
    let key: ServiceAccountKey =
        serde_json::from_str(&content).map_err(|e| GtmError::InvalidParams(e.to_string()))?;
    Ok(key)
}

pub async fn login(key_path: &Path, token_path: &Path) -> Result<TokenData> {
    let key = load_key(key_path)?;
    let jwt = build_jwt(&key)?;
    let token_uri = key.token_uri.as_deref().unwrap_or(TOKEN_URI);
    let token = exchange_jwt(&jwt, token_uri).await?;
    token_store::save_token(token_path, &token)?;
    eprintln!(
        "Authentication successful (service account: {}).",
        key.client_email
    );
    Ok(token)
}

pub async fn ensure_valid_token(config: &Config, key_path: &Path) -> Result<String> {
    if let Some(token) = token_store::load_token(&config.token_path)? {
        if !token.is_expired() {
            return Ok(token.access_token);
        }
    }
    // Service accounts don't have refresh tokens; re-mint JWT
    let token = login(key_path, &config.token_path).await?;
    Ok(token.access_token)
}

fn build_jwt(key: &ServiceAccountKey) -> Result<String> {
    let header = serde_json::json!({
        "alg": "RS256",
        "typ": "JWT",
        "kid": key.private_key_id,
    });

    let now = Utc::now().timestamp();
    let claims = serde_json::json!({
        "iss": key.client_email,
        "scope": SCOPES.join(" "),
        "aud": TOKEN_URI,
        "iat": now,
        "exp": now + 3600,
    });

    let header_b64 = URL_SAFE_NO_PAD.encode(header.to_string().as_bytes());
    let claims_b64 = URL_SAFE_NO_PAD.encode(claims.to_string().as_bytes());
    let unsigned = format!("{header_b64}.{claims_b64}");

    let private_key = RsaPrivateKey::from_pkcs8_pem(&key.private_key)
        .map_err(|e| GtmError::InvalidParams(format!("Failed to parse private key: {e}")))?;
    let signing_key = SigningKey::<Sha256>::new(private_key);
    let signature = signing_key.sign(unsigned.as_bytes());
    let sig_b64 = URL_SAFE_NO_PAD.encode(signature.to_bytes());

    Ok(format!("{unsigned}.{sig_b64}"))
}

#[derive(Deserialize)]
struct JwtTokenResponse {
    access_token: String,
    expires_in: Option<i64>,
}

async fn exchange_jwt(jwt: &str, token_uri: &str) -> Result<TokenData> {
    let client = reqwest::Client::new();
    let resp = client
        .post(token_uri)
        .form(&[
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", jwt),
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

    let token_resp: JwtTokenResponse = resp.json().await?;
    Ok(TokenData {
        access_token: token_resp.access_token,
        refresh_token: None,
        expires_at: token_resp
            .expires_in
            .map(|s| Utc::now() + chrono::Duration::seconds(s)),
        expiry_date: None,
    })
}
