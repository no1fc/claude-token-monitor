//! Claude subscription plan tiers and their approximate token ceilings.
//!
//! NOTE: Anthropic does not publish exact token limits per window, and they
//! change over time. These constants are ONLY used for the JSONL-estimation
//! fallback gauges. When the usage API is reachable, gauges use the API's
//! authoritative percentage instead and never rely on these numbers.
//! All values are user-overridable in settings.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PlanTier {
    Pro,
    Max5x,
    Max20x,
    Team,
    Unknown,
}

impl PlanTier {
    /// Human-readable label for the UI badge.
    pub fn label(&self) -> &'static str {
        match self {
            PlanTier::Pro => "Pro",
            PlanTier::Max5x => "Max 5x",
            PlanTier::Max20x => "Max 20x",
            PlanTier::Team => "Team",
            PlanTier::Unknown => "Unknown",
        }
    }

    /// Map Claude Code's `subscriptionType` + `rateLimitTier` strings to a tier.
    pub fn from_credentials(
        subscription_type: Option<&str>,
        rate_limit_tier: Option<&str>,
    ) -> PlanTier {
        if let Some(tier) = rate_limit_tier {
            let t = tier.to_ascii_lowercase();
            if t.contains("max") && t.contains("20") {
                return PlanTier::Max20x;
            }
            if t.contains("max") && t.contains("5") {
                return PlanTier::Max5x;
            }
            if t.contains("max") {
                return PlanTier::Max5x; // default Max bucket
            }
            if t.contains("pro") {
                return PlanTier::Pro;
            }
            if t.contains("team") {
                return PlanTier::Team;
            }
        }
        match subscription_type.map(|s| s.to_ascii_lowercase()) {
            Some(s) if s.contains("max") => PlanTier::Max5x,
            Some(s) if s.contains("pro") => PlanTier::Pro,
            Some(s) if s.contains("team") => PlanTier::Team,
            _ => PlanTier::Unknown,
        }
    }
}

/// Approximate token ceilings for a plan, per window. See module note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanLimits {
    pub five_hour_tokens: u64,
    pub seven_day_tokens: u64,
}

// Baseline figures derived from community measurements (counting all token
// categories incl. cache). APPROX — calibrate against your own `/usage`.
const PRO_5H: u64 = 19_000_000;
const PRO_7D: u64 = 130_000_000;

pub fn default_limits(tier: PlanTier) -> PlanLimits {
    match tier {
        PlanTier::Pro => PlanLimits {
            five_hour_tokens: PRO_5H,
            seven_day_tokens: PRO_7D,
        },
        PlanTier::Max5x => PlanLimits {
            five_hour_tokens: PRO_5H * 5,
            seven_day_tokens: PRO_7D * 5,
        },
        PlanTier::Max20x => PlanLimits {
            five_hour_tokens: PRO_5H * 20,
            seven_day_tokens: PRO_7D * 20,
        },
        PlanTier::Team => PlanLimits {
            five_hour_tokens: PRO_5H,
            seven_day_tokens: PRO_7D,
        },
        PlanTier::Unknown => PlanLimits {
            five_hour_tokens: PRO_5H * 5,
            seven_day_tokens: PRO_7D * 5,
        },
    }
}

/// Resolve effective limits: a user override wins over the plan default.
pub fn resolve_limits(tier: PlanTier, override_limits: Option<PlanLimits>) -> PlanLimits {
    override_limits.unwrap_or_else(|| default_limits(tier))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_max5x_from_rate_limit_tier() {
        let tier = PlanTier::from_credentials(Some("max"), Some("default_claude_max_5x"));
        assert_eq!(tier, PlanTier::Max5x);
    }

    #[test]
    fn maps_max20x_from_rate_limit_tier() {
        let tier = PlanTier::from_credentials(Some("max"), Some("default_claude_max_20x"));
        assert_eq!(tier, PlanTier::Max20x);
    }

    #[test]
    fn maps_pro() {
        assert_eq!(PlanTier::from_credentials(Some("pro"), None), PlanTier::Pro);
    }

    #[test]
    fn unknown_when_nothing_matches() {
        assert_eq!(PlanTier::from_credentials(None, None), PlanTier::Unknown);
    }

    #[test]
    fn override_wins_over_default() {
        let ov = PlanLimits {
            five_hour_tokens: 1,
            seven_day_tokens: 2,
        };
        assert_eq!(resolve_limits(PlanTier::Max5x, Some(ov)), ov);
        assert_eq!(
            resolve_limits(PlanTier::Pro, None),
            default_limits(PlanTier::Pro)
        );
    }

    #[test]
    fn max5x_is_five_times_pro() {
        let pro = default_limits(PlanTier::Pro);
        let max5 = default_limits(PlanTier::Max5x);
        assert_eq!(max5.five_hour_tokens, pro.five_hour_tokens * 5);
    }
}
