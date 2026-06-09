//! Shared data contract between the Rust backend and the TypeScript frontend.
//! Serialized as camelCase JSON; mirror these shapes in `src/types.ts`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::plan::PlanTier;

/// Where the displayed numbers came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DataSource {
    /// Authoritative percentages/resets from the unofficial usage API.
    Api,
    /// Estimated locally from JSONL transcripts.
    Jsonl,
    /// API percentages + JSONL tokens/cost/burn merged.
    Hybrid,
}

/// Raw token counts. All fields are additive so breakdowns can be summed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenBreakdown {
    pub input: u64,
    pub output: u64,
    pub cache_creation: u64,
    pub cache_read: u64,
}

impl TokenBreakdown {
    /// Sum of every token category.
    pub fn total(&self) -> u64 {
        self.input + self.output + self.cache_creation + self.cache_read
    }

    /// Accumulate another breakdown into this one (immutable-friendly helper
    /// returns a new value rather than mutating callers' originals).
    pub fn plus(self, other: TokenBreakdown) -> TokenBreakdown {
        TokenBreakdown {
            input: self.input + other.input,
            output: self.output + other.output,
            cache_creation: self.cache_creation + other.cache_creation,
            cache_read: self.cache_read + other.cache_read,
        }
    }
}

// `total` is computed; expose it to the frontend as a field.
impl Serialize for TokenBreakdownWire {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut st = s.serialize_struct("TokenBreakdown", 5)?;
        st.serialize_field("input", &self.0.input)?;
        st.serialize_field("output", &self.0.output)?;
        st.serialize_field("cacheCreation", &self.0.cache_creation)?;
        st.serialize_field("cacheRead", &self.0.cache_read)?;
        st.serialize_field("total", &self.0.total())?;
        st.end()
    }
}

/// Wrapper used only where we want `total` included in the wire format.
pub struct TokenBreakdownWire(pub TokenBreakdown);

/// Status of a single rate-limit window (5-hour or 7-day).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowStatus {
    /// 0.0 – 100.0
    pub percent_used: f64,
    /// 0.0 – 100.0
    pub remaining_percent: f64,
    /// When the window resets (UTC). None if unknown (e.g. idle, JSONL-only with no activity).
    pub resets_at: Option<DateTime<Utc>>,
    /// Seconds until reset; 0 if unknown.
    pub remaining_secs: i64,
    /// True when derived from JSONL estimation rather than the API.
    pub estimated: bool,
    /// Tokens counted in this window (JSONL). None when only the API percent is known.
    pub tokens_used: Option<u64>,
    /// Plan token ceiling for this window (approximate). None when unknown.
    pub token_limit: Option<u64>,
}

/// Stats for the currently-active session/block.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStats {
    pub session_id: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    #[serde(serialize_with = "ser_tokens")]
    pub tokens: TokenBreakdown,
    pub model: Option<String>,
}

/// Per-model usage with computed cost.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelUsage {
    pub model: String,
    #[serde(serialize_with = "ser_tokens")]
    pub tokens: TokenBreakdown,
    pub cost_usd: f64,
}

/// Burn-rate projection.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BurnStats {
    pub tokens_per_min: f64,
    pub cost_per_hour_usd: f64,
    /// Projected time the 5-hour limit is hit at the current rate.
    pub five_hour_eta: Option<DateTime<Utc>>,
    /// Projected time the 7-day limit is hit at the current rate.
    pub seven_day_eta: Option<DateTime<Utc>>,
}

/// Context-window occupancy for the currently active session.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextStatus {
    /// Tokens currently occupying the model's context window.
    pub used: u64,
    /// The model's context window size.
    pub limit: u64,
    /// 0.0 – 100.0
    pub percent_used: f64,
    /// Tokens of context still available.
    pub remaining: u64,
    /// Model of the active session.
    pub model: String,
}

/// Cost breakdown in USD.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CostStats {
    pub session_usd: f64,
    pub five_hour_usd: f64,
    pub seven_day_usd: f64,
    pub total_usd: f64,
}

/// The full snapshot pushed to the frontend.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSnapshot {
    pub generated_at: DateTime<Utc>,
    pub source: DataSource,
    pub plan: PlanTier,
    pub five_hour: WindowStatus,
    pub seven_day: WindowStatus,
    pub seven_day_opus: Option<WindowStatus>,
    pub seven_day_sonnet: Option<WindowStatus>,
    pub current_session: SessionStats,
    pub per_model: Vec<ModelUsage>,
    pub burn: BurnStats,
    pub cost: CostStats,
    /// Context-window occupancy of the active session (None if no activity).
    pub context: Option<ContextStatus>,
    /// Non-fatal warnings (e.g. parse failures, API unavailable).
    pub warnings: Vec<String>,
}

/// serde helper: serialize a `TokenBreakdown` with the computed `total` field.
fn ser_tokens<S: serde::Serializer>(t: &TokenBreakdown, s: S) -> Result<S::Ok, S::Error> {
    TokenBreakdownWire(*t).serialize(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_sums_all_categories() {
        let t = TokenBreakdown {
            input: 10,
            output: 20,
            cache_creation: 5,
            cache_read: 100,
        };
        assert_eq!(t.total(), 135);
    }

    #[test]
    fn plus_is_field_wise_and_nonmutating() {
        let a = TokenBreakdown {
            input: 1,
            output: 2,
            cache_creation: 3,
            cache_read: 4,
        };
        let b = TokenBreakdown {
            input: 10,
            output: 20,
            cache_creation: 30,
            cache_read: 40,
        };
        let c = a.plus(b);
        assert_eq!(
            c,
            TokenBreakdown {
                input: 11,
                output: 22,
                cache_creation: 33,
                cache_read: 44
            }
        );
        // originals untouched
        assert_eq!(a.input, 1);
        assert_eq!(b.input, 10);
    }

    #[test]
    fn token_breakdown_wire_includes_total() {
        let t = TokenBreakdown {
            input: 1,
            output: 2,
            cache_creation: 3,
            cache_read: 4,
        };
        let json = serde_json::to_value(TokenBreakdownWire(t)).unwrap();
        assert_eq!(json["total"], 10);
        assert_eq!(json["cacheCreation"], 3);
    }
}
