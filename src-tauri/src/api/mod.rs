//! Unofficial Claude usage API client (best-effort) and OAuth token refresh.

pub mod token_refresh;
pub mod usage_client;

/// User-Agent for the usage endpoint. MUST begin with `claude-code/`.
pub fn user_agent() -> String {
    format!("claude-code/{}", env!("CARGO_PKG_VERSION"))
}
