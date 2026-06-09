//! Build `WindowStatus` values for the 5-hour and 7-day windows, either from
//! local JSONL estimation or from the authoritative usage API.

use chrono::{DateTime, Duration, Utc};

use super::aggregate::{events_in_window, sum_tokens};
use super::blocks::active_block;
use crate::jsonl::UsageEvent;
use crate::model::WindowStatus;

pub const SEVEN_DAYS: i64 = 7;

fn pct(used: u64, limit: u64) -> f64 {
    if limit == 0 {
        return 0.0;
    }
    ((used as f64 / limit as f64) * 100.0).clamp(0.0, 100.0)
}

fn remaining_secs(resets_at: Option<DateTime<Utc>>, now: DateTime<Utc>) -> i64 {
    match resets_at {
        Some(r) => (r - now).num_seconds().max(0),
        None => 0,
    }
}

/// 5-hour window estimated from the active block.
pub fn five_hour_estimated(
    events: &[UsageEvent],
    now: DateTime<Utc>,
    limit: u64,
) -> WindowStatus {
    match active_block(events, now) {
        Some(b) => {
            let used = b.tokens.total();
            let resets_at = Some(b.end);
            let percent = pct(used, limit);
            WindowStatus {
                percent_used: percent,
                remaining_percent: 100.0 - percent,
                resets_at,
                remaining_secs: remaining_secs(resets_at, now),
                estimated: true,
                tokens_used: Some(used),
                token_limit: Some(limit),
            }
        }
        None => WindowStatus {
            percent_used: 0.0,
            remaining_percent: 100.0,
            resets_at: None,
            remaining_secs: 0,
            estimated: true,
            tokens_used: Some(0),
            token_limit: Some(limit),
        },
    }
}

/// 7-day window estimated from a rolling sum of the last 7 days.
/// `resets_at` is approximated as (oldest counted event + 7d) — when that usage
/// begins to roll off. Precise resets come only from the API.
pub fn seven_day_estimated(
    events: &[UsageEvent],
    now: DateTime<Utc>,
    limit: u64,
) -> WindowStatus {
    let since = now - Duration::days(SEVEN_DAYS);
    let win = events_in_window(events, since, now);
    let used = sum_tokens(&win).total();
    let percent = pct(used, limit);
    let resets_at = win.iter().map(|e| e.ts).min().map(|t| t + Duration::days(SEVEN_DAYS));
    WindowStatus {
        percent_used: percent,
        remaining_percent: 100.0 - percent,
        resets_at,
        remaining_secs: remaining_secs(resets_at, now),
        estimated: true,
        tokens_used: Some(used),
        token_limit: Some(limit),
    }
}

/// Build a window status from the authoritative API: utilization (0–100) and a
/// reset timestamp. Token counts may be merged in separately by the caller.
pub fn from_api(
    utilization: f64,
    resets_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
    tokens_used: Option<u64>,
) -> WindowStatus {
    let percent = utilization.clamp(0.0, 100.0);
    WindowStatus {
        percent_used: percent,
        remaining_percent: 100.0 - percent,
        resets_at,
        remaining_secs: remaining_secs(resets_at, now),
        estimated: false,
        tokens_used,
        token_limit: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TokenBreakdown;

    fn ev(ts: &str, input: u64) -> UsageEvent {
        UsageEvent {
            ts: DateTime::parse_from_rfc3339(ts).unwrap().with_timezone(&Utc),
            session_id: Some("s1".into()),
            model: "claude-opus-4-6".into(),
            tokens: TokenBreakdown { input, ..Default::default() },
        }
    }
    fn at(ts: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(ts).unwrap().with_timezone(&Utc)
    }

    #[test]
    fn five_hour_percent_and_reset() {
        let events = vec![ev("2026-06-09T01:00:00Z", 50)];
        let now = at("2026-06-09T02:00:00Z");
        let w = five_hour_estimated(&events, now, 100);
        assert_eq!(w.percent_used, 50.0);
        assert_eq!(w.remaining_percent, 50.0);
        assert_eq!(w.resets_at, Some(at("2026-06-09T06:00:00Z")));
        assert_eq!(w.remaining_secs, 4 * 3600);
        assert!(w.estimated);
    }

    #[test]
    fn five_hour_idle_is_zero() {
        let events = vec![ev("2026-06-09T01:00:00Z", 50)];
        let now = at("2026-06-09T12:00:00Z");
        let w = five_hour_estimated(&events, now, 100);
        assert_eq!(w.percent_used, 0.0);
        assert_eq!(w.tokens_used, Some(0));
    }

    #[test]
    fn percent_clamps_at_100() {
        let events = vec![ev("2026-06-09T01:00:00Z", 500)];
        let now = at("2026-06-09T02:00:00Z");
        let w = five_hour_estimated(&events, now, 100);
        assert_eq!(w.percent_used, 100.0);
    }

    #[test]
    fn seven_day_sums_within_window() {
        let events = vec![
            ev("2026-06-01T00:00:00Z", 10), // exactly 8 days before -> out
            ev("2026-06-03T00:00:00Z", 30), // within 7 days -> in
            ev("2026-06-08T00:00:00Z", 20),
        ];
        let now = at("2026-06-09T00:00:00Z");
        let w = seven_day_estimated(&events, now, 1000);
        assert_eq!(w.tokens_used, Some(50));
        assert_eq!(w.percent_used, 5.0);
    }

    #[test]
    fn from_api_uses_utilization() {
        let now = at("2026-06-09T00:00:00Z");
        let reset = at("2026-06-09T05:00:00Z");
        let w = from_api(33.0, Some(reset), now, None);
        assert_eq!(w.percent_used, 33.0);
        assert_eq!(w.remaining_percent, 67.0);
        assert!(!w.estimated);
        assert_eq!(w.remaining_secs, 5 * 3600);
    }
}
