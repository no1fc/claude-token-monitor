//! Context-window usage for the *currently active* Claude Code session.
//!
//! Distinct from the 5h/weekly quota: this is how full the model's context
//! window is right now. The latest message's prompt size
//! (`input + cache_creation + cache_read`) is the current context occupancy;
//! compare it to the model's context limit.

use crate::jsonl::UsageEvent;
use crate::model::ContextStatus;

/// Standard context window; current Opus/Sonnet 4.x expose 1M.
pub const DEFAULT_CONTEXT_LIMIT: u64 = 200_000;
pub const LARGE_CONTEXT_LIMIT: u64 = 1_000_000;

/// Best-guess context limit from the model id. User override wins (see caller).
pub fn context_limit_for_model(model: &str) -> u64 {
    let m = model.to_ascii_lowercase();
    if m.contains("opus") || m.contains("sonnet") {
        LARGE_CONTEXT_LIMIT
    } else {
        DEFAULT_CONTEXT_LIMIT
    }
}

/// Compute context status from the most recent event (the active session's last
/// message). Returns None when there are no events.
pub fn current_context(
    events: &[UsageEvent],
    override_limit: Option<u64>,
) -> Option<ContextStatus> {
    let latest = events.iter().max_by_key(|e| e.ts)?;
    let used = latest.tokens.input + latest.tokens.cache_creation + latest.tokens.cache_read;
    let limit = override_limit
        .filter(|l| *l > 0)
        .unwrap_or_else(|| context_limit_for_model(&latest.model));
    let percent_used = if limit == 0 {
        0.0
    } else {
        ((used as f64 / limit as f64) * 100.0).clamp(0.0, 100.0)
    };
    Some(ContextStatus {
        used,
        limit,
        percent_used,
        remaining: limit.saturating_sub(used),
        model: latest.model.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TokenBreakdown;
    use chrono::{DateTime, Utc};

    fn ev(ts: &str, model: &str, input: u64, cache_read: u64) -> UsageEvent {
        UsageEvent {
            ts: DateTime::parse_from_rfc3339(ts)
                .unwrap()
                .with_timezone(&Utc),
            session_id: Some("s1".into()),
            model: model.into(),
            tokens: TokenBreakdown {
                input,
                cache_read,
                ..Default::default()
            },
        }
    }

    #[test]
    fn none_when_no_events() {
        assert!(current_context(&[], None).is_none());
    }

    #[test]
    fn uses_latest_event_and_model_limit() {
        let events = vec![
            ev("2026-06-09T01:00:00Z", "claude-opus-4-8", 10, 10),
            ev("2026-06-09T02:00:00Z", "claude-opus-4-8", 100_000, 400_000),
        ];
        let c = current_context(&events, None).unwrap();
        assert_eq!(c.used, 500_000);
        assert_eq!(c.limit, LARGE_CONTEXT_LIMIT); // opus -> 1M
        assert_eq!(c.percent_used, 50.0);
        assert_eq!(c.remaining, 500_000);
    }

    #[test]
    fn haiku_defaults_to_200k() {
        let events = vec![ev("2026-06-09T01:00:00Z", "claude-haiku-4-5", 50_000, 0)];
        let c = current_context(&events, None).unwrap();
        assert_eq!(c.limit, DEFAULT_CONTEXT_LIMIT);
        assert_eq!(c.percent_used, 25.0);
    }

    #[test]
    fn override_limit_wins() {
        let events = vec![ev("2026-06-09T01:00:00Z", "claude-opus-4-8", 100_000, 0)];
        let c = current_context(&events, Some(200_000)).unwrap();
        assert_eq!(c.limit, 200_000);
        assert_eq!(c.percent_used, 50.0);
    }

    #[test]
    fn percent_clamps_at_100() {
        let events = vec![ev("2026-06-09T01:00:00Z", "claude-haiku-4-5", 999_999, 0)];
        let c = current_context(&events, None).unwrap();
        assert_eq!(c.percent_used, 100.0);
        assert_eq!(c.remaining, 0);
    }
}
