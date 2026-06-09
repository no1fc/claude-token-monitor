//! User settings, persisted as JSON in the OS config directory.

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::plan::{PlanLimits, PlanTier};

/// Hard floor on the API poll interval to avoid the aggressive 429 bucket.
pub const MIN_REFRESH_SECS: u64 = 60;
pub const DEFAULT_REFRESH_SECS: u64 = 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub refresh_interval_secs: u64,
    pub use_api: bool,
    pub plan_override: Option<PlanTier>,
    pub plan_limit_overrides: Option<PlanLimits>,
    pub write_back_tokens: bool,
    pub always_on_top: bool,
    pub opacity: f64,
    pub theme: String,
    pub currency: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            refresh_interval_secs: DEFAULT_REFRESH_SECS,
            use_api: true,
            plan_override: None,
            plan_limit_overrides: None,
            write_back_tokens: false,
            always_on_top: true,
            opacity: 1.0,
            theme: "dark".to_string(),
            currency: "USD".to_string(),
        }
    }
}

impl Settings {
    /// Clamp values into safe ranges (called after load and before save).
    pub fn sanitized(mut self) -> Self {
        if self.refresh_interval_secs < MIN_REFRESH_SECS {
            self.refresh_interval_secs = MIN_REFRESH_SECS;
        }
        self.opacity = self.opacity.clamp(0.2, 1.0);
        self
    }
}

/// Load settings from disk, returning defaults if the file is absent or invalid.
pub fn load() -> Settings {
    let Some(path) = crate::paths::settings_path() else {
        return Settings::default();
    };
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str::<Settings>(&content)
            .map(Settings::sanitized)
            .unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

/// Persist settings atomically (write temp then rename).
pub fn save(settings: &Settings) -> AppResult<()> {
    let dir =
        crate::paths::settings_dir().ok_or_else(|| AppError::Config("no config dir".into()))?;
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("settings.json");
    let tmp = dir.join("settings.json.tmp");
    let json = serde_json::to_string_pretty(settings)?;
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

/// Effective plan tier: explicit override wins over detected tier.
pub fn effective_tier(settings: &Settings, detected: PlanTier) -> PlanTier {
    settings.plan_override.unwrap_or(detected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sane() {
        let s = Settings::default();
        assert!(s.use_api);
        assert_eq!(s.refresh_interval_secs, DEFAULT_REFRESH_SECS);
        assert!(!s.write_back_tokens);
    }

    #[test]
    fn sanitize_enforces_min_interval_and_opacity() {
        let s = Settings {
            refresh_interval_secs: 5,
            opacity: 5.0,
            ..Default::default()
        }
        .sanitized();
        assert_eq!(s.refresh_interval_secs, MIN_REFRESH_SECS);
        assert_eq!(s.opacity, 1.0);
    }

    #[test]
    fn override_tier_wins() {
        let s = Settings {
            plan_override: Some(PlanTier::Max20x),
            ..Default::default()
        };
        assert_eq!(effective_tier(&s, PlanTier::Pro), PlanTier::Max20x);
        let s2 = Settings::default();
        assert_eq!(effective_tier(&s2, PlanTier::Pro), PlanTier::Pro);
    }

    #[test]
    fn round_trips_through_json() {
        let s = Settings::default();
        let json = serde_json::to_string(&s).unwrap();
        let back: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(back.theme, s.theme);
        assert_eq!(back.currency, s.currency);
    }
}
