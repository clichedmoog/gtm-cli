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

    #[error("Validation failed: {0} error(s) found")]
    ValidationFailed(usize),

    #[error(transparent)]
    Http(#[from] reqwest::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, GtmError>;

/// Stable exit codes for programmatic use (AI agents, CI pipelines).
///
/// - 0: Success
/// - 1: API or general error
/// - 2: Authentication error
/// - 3: Validation error (from `gtm validate`)
/// - 4: Invalid input (bad parameters, malformed JSON)
impl GtmError {
    pub fn exit_code(&self) -> i32 {
        match self {
            GtmError::AuthRequired
            | GtmError::CredentialsNotFound { .. }
            | GtmError::TokenRefreshFailed(_) => 2,
            GtmError::ValidationFailed(_) => 3,
            GtmError::InvalidParams(_) | GtmError::Json(_) => 4,
            GtmError::ApiError { .. } | GtmError::Http(_) | GtmError::Io(_) => 1,
        }
    }

    /// Print a structured error to stderr and exit with the appropriate code.
    ///
    /// When stderr is not a terminal (piped), outputs JSON for machine parsing:
    /// `{"error": {"code": 2, "type": "auth_required", "message": "..."}}`
    pub fn exit_with_message(&self) -> ! {
        let code = self.exit_code();

        if !std::io::IsTerminal::is_terminal(&std::io::stderr()) {
            // Structured JSON for pipes/agents
            let error_type = match self {
                GtmError::AuthRequired => "auth_required",
                GtmError::CredentialsNotFound { .. } => "credentials_not_found",
                GtmError::TokenRefreshFailed(_) => "token_refresh_failed",
                GtmError::ApiError { .. } => "api_error",
                GtmError::InvalidParams(_) => "invalid_params",
                GtmError::ValidationFailed(_) => "validation_failed",
                GtmError::Http(_) => "http_error",
                GtmError::Io(_) => "io_error",
                GtmError::Json(_) => "json_error",
            };
            let json = serde_json::json!({
                "error": {
                    "code": code,
                    "type": error_type,
                    "message": format!("{self}"),
                }
            });
            eprintln!("{}", serde_json::to_string(&json).unwrap_or_default());
        } else {
            // Human-friendly for terminals
            eprintln!("Error: {self}");
            match self {
                GtmError::AuthRequired | GtmError::TokenRefreshFailed(_) => {
                    eprintln!("Hint: Run `gtm auth login` to authenticate.");
                }
                GtmError::ApiError {
                    status: 403,
                    message,
                } if message.contains("scope") || message.contains("authentication") => {
                    eprintln!("Hint: Insufficient OAuth scopes. Run `gtm auth login` to re-authenticate with required permissions.");
                    eprintln!("      Publishing requires the tagmanager.publish scope.");
                }
                GtmError::ApiError { status: 403, .. } => {
                    eprintln!("Hint: Check that your account has the necessary GTM permissions.");
                    eprintln!("      Use `gtm permissions list` to view current access.");
                }
                _ => {}
            }
        }

        std::process::exit(code);
    }
}
