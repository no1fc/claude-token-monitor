//! OAuth token refresh. Used best-effort when the access token is expired.
//! By default the refreshed token is kept in-memory only and NOT written back
//! to `~/.claude/.credentials.json` (avoids racing Claude Code's own refresh).

use serde::Deserialize;

use crate::error::{AppError, AppResult};

pub const TOKEN_URL: &str = "https://platform.claude.com/v1/oauth/token";
pub const OAUTH_CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";

#[derive(Debug, Clone, Deserialize)]
pub struct RefreshedToken {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub expires_in: Option<i64>,
}

/// Parse a refresh response body. Pure — used in tests.
pub fn parse_refresh(content: &str) -> AppResult<RefreshedToken> {
    Ok(serde_json::from_str(content)?)
}

/// Perform the live refresh request.
pub async fn refresh(client: &reqwest::Client, refresh_token: &str) -> AppResult<RefreshedToken> {
    let body = serde_json::json!({
        "grant_type": "refresh_token",
        "refresh_token": refresh_token,
        "client_id": OAUTH_CLIENT_ID,
    });
    let resp = client
        .post(TOKEN_URL)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::Api(e.to_string()))?;

    match resp.status().as_u16() {
        200 => {
            let text = resp
                .text()
                .await
                .map_err(|e| AppError::Api(e.to_string()))?;
            parse_refresh(&text)
        }
        401 | 403 => Err(AppError::Unauthorized),
        other => Err(AppError::Api(format!("token refresh HTTP {other}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_refresh_response() {
        let body = r#"{"access_token":"test-access-token","refresh_token":"test-refresh-token","expires_in":3600}"#;
        let r = parse_refresh(body).unwrap();
        assert_eq!(r.access_token, "test-access-token");
        assert_eq!(r.expires_in, Some(3600));
    }

    #[test]
    fn tolerates_missing_optional_fields() {
        let r = parse_refresh(r#"{"access_token":"x"}"#).unwrap();
        assert!(r.refresh_token.is_none());
        assert!(r.expires_in.is_none());
    }
}
