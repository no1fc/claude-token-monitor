//! 5-hour rolling "block" detection, matching ccusage's algorithm.
//!
//! Claude Code bills usage in 5-hour windows that begin with your first message
//! and reset 5 hours later. ccusage reconstructs these from transcript
//! timestamps:
//!   * a block starts at the first event's time floored to the hour;
//!   * it lasts exactly 5 hours;
//!   * an inactivity gap of >= 5 hours closes the block and the next event opens
//!     a new one (again floored to the hour).
//!
//! All math is in UTC and `now` is injected so the logic is fully deterministic
//! and unit-testable.

use chrono::{DateTime, Duration, Timelike, Utc};

use crate::jsonl::UsageEvent;
use crate::model::TokenBreakdown;

pub const BLOCK_HOURS: i64 = 5;

#[derive(Debug, Clone)]
pub struct Block {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub last_event: DateTime<Utc>,
    pub tokens: TokenBreakdown,
    pub event_count: usize,
    pub session_id: Option<String>,
    pub model: Option<String>,
}

impl Block {
    pub fn contains(&self, now: DateTime<Utc>) -> bool {
        now >= self.start && now < self.end
    }
}

fn floor_to_hour(dt: DateTime<Utc>) -> DateTime<Utc> {
    dt.with_minute(0)
        .and_then(|d| d.with_second(0))
        .and_then(|d| d.with_nanosecond(0))
        .unwrap_or(dt)
}

/// Reconstruct all 5-hour blocks from usage events (any order).
pub fn build_blocks(events: &[UsageEvent]) -> Vec<Block> {
    if events.is_empty() {
        return Vec::new();
    }
    let mut sorted: Vec<&UsageEvent> = events.iter().collect();
    sorted.sort_by_key(|e| e.ts);

    let gap = Duration::hours(BLOCK_HOURS);
    let mut blocks: Vec<Block> = Vec::new();
    let mut current: Option<Block> = None;

    for ev in sorted {
        match current.as_mut() {
            Some(b) if ev.ts < b.start + gap && ev.ts - b.last_event < gap => {
                // belongs to the current block
                b.tokens = b.tokens.plus(ev.tokens);
                b.event_count += 1;
                b.last_event = ev.ts;
                b.model = Some(ev.model.clone());
            }
            _ => {
                if let Some(b) = current.take() {
                    blocks.push(b);
                }
                let start = floor_to_hour(ev.ts);
                current = Some(Block {
                    start,
                    end: start + gap,
                    last_event: ev.ts,
                    tokens: ev.tokens,
                    event_count: 1,
                    session_id: ev.session_id.clone(),
                    model: Some(ev.model.clone()),
                });
            }
        }
    }
    if let Some(b) = current.take() {
        blocks.push(b);
    }
    blocks
}

/// The block containing `now`, if any (None when idle / between blocks).
pub fn active_block(events: &[UsageEvent], now: DateTime<Utc>) -> Option<Block> {
    build_blocks(events).into_iter().find(|b| b.contains(now))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(ts: &str, tokens: u64) -> UsageEvent {
        UsageEvent {
            ts: DateTime::parse_from_rfc3339(ts).unwrap().with_timezone(&Utc),
            session_id: Some("s1".into()),
            model: "claude-opus-4-6".into(),
            tokens: TokenBreakdown { input: tokens, ..Default::default() },
        }
    }

    fn at(ts: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(ts).unwrap().with_timezone(&Utc)
    }

    #[test]
    fn single_block_starts_floored_to_hour() {
        let events = vec![ev("2026-06-09T01:23:45.000Z", 100)];
        let blocks = build_blocks(&events);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].start, at("2026-06-09T01:00:00.000Z"));
        assert_eq!(blocks[0].end, at("2026-06-09T06:00:00.000Z"));
        assert_eq!(blocks[0].tokens.input, 100);
    }

    #[test]
    fn events_within_5h_share_a_block() {
        let events = vec![
            ev("2026-06-09T01:10:00.000Z", 10),
            ev("2026-06-09T03:00:00.000Z", 20),
            ev("2026-06-09T05:30:00.000Z", 5), // < block start(01:00)+5h = 06:00
        ];
        let blocks = build_blocks(&events);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].tokens.input, 35);
        assert_eq!(blocks[0].event_count, 3);
    }

    #[test]
    fn event_past_block_window_opens_new_block() {
        // Second event is after 01:00+5h = 06:00 but gap < 5h, so a new block.
        let events = vec![
            ev("2026-06-09T01:10:00.000Z", 10),
            ev("2026-06-09T06:30:00.000Z", 20),
        ];
        let blocks = build_blocks(&events);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[1].start, at("2026-06-09T06:00:00.000Z"));
    }

    #[test]
    fn large_gap_closes_block() {
        let events = vec![
            ev("2026-06-09T01:10:00.000Z", 10),
            ev("2026-06-09T20:00:00.000Z", 20), // >5h gap
        ];
        let blocks = build_blocks(&events);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[1].start, at("2026-06-09T20:00:00.000Z"));
    }

    #[test]
    fn active_block_found_when_now_inside() {
        let events = vec![ev("2026-06-09T01:10:00.000Z", 10)];
        let now = at("2026-06-09T03:00:00.000Z");
        let b = active_block(&events, now).unwrap();
        assert_eq!(b.tokens.input, 10);
    }

    #[test]
    fn no_active_block_when_idle() {
        let events = vec![ev("2026-06-09T01:10:00.000Z", 10)];
        let now = at("2026-06-09T09:00:00.000Z"); // past 06:00 end
        assert!(active_block(&events, now).is_none());
    }

    #[test]
    fn unsorted_input_is_handled() {
        let events = vec![
            ev("2026-06-09T03:00:00.000Z", 20),
            ev("2026-06-09T01:10:00.000Z", 10),
        ];
        let blocks = build_blocks(&events);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].start, at("2026-06-09T01:00:00.000Z"));
    }
}
