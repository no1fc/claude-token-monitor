//! Burn-rate and limit-hit ETA projection from the active 5-hour block.

use chrono::{DateTime, Duration, Utc};

use crate::model::BurnStats;

/// Inputs for the burn computation. `now` is injected for deterministic tests.
#[derive(Debug, Clone)]
pub struct BurnInputs {
    pub active_tokens: u64,
    pub active_cost_usd: f64,
    pub block_start: Option<DateTime<Utc>>,
    pub block_end: Option<DateTime<Utc>>,
    pub five_hour_limit: u64,
    pub seven_day_used: u64,
    pub seven_day_limit: u64,
}

/// Minimum elapsed time before we trust a rate (avoids wild early numbers).
const MIN_ELAPSED_SECS: i64 = 30;

pub fn compute(inp: &BurnInputs, now: DateTime<Utc>) -> BurnStats {
    let Some(start) = inp.block_start else {
        return BurnStats::default();
    };
    let elapsed_secs = (now - start).num_seconds();
    if elapsed_secs < MIN_ELAPSED_SECS || inp.active_tokens == 0 {
        return BurnStats::default();
    }

    let elapsed_min = elapsed_secs as f64 / 60.0;
    let tokens_per_min = inp.active_tokens as f64 / elapsed_min;
    let cost_per_hour = inp.active_cost_usd / (elapsed_secs as f64 / 3600.0);

    let five_hour_eta = project_eta(
        inp.active_tokens,
        inp.five_hour_limit,
        tokens_per_min,
        now,
        inp.block_end, // resets at block end; no ETA if we won't hit it first
    );
    let seven_day_eta = project_eta(
        inp.seven_day_used,
        inp.seven_day_limit,
        tokens_per_min,
        now,
        None, // weekly reset unknown in estimation
    );

    BurnStats {
        tokens_per_min,
        cost_per_hour_usd: cost_per_hour,
        five_hour_eta,
        seven_day_eta,
    }
}

/// Project when `used` reaches `limit` at `rate` tokens/min. Returns None if the
/// rate is zero, the limit is already reached, or a `reset_before` time occurs
/// first (the window resets before the limit is hit).
fn project_eta(
    used: u64,
    limit: u64,
    tokens_per_min: f64,
    now: DateTime<Utc>,
    reset_before: Option<DateTime<Utc>>,
) -> Option<DateTime<Utc>> {
    if tokens_per_min <= 0.0 || limit == 0 || used >= limit {
        return None;
    }
    let remaining = (limit - used) as f64;
    let mins_to_limit = remaining / tokens_per_min;
    let eta = now + Duration::seconds((mins_to_limit * 60.0) as i64);
    match reset_before {
        Some(reset) if eta >= reset => None, // window resets first
        _ => Some(eta),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(ts: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(ts).unwrap().with_timezone(&Utc)
    }

    #[test]
    fn zero_when_no_block() {
        let inp = BurnInputs {
            active_tokens: 0,
            active_cost_usd: 0.0,
            block_start: None,
            block_end: None,
            five_hour_limit: 100,
            seven_day_used: 0,
            seven_day_limit: 100,
        };
        let b = compute(&inp, at("2026-06-09T02:00:00Z"));
        assert_eq!(b.tokens_per_min, 0.0);
        assert!(b.five_hour_eta.is_none());
    }

    #[test]
    fn computes_rate_over_elapsed() {
        // 60 tokens over 60 minutes -> 1 token/min.
        let inp = BurnInputs {
            active_tokens: 60,
            active_cost_usd: 6.0,
            block_start: Some(at("2026-06-09T01:00:00Z")),
            block_end: Some(at("2026-06-09T06:00:00Z")),
            five_hour_limit: 1_000_000,
            seven_day_used: 60,
            seven_day_limit: 1_000_000,
        };
        let b = compute(&inp, at("2026-06-09T02:00:00Z"));
        assert!((b.tokens_per_min - 1.0).abs() < 1e-9);
        assert!((b.cost_per_hour_usd - 6.0).abs() < 1e-9);
    }

    #[test]
    fn five_hour_eta_none_when_resets_first() {
        // Low burn, huge limit -> would take far longer than the block; resets first.
        let inp = BurnInputs {
            active_tokens: 60,
            active_cost_usd: 0.0,
            block_start: Some(at("2026-06-09T01:00:00Z")),
            block_end: Some(at("2026-06-09T06:00:00Z")),
            five_hour_limit: 1_000_000,
            seven_day_used: 0,
            seven_day_limit: 1_000_000_000,
        };
        let b = compute(&inp, at("2026-06-09T02:00:00Z"));
        assert!(b.five_hour_eta.is_none());
    }

    #[test]
    fn five_hour_eta_set_when_hit_before_reset() {
        // 100 tokens over 1 min -> 100/min. Limit 6100, used 100 -> 6000 left ->
        // 60 more minutes -> 02:01, before the 06:00 reset.
        let inp = BurnInputs {
            active_tokens: 100,
            active_cost_usd: 0.0,
            block_start: Some(at("2026-06-09T01:00:00Z")),
            block_end: Some(at("2026-06-09T06:00:00Z")),
            five_hour_limit: 6100,
            seven_day_used: 0,
            seven_day_limit: 1_000_000_000,
        };
        let b = compute(&inp, at("2026-06-09T01:01:00Z"));
        let eta = b.five_hour_eta.expect("eta present");
        assert_eq!(eta, at("2026-06-09T02:01:00Z"));
    }
}
