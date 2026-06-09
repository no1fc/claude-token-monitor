//! Combine local JSONL analytics with the (optional) authoritative API response
//! into a single `UsageSnapshot` for the frontend. Pure and deterministic:
//! `now` is injected.

use chrono::{DateTime, Duration, Utc};

use crate::analytics::aggregate::{events_in_window, per_model, total_cost};
use crate::analytics::blocks::active_block;
use crate::analytics::burn_rate::{self, BurnInputs};
use crate::analytics::windows::{self, SEVEN_DAYS};
use crate::api::usage_client::{UsageApiResponse, WindowUtil};
use crate::jsonl::UsageEvent;
use crate::model::{
    BurnStats, CostStats, DataSource, ModelUsage, SessionStats, TokenBreakdown, UsageSnapshot,
    WindowStatus,
};
use crate::plan::{PlanLimits, PlanTier};

/// Merge an estimated window with an API window (when present), preserving the
/// JSONL token count but taking the authoritative percent + reset.
fn merge_window(
    estimated: WindowStatus,
    api: Option<&WindowUtil>,
    now: DateTime<Utc>,
) -> WindowStatus {
    match api {
        Some(u) => windows::from_api(u.utilization, u.resets_at, now, estimated.tokens_used),
        None => estimated,
    }
}

/// Carry one authoritative API window onto a fresh local snapshot: keep the
/// API's percent/reset/`estimated=false`, but adopt the fresh JSONL token count
/// and recompute the countdown against `now`.
fn carry_window(
    api_win: WindowStatus,
    fresh_tokens: Option<u64>,
    now: DateTime<Utc>,
) -> WindowStatus {
    let remaining_secs = match api_win.resets_at {
        Some(r) => (r - now).num_seconds().max(0),
        None => 0,
    };
    WindowStatus {
        remaining_secs,
        tokens_used: fresh_tokens.or(api_win.tokens_used),
        ..api_win
    }
}

/// Overlay the previous snapshot's authoritative API window values onto a
/// freshly-built JSONL-only snapshot. This lets the file-watcher refresh update
/// tokens/cost/burn instantly without regressing the gauges to local estimates
/// between API polls. Token counts stay fresh; percent + reset come from the API.
///
/// No-op when the previous snapshot's windows were themselves estimated (i.e. the
/// API has not produced an authoritative value yet) — honest local estimates win.
pub fn carry_authoritative_windows(
    mut fresh: UsageSnapshot,
    prev: &UsageSnapshot,
    now: DateTime<Utc>,
) -> UsageSnapshot {
    if prev.five_hour.estimated {
        return fresh;
    }
    fresh.five_hour = carry_window(prev.five_hour.clone(), fresh.five_hour.tokens_used, now);
    fresh.seven_day = carry_window(prev.seven_day.clone(), fresh.seven_day.tokens_used, now);
    fresh.seven_day_opus = prev
        .seven_day_opus
        .clone()
        .map(|w| carry_window(w, None, now));
    fresh.seven_day_sonnet = prev
        .seven_day_sonnet
        .clone()
        .map(|w| carry_window(w, None, now));
    fresh.source = DataSource::Hybrid;
    fresh
        .warnings
        .retain(|w| !w.contains("Usage API unavailable"));
    fresh
}

