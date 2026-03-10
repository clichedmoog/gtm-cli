#[derive(thiserror::Error, Debug)]
pub enum GtmError {
    #[error("Authentication required. Run `gtm auth login` to authenticate.")]
    AuthRequired,

    #[error("Credentials file not found at {path}. Download OAuth 2.0 credentials from Google Cloud Console.")]
    CredentialsNotFound { path: String },

    #[error("Token expired and refresh failed: {0}")]
    TokenRefreshFailed(String),

    #[error("API error {status}: {message}")]
    ApiError { status: u16, message: String },

    #[error("Invalid parameter JSON: {0}")]
    InvalidParams(String),

    #[error(transparent)]
    Http(#[from] reqwest::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, GtmError>;

/// Display-friendly error output for CLI
impl GtmError {
    pub fn exit_with_message(&self) -> ! {
        eprintln!("Error: {self}");
        if matches!(
            self,
            GtmError::AuthRequired | GtmError::TokenRefreshFailed(_)
        ) {
            eprintln!("Hint: Run `gtm auth login` to authenticate.");
        }
        std::process::exit(1);
    }
}
