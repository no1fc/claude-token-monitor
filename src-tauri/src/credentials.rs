//! Read Claude Code OAuth credentials from `~/.claude/.credentials.json`.
//!
//! SECURITY: the access/refresh tokens must never be logged, serialized to the
//! frontend, or embedded in errors. The `Debug` impl is manually redacted and
//! tokens are zeroized on drop.

use serde::Deserialize;
use zeroize::Zeroize;

use crate::error::{AppError, AppResult};
use crate::plan::PlanTier;

#[derive(Clone)]
pub struct Credentials {
    access_token: String,
    refresh_token: Option<String>,
    pub expires_at_ms: i64,
    pub subscription_type: Option<String>,
    pub rate_limit_tier: Option<String>,
}

impl Drop for Credentials {
    fn drop(&mut self) {
        self.access_token.zeroize();
        if let Some(r) = self.refresh_token.as_mut() {
            r.zeroize();
        }
    }
}

// Redacted debug — never print token material.
impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field("access_token", &"<redacted>")
            .field(
                "refresh_token",
                &self.refresh_token.as_ref().map(|_| "<redacted>"),
            )
            .field("expires_at_ms", &self.expires_at_ms)
            .field("subscription_type", &self.subscription_type)
            .field("rate_limit_tier", &self.rate_limit_tier)
            .finish()
    }
}

impl Credentials {
    /// Access the bearer token. Callers must not log the returned value.
    pub fn access_token(&self) -> &str {
        &self.access_token
    }

    pub fn refresh_token(&self) -> Option<&str> {
        self.refresh_token.as_deref()
    }

    pub fn plan_tier(&self) -> PlanTier {
        PlanTier::from_credentials(
            self.subscription_type.as_deref(),
            self.rate_limit_tier.as_deref(),
        )
    }

    /// True when the token is expired (or within `skew_secs` of expiry).
    pub fn is_expired(&self, now_ms: i64, skew_secs: i64) -> bool {
        now_ms >= self.expires_at_ms - skew_secs * 1000
    }
}

#[derive(Debug, Deserialize)]
struct CredentialsFile {
    #[serde(rename = "claudeAiOauth")]
    claude_ai_oauth: Option<OauthBlock>,
}

#[derive(Debug, Deserialize)]
struct OauthBlock {
    #[serde(rename = "accessToken")]
    access_token: Option<String>,
    #[serde(rename = "refreshToken")]
    refresh_token: Option<String>,
    #[serde(rename = "expiresAt")]
    expires_at: Option<i64>,
    #[serde(rename = "subscriptionType")]
    subscription_type: Option<String>,
    #[serde(rename = "rateLimitTier")]
    rate_limit_tier: Option<String>,
}

/// Parse credentials JSON content. Returns `Ok(None)` when no OAuth block /
/// access token is present (not an error — the app degrades to JSONL mode).
pub fn parse(content: &str) -> AppResult<Option<Credentials>> {
    let file: CredentialsFile = serde_json::from_str(content)?;
    let Some(block) = file.claude_ai_oauth else {
        return Ok(None);
    };
    let Some(access_token) = block.access_token.filter(|t| !t.is_empty()) else {
        return Ok(None);
    };
    Ok(Some(Credentials {
        access_token,
        refresh_token: block.refresh_token,
        expires_at_ms: block.expires_at.unwrap_or(0),
        subscription_type: block.subscription_type,
        rate_limit_tier: block.rate_limit_tier,
    }))
}

/// Load credentials from disk, falling back to the `CLAUDE_CODE_OAUTH_TOKEN`
/// environment variable. `Ok(None)` means "no credentials" (degrade gracefully).
pub fn load() -> AppResult<Option<Credentials>> {
    if let Some(path) = crate::paths::credentials_path() {
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                if let Some(creds) = parse(&content)? {
                    return Ok(Some(creds));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => { /* fall through */ }
            Err(e) => return Err(AppError::Io(e.to_string())),
        }
    }

    // Fallback: environment token (e.g. macOS Keychain users export this).
    if let Ok(token) = std::env::var("CLAUDE_CODE_OAUTH_TOKEN") {
        if !token.trim().is_empty() {
            return Ok(Some(Credentials {
                access_token: token,
                refresh_token: None,
                expires_at_ms: i64::MAX, // unknown; treat as non-expiring
                subscription_type: None,
                rate_limit_tier: None,
            }));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Placeholder (non-secret) token values for tests.
    const FAKE_ACCESS: &str = "test-access-token-value";
    const SAMPLE: &str = r#"{
      "claudeAiOauth": {
        "accessToken": "test-access-token-value",
        "refreshToken": "test-refresh-token-value",
        "expiresAt": 1780991216303,
        "subscriptionType": "max",
        "rateLimitTier": "default_claude_max_5x"
      }
    }"#;

    #[test]
    fn parses_oauth_block() {
        let c = parse(SAMPLE).unwrap().unwrap();
        assert_eq!(c.access_token(), FAKE_ACCESS);
        assert_eq!(c.plan_tier(), PlanTier::Max5x);
        assert_eq!(c.expires_at_ms, 1780991216303);
    }

    #[test]
    fn debug_is_redacted() {
        let c = parse(SAMPLE).unwrap().unwrap();
        let dbg = format!("{c:?}");
        assert!(!dbg.contains(FAKE_ACCESS), "debug must not leak token");
        assert!(
            !dbg.contains("test-refresh-token-value"),
            "debug must not leak refresh token"
        );
        assert!(dbg.contains("<redacted>"));
    }

    #[test]
    fn missing_oauth_block_is_none() {
        assert!(parse(r#"{"other":1}"#).unwrap().is_none());
    }

    #[test]
    fn expiry_with_skew() {
        let c = parse(SAMPLE).unwrap().unwrap();
        // exactly at expiry -> expired
        assert!(c.is_expired(1780991216303, 0));
        // a minute before, with 0 skew -> not expired
        assert!(!c.is_expired(1780991216303 - 60_000, 0));
        // a minute before, with 120s skew -> expired
        assert!(c.is_expired(1780991216303 - 60_000, 120));
    }
}