#[allow(clippy::too_many_arguments)]
pub fn build(
    events: &[UsageEvent],
    api: Option<&UsageApiResponse>,
    tier: PlanTier,
    limits: PlanLimits,
    parse_failures: usize,
    context_limit_override: Option<u64>,
    now: DateTime<Utc>,
) -> UsageSnapshot {
    let active = active_block(events, now);

    // --- estimated windows from JSONL ---
    let five_est = windows::five_hour_estimated(events, now, limits.five_hour_tokens);
    let seven_est = windows::seven_day_estimated(events, now, limits.seven_day_tokens);

    // --- merge with API authoritative values ---
    let five_hour = merge_window(five_est, api.and_then(|a| a.five_hour.as_ref()), now);
    let seven_day = merge_window(
        seven_est.clone(),
        api.and_then(|a| a.seven_day.as_ref()),
        now,
    );
    let seven_day_opus = api
        .and_then(|a| a.seven_day_opus.as_ref())
        .map(|u| windows::from_api(u.utilization, u.resets_at, now, None));
    let seven_day_sonnet = api
        .and_then(|a| a.seven_day_sonnet.as_ref())
        .map(|u| windows::from_api(u.utilization, u.resets_at, now, None));

    let source = if api.is_some() {
        DataSource::Hybrid
    } else {
        DataSource::Jsonl
    };

    // --- current session (= active block) ---
    let (session, session_cost) = match &active {
        Some(b) => {
            let block_events: Vec<&UsageEvent> = events
                .iter()
                .filter(|e| e.ts >= b.start && e.ts < b.end && e.ts <= now)
                .collect();
            let cost = total_cost(&block_events);
            (
                SessionStats {
                    session_id: b.session_id.clone(),
                    started_at: Some(b.start),
                    tokens: b.tokens,
                    model: b.model.clone(),
                },
                cost,
            )
        }
        None => (
            SessionStats {
                session_id: None,
                started_at: None,
                tokens: TokenBreakdown::default(),
                model: None,
            },
            0.0,
        ),
    };

    // --- 7-day window aggregates ---
    let since_7d = now - Duration::days(SEVEN_DAYS);
    let week_events = events_in_window(events, since_7d, now);
    let per_model_list: Vec<ModelUsage> = per_model(&week_events);
    let seven_day_cost = total_cost(&week_events);

    let all_refs: Vec<&UsageEvent> = events.iter().collect();
    let total_usd = total_cost(&all_refs);

    // --- burn rate ---
    let burn: BurnStats = match &active {
        Some(b) => burn_rate::compute(
            &BurnInputs {
                active_tokens: b.tokens.total(),
                active_cost_usd: session_cost,
                block_start: Some(b.start),
                block_end: Some(b.end),
                five_hour_limit: limits.five_hour_tokens,
                seven_day_used: seven_est.tokens_used.unwrap_or(0),
                seven_day_limit: limits.seven_day_tokens,
            },
            now,
        ),
        None => BurnStats::default(),
    };

    // --- warnings ---
    let mut warnings = Vec::new();
    if parse_failures > 0 {
        warnings.push(format!(
            "{parse_failures} transcript line(s) could not be parsed"
        ));
    }
    if api.is_none() {
        warnings.push("Usage API unavailable — showing local estimates".to_string());
    }

    UsageSnapshot {
        generated_at: now,
        source,
        plan: tier,
        five_hour,
        seven_day,
        seven_day_opus,
        seven_day_sonnet,
        current_session: session,
        per_model: per_model_list,
        burn,
        context: crate::analytics::context::current_context(events, context_limit_override),
        cost: CostStats {
            session_usd: session_cost,
            five_hour_usd: session_cost,
            seven_day_usd: seven_day_cost,
            total_usd,
        },
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(ts: &str, model: &str, input: u64) -> UsageEvent {
        UsageEvent {
            ts: DateTime::parse_from_rfc3339(ts)
                .unwrap()
                .with_timezone(&Utc),
            session_id: Some("s1".into()),
            model: model.into(),
            tokens: TokenBreakdown {
                input,
                ..Default::default()
            },
        }
    }
    fn at(ts: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(ts)
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn jsonl_only_source_and_estimates() {
        let events = vec![ev("2026-06-09T01:00:00Z", "claude-opus-4-6", 50)];
        let limits = PlanLimits {
            five_hour_tokens: 100,
            seven_day_tokens: 1000,
        };
        let snap = build(
            &events,
            None,
            PlanTier::Max5x,
            limits,
            0,
            None,
            at("2026-06-09T02:00:00Z"),
        );
        assert_eq!(snap.source, DataSource::Jsonl);
        assert!(snap.five_hour.estimated);
        assert_eq!(snap.five_hour.percent_used, 50.0);
        assert_eq!(snap.current_session.tokens.input, 50);
        assert!(snap.warnings.iter().any(|w| w.contains("estimates")));
    }

    #[test]
    fn api_overrides_percent_keeps_tokens() {
        let events = vec![ev("2026-06-09T01:00:00Z", "claude-opus-4-6", 50)];
        let limits = PlanLimits {
            five_hour_tokens: 100,
            seven_day_tokens: 1000,
        };
        let api = UsageApiResponse {
            five_hour: Some(WindowUtil {
                utilization: 80.0,
                resets_at: Some(at("2026-06-09T06:00:00Z")),
            }),
            seven_day: Some(WindowUtil {
                utilization: 10.0,
                resets_at: None,
            }),
            ..Default::default()
        };
        let snap = build(
            &events,
            Some(&api),
            PlanTier::Max5x,
            limits,
            0,
            None,
            at("2026-06-09T02:00:00Z"),
        );
        assert_eq!(snap.source, DataSource::Hybrid);
        assert!(!snap.five_hour.estimated);
        assert_eq!(snap.five_hour.percent_used, 80.0); // API value, not 50
        assert_eq!(snap.five_hour.tokens_used, Some(50)); // JSONL token count kept
    }

    /// Build an authoritative (API) snapshot, then a fresh local one, mimicking
    /// the timer-then-watcher sequence at runtime.
    fn authoritative_then_local() -> (UsageSnapshot, UsageSnapshot) {
        let events = vec![ev("2026-06-09T01:00:00Z", "claude-opus-4-6", 50)];
        let limits = PlanLimits {
            five_hour_tokens: 100, // tiny ceiling so the local estimate is wildly high
            seven_day_tokens: 1000,
        };
        let api = UsageApiResponse {
            five_hour: Some(WindowUtil {
                utilization: 6.0,
                resets_at: Some(at("2026-06-09T06:00:00Z")),
            }),
            seven_day: Some(WindowUtil {
                utilization: 11.0,
                resets_at: Some(at("2026-06-11T00:00:00Z")),
            }),
            seven_day_sonnet: Some(WindowUtil {
                utilization: 1.0,
                resets_at: Some(at("2026-06-10T00:00:00Z")),
            }),
            ..Default::default()
        };
        let now = at("2026-06-09T02:00:00Z");
        let prev = build(&events, Some(&api), PlanTier::Max5x, limits, 0, None, now);
        // A bit later, more tokens, no API (watcher path).
        let later = vec![
            ev("2026-06-09T01:00:00Z", "claude-opus-4-6", 50),
            ev("2026-06-09T02:30:00Z", "claude-opus-4-6", 40),
        ];
        let fresh = build(
            &later,
            None,
            PlanTier::Max5x,
            limits,
            0,
            None,
            at("2026-06-09T03:00:00Z"),
        );
        (prev, fresh)
    }

    #[test]
    fn carry_keeps_authoritative_percent_with_fresh_tokens() {
        let (prev, fresh) = authoritative_then_local();
        assert!(
            fresh.five_hour.estimated,
            "precondition: local is estimated"
        );
        let fresh_tokens = fresh.five_hour.tokens_used;

        let out = carry_authoritative_windows(fresh, &prev, at("2026-06-09T03:00:00Z"));

        assert_eq!(out.five_hour.percent_used, 6.0); // authoritative, not the 100% estimate
        assert!(!out.five_hour.estimated);
        assert_eq!(out.five_hour.tokens_used, fresh_tokens); // fresh JSONL count kept
        assert_eq!(out.seven_day.percent_used, 11.0);
        assert_eq!(out.source, DataSource::Hybrid);
        assert!(
            !out.warnings
                .iter()
                .any(|w| w.contains("Usage API unavailable")),
            "api-unavailable warning must be dropped"
        );
    }

    #[test]
    fn carry_recomputes_remaining_secs_against_now() {
        let (prev, fresh) = authoritative_then_local();
        // reset at 06:00, now 03:00 -> 3h remaining
        let out = carry_authoritative_windows(fresh, &prev, at("2026-06-09T03:00:00Z"));
        assert_eq!(out.five_hour.remaining_secs, 3 * 3600);
    }

    #[test]
    fn carry_propagates_per_model_windows() {
        let (prev, fresh) = authoritative_then_local();
        let out = carry_authoritative_windows(fresh, &prev, at("2026-06-09T03:00:00Z"));
        assert_eq!(
            out.seven_day_sonnet.as_ref().map(|w| w.percent_used),
            Some(1.0)
        );
    }

    #[test]
    fn carry_is_noop_when_prev_estimated() {
        // prev built without API -> estimated windows; nothing authoritative to carry.
        let events = vec![ev("2026-06-09T01:00:00Z", "claude-opus-4-6", 50)];
        let limits = PlanLimits {
            five_hour_tokens: 100,
            seven_day_tokens: 1000,
        };
        let now = at("2026-06-09T02:00:00Z");
        let prev = build(&events, None, PlanTier::Max5x, limits, 0, None, now);
        let fresh = build(&events, None, PlanTier::Max5x, limits, 0, None, now);
        let out = carry_authoritative_windows(fresh, &prev, now);
        assert!(out.five_hour.estimated);
        assert_eq!(out.source, DataSource::Jsonl);
        assert!(out.warnings.iter().any(|w| w.contains("estimates")));
    }

    #[test]
    fn parse_failures_surface_as_warning() {
        let events = vec![ev("2026-06-09T01:00:00Z", "claude-opus-4-6", 50)];
        let limits = PlanLimits {
            five_hour_tokens: 100,
            seven_day_tokens: 1000,
        };
        let snap = build(
            &events,
            None,
            PlanTier::Pro,
            limits,
            3,
            None,
            at("2026-06-09T02:00:00Z"),
        );
        assert!(snap.warnings.iter().any(|w| w.contains("3 transcript")));
    }
}
