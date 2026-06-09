//! Client for the unofficial Claude usage endpoint that powers `/usage`.
//!
//! `GET https://api.anthropic.com/api/oauth/usage`
//! Headers: Authorization: Bearer <oauth>, anthropic-beta, anthropic-version,
//! and a `claude-code/<ver>` User-Agent (mandatory — a wrong UA lands you in an
//! aggressively rate-limited bucket).
//!
//! This is undocumented and best-effort: callers must degrade to JSONL on any
//! failure. The HTTP call itself is not unit-tested; `parse_usage` is.

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::error::{AppError, AppResult};

pub const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
pub const BETA_HEADER: &str = "oauth-2025-04-20";
pub const ANTHROPIC_VERSION: &str = "2023-06-01";

#[derive(Debug, Clone, Deserialize)]
pub struct WindowUtil {
    #[serde(default)]
    pub utilization: f64,
    #[serde(default)]
    pub resets_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct UsageApiResponse {
    #[serde(default)]
    pub five_hour: Option<WindowUtil>,
    #[serde(default)]
    pub seven_day: Option<WindowUtil>,
    #[serde(default)]
    pub seven_day_opus: Option<WindowUtil>,
    #[serde(default)]
    pub seven_day_sonnet: Option<WindowUtil>,
}

/// Parse a usage API JSON body. Pure — used directly in tests.
pub fn parse_usage(content: &str) -> AppResult<UsageApiResponse> {
    Ok(serde_json::from_str(content)?)
}

/// Perform the live request. The User-Agent MUST start with `claude-code/`.
pub async fn fetch_usage(
    client: &reqwest::Client,
    token: &str,
    user_agent: &str,
) -> AppResult<UsageApiResponse> {
    let resp = client
        .get(USAGE_URL)
        .header("Authorization", format!("Bearer {token}"))
        .header("anthropic-beta", BETA_HEADER)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .header("User-Agent", user_agent)
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| AppError::Api(e.to_string()))?;

    match resp.status().as_u16() {
        200 => {
            let text = resp.text().await.map_err(|e| AppError::Api(e.to_string()))?;
            parse_usage(&text)
        }
        401 => Err(AppError::Unauthorized),
        429 => Err(AppError::RateLimited),
        other => Err(AppError::Api(format!("HTTP {other}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
      "five_hour":  { "utilization": 33.0, "resets_at": "2026-04-11T07:00:00.528743+00:00" },
      "seven_day":  { "utilization": 13.0, "resets_at": "2026-04-17T00:59:59.951713+00:00" },
      "seven_day_opus": null,
      "seven_day_sonnet": { "utilization": 1.0, "resets_at": "2026-04-16T03:00:00.951719+00:00" },
      "extra_usage": { "is_enabled": false }
    }"#;

    #[test]
    fn parses_full_response() {
        let r = parse_usage(SAMPLE).unwrap();
        assert_eq!(r.five_hour.as_ref().unwrap().utilization, 33.0);
        assert!(r.five_hour.as_ref().unwrap().resets_at.is_some());
        assert_eq!(r.seven_day.as_ref().unwrap().utilization, 13.0);
        assert!(r.seven_day_opus.is_none());
        assert_eq!(r.seven_day_sonnet.as_ref().unwrap().utilization, 1.0);
    }

    #[test]
    fn parses_idle_zero_utilization() {
        let idle = r#"{"five_hour":{"utilization":0.0,"resets_at":null},"seven_day":{"utilization":0.0,"resets_at":null}}"#;
        let r = parse_usage(idle).unwrap();
        assert_eq!(r.five_hour.unwrap().utilization, 0.0);
    }

    #[test]
    fn tolerates_missing_fields() {
        let r = parse_usage("{}").unwrap();
        assert!(r.five_hour.is_none());
    }
}
