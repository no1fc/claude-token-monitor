//! Shared application state and the async snapshot-building orchestration that
//! ties JSONL scanning, credential loading, and the usage API together.

use std::sync::Mutex;

use chrono::Utc;

use crate::api::{self, usage_client::UsageApiResponse};
use crate::config::{self, Settings};
use crate::credentials::{self, Credentials};
use crate::jsonl::scan_dir;
use crate::model::UsageSnapshot;
use crate::plan::{resolve_limits, PlanTier};
use crate::snapshot;

/// A refreshed token held in memory only (never written back by default).
struct InMemoryToken {
    access_token: String,
    expires_at_ms: i64,
}

impl Drop for InMemoryToken {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.access_token.zeroize();
    }
}

pub struct AppState {
    http: reqwest::Client,
    pub settings: Mutex<Settings>,
    pub last_snapshot: Mutex<Option<UsageSnapshot>>,
    in_memory_token: Mutex<Option<InMemoryToken>>,
}

impl AppState {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .unwrap_or_default();
        AppState {
            http,
            settings: Mutex::new(config::load()),
            last_snapshot: Mutex::new(None),
            in_memory_token: Mutex::new(None),
        }
    }

    pub fn settings_snapshot(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }

    /// Build a fresh snapshot from all sources (including the API) and cache it.
    pub async fn build_snapshot(&self) -> UsageSnapshot {
        self.build_snapshot_inner(true).await
    }

    /// Build a snapshot from JSONL only (no API call). Used by the file watcher
    /// so frequent transcript writes don't hammer the rate-limited endpoint;
    /// the authoritative percentages refresh on the timer instead.
    pub async fn build_snapshot_local(&self) -> UsageSnapshot {
        self.build_snapshot_inner(false).await
    }

    async fn build_snapshot_inner(&self, allow_api: bool) -> UsageSnapshot {
        let settings = self.settings_snapshot();
        let now = Utc::now();

        // JSONL scan (blocking IO done inline; fast for typical sizes).
        let outcome = match crate::paths::projects_dir() {
            Some(dir) => scan_dir(&dir),
            None => Default::default(),
        };

        // Credentials + plan tier.
        let creds = credentials::load().ok().flatten();
        let detected_tier = creds
            .as_ref()
            .map(|c| c.plan_tier())
            .unwrap_or(PlanTier::Unknown);
        let tier = config::effective_tier(&settings, detected_tier);
        let limits = resolve_limits(tier, settings.plan_limit_overrides);

        // Best-effort API.
        let api: Option<UsageApiResponse> = if allow_api && settings.use_api {
            match creds.as_ref() {
                Some(c) => self.try_api(c).await,
                None => None,
            }
        } else {
            None
        };

        let snap = snapshot::build(
            &outcome.events,
            api.as_ref(),
            tier,
            limits,
            outcome.parse_failures,
            settings.context_limit_override,
            now,
        );

        *self.last_snapshot.lock().unwrap() = Some(snap.clone());
        snap
    }

    /// Try the usage API, refreshing the token once on 401. Returns None on any
    /// failure (caller degrades to JSONL estimates).
    async fn try_api(&self, creds: &Credentials) -> Option<UsageApiResponse> {
        let ua = api::user_agent();
        let token = self.current_token(creds);

        match api::usage_client::fetch_usage(&self.http, &token, &ua).await {
            Ok(resp) => Some(resp),
            Err(crate::error::AppError::Unauthorized) => self.refresh_and_retry(creds, &ua).await,
            Err(_) => None,
        }
    }

    /// Pick the freshest token: an in-memory refreshed one if still valid,
    /// otherwise the on-disk credential.
    fn current_token(&self, creds: &Credentials) -> String {
        let now_ms = Utc::now().timestamp_millis();
        if let Some(t) = self.in_memory_token.lock().unwrap().as_ref() {
            if now_ms < t.expires_at_ms {
                return t.access_token.clone();
            }
        }
        creds.access_token().to_string()
    }

    async fn refresh_and_retry(&self, creds: &Credentials, ua: &str) -> Option<UsageApiResponse> {
        let refresh_token = creds.refresh_token()?;
        let refreshed = api::token_refresh::refresh(&self.http, refresh_token)
            .await
            .ok()?;
        let now_ms = Utc::now().timestamp_millis();
        let expires_at_ms = refreshed
            .expires_in
            .map(|s| now_ms + s * 1000)
            .unwrap_or(now_ms + 3600 * 1000);

        let token = refreshed.access_token.clone();
        *self.in_memory_token.lock().unwrap() = Some(InMemoryToken {
            access_token: refreshed.access_token,
            expires_at_ms,
        });

        api::usage_client::fetch_usage(&self.http, &token, ua)
            .await
            .ok()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
