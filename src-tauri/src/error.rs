//! Application error type. IMPORTANT: variants must never embed credential
//! material. Messages are surfaced to the UI and logs.

use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("io error: {0}")]
    Io(String),

    #[error("json parse error: {0}")]
    Json(String),

    #[error("no Claude data found at {0}")]
    NoData(String),

    #[error("credentials unavailable")]
    NoCredentials,

    #[error("usage API unauthorized")]
    Unauthorized,

    #[error("usage API rate limited")]
    RateLimited,

    #[error("usage API request failed: {0}")]
    Api(String),

    #[error("config error: {0}")]
    Config(String),

    #[error("{0}")]
    Other(String),
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Json(e.to_string())
    }
}

/// Serialized error sent across the Tauri IPC boundary. Carries a coarse `kind`
/// plus a human message — never any token.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WireError {
    pub kind: String,
    pub message: String,
}

impl From<&AppError> for WireError {
    fn from(e: &AppError) -> Self {
        let kind = match e {
            AppError::Io(_) => "io",
            AppError::Json(_) => "json",
            AppError::NoData(_) => "noData",
            AppError::NoCredentials => "noCredentials",
            AppError::Unauthorized => "unauthorized",
            AppError::RateLimited => "rateLimited",
            AppError::Api(_) => "api",
            AppError::Config(_) => "config",
            AppError::Other(_) => "other",
        };
        WireError {
            kind: kind.to_string(),
            message: e.to_string(),
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        WireError::from(self).serialize(s)
    }
}

pub type AppResult<T> = Result<T, AppError>;
