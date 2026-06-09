//! Aggregations over usage events: windowed token sums, per-model breakdown,
//! and cost rollups.

use std::collections::HashMap;

use chrono::{DateTime, Utc};

use super::pricing;
use crate::jsonl::UsageEvent;
use crate::model::{ModelUsage, TokenBreakdown};

/// Events with `since <= ts <= now`.
pub fn events_in_window<'a>(
    events: &'a [UsageEvent],
    since: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Vec<&'a UsageEvent> {
    events
        .iter()
        .filter(|e| e.ts >= since && e.ts <= now)
        .collect()
}

/// Sum every token category across the given events.
pub fn sum_tokens(events: &[&UsageEvent]) -> TokenBreakdown {
    events
        .iter()
        .fold(TokenBreakdown::default(), |acc, e| acc.plus(e.tokens))
}

/// Total estimated USD cost across events (cost is per-event by its model).
pub fn total_cost(events: &[&UsageEvent]) -> f64 {
    events
        .iter()
        .map(|e| pricing::cost(&e.model, &e.tokens))
        .sum()
}

/// Per-model usage breakdown, sorted by total tokens descending.
pub fn per_model(events: &[&UsageEvent]) -> Vec<ModelUsage> {
    let mut by_model: HashMap<String, TokenBreakdown> = HashMap::new();
    for e in events {
        let entry = by_model.entry(e.model.clone()).or_default();
        *entry = entry.plus(e.tokens);
    }
    let mut out: Vec<ModelUsage> = by_model
        .into_iter()
        .map(|(model, tokens)| ModelUsage {
            cost_usd: pricing::cost(&model, &tokens),
            model,
            tokens,
        })
        .collect();
    out.sort_by(|a, b| b.tokens.total().cmp(&a.tokens.total()));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(ts: &str, model: &str, input: u64) -> UsageEvent {
        UsageEvent {
            ts: DateTime::parse_from_rfc3339(ts).unwrap().with_timezone(&Utc),
            session_id: Some("s1".into()),
            model: model.into(),
            tokens: TokenBreakdown { input, ..Default::default() },
        }
    }
    fn at(ts: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(ts).unwrap().with_timezone(&Utc)
    }

    #[test]
    fn window_filters_by_time_inclusive() {
        let events = vec![
            ev("2026-06-01T00:00:00Z", "claude-opus-4-6", 10),
            ev("2026-06-08T00:00:00Z", "claude-opus-4-6", 20),
        ];
        let now = at("2026-06-09T00:00:00Z");
        let since = now - chrono::Duration::days(7);
        let win = events_in_window(&events, since, now);
        assert_eq!(win.len(), 1, "only the within-7-days event");
        assert_eq!(sum_tokens(&win).input, 20);
    }

    #[test]
    fn per_model_groups_and_sorts() {
        let events = vec![
            ev("2026-06-09T01:00:00Z", "claude-haiku-4-5", 5),
            ev("2026-06-09T01:01:00Z", "claude-opus-4-6", 100),
            ev("2026-06-09T01:02:00Z", "claude-opus-4-6", 50),
        ];
        let refs: Vec<&UsageEvent> = events.iter().collect();
        let pm = per_model(&refs);
        assert_eq!(pm.len(), 2);
        assert_eq!(pm[0].model, "claude-opus-4-6"); // largest first
        assert_eq!(pm[0].tokens.input, 150);
    }

    #[test]
    fn total_cost_sums_per_event() {
        let events = vec![ev("2026-06-09T01:00:00Z", "claude-opus-4-6", 1_000_000)];
        let refs: Vec<&UsageEvent> = events.iter().collect();
        assert!((total_cost(&refs) - 15.0).abs() < 1e-9);
    }
}
