//! Turn raw transcript lines into clean, de-duplicated `UsageEvent`s.

use std::collections::HashSet;

use chrono::{DateTime, Utc};

use super::records::RawLine;
use crate::model::TokenBreakdown;

/// A single billable token event extracted from the transcript.
#[derive(Debug, Clone)]
pub struct UsageEvent {
    pub ts: DateTime<Utc>,
    pub session_id: Option<String>,
    pub model: String,
    pub tokens: TokenBreakdown,
}

/// Result of parsing one or more files.
#[derive(Debug, Default)]
pub struct ParseOutcome {
    pub events: Vec<UsageEvent>,
    /// Lines that looked like data but failed to parse (malformed JSON, bad
    /// timestamp). Surfaced as a non-fatal warning, never fatal.
    pub parse_failures: usize,
}

const UNKNOWN_MODEL: &str = "unknown";

/// Build the dedup key the way ccusage does: `requestId` + `message.id`.
/// Returns `None` when we cannot form a stable key (then the event is always
/// kept — better to slightly over-count than silently drop real usage).
fn dedup_key(line: &RawLine) -> Option<String> {
    let req = line.request_id.as_deref()?;
    let msg_id = line.message.as_ref().and_then(|m| m.id.as_deref())?;
    Some(format!("{req}:{msg_id}"))
}

fn parse_ts(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

/// Parse JSONL `content`, appending events into `out` and recording dedup state
/// in `seen` (shared across files in a scan).
pub fn parse_into(content: &str, seen: &mut HashSet<String>, out: &mut ParseOutcome) {
    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let parsed: RawLine = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => {
                out.parse_failures += 1;
                continue;
            }
        };

        let Some(message) = parsed.message.as_ref() else {
            continue; // not a message line
        };
        let Some(usage) = message.usage else {
            continue; // no token data
        };
        if !usage.has_activity() {
            continue;
        }

        // Timestamp is required for windowing.
        let Some(ts) = parsed.timestamp.as_deref().and_then(parse_ts) else {
            out.parse_failures += 1;
            continue;
        };

        // Dedup when we have a stable key.
        if let Some(key) = dedup_key(&parsed) {
            if !seen.insert(key) {
                continue; // already counted
            }
        }

        out.events.push(UsageEvent {
            ts,
            session_id: parsed.session_id.clone(),
            model: message
                .model
                .clone()
                .unwrap_or_else(|| UNKNOWN_MODEL.to_string()),
            tokens: TokenBreakdown {
                input: usage.input_tokens,
                output: usage.output_tokens,
                cache_creation: usage.cache_creation_input_tokens,
                cache_read: usage.cache_read_input_tokens,
            },
        });
    }
}

/// Convenience: parse a single content blob from scratch.
pub fn parse_str(content: &str) -> ParseOutcome {
    let mut seen = HashSet::new();
    let mut out = ParseOutcome::default();
    parse_into(content, &mut seen, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(ts: &str, req: &str, msg: &str, model: &str, inp: u64, out: u64) -> String {
        format!(
            r#"{{"type":"assistant","timestamp":"{ts}","sessionId":"s1","requestId":"{req}","message":{{"id":"{msg}","model":"{model}","usage":{{"input_tokens":{inp},"output_tokens":{out},"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}}}}}"#
        )
    }

    #[test]
    fn extracts_events_with_tokens() {
        let content = line("2026-06-09T01:00:00.000Z", "r1", "m1", "claude-opus-4-6", 10, 20);
        let outcome = parse_str(&content);
        assert_eq!(outcome.events.len(), 1);
        assert_eq!(outcome.events[0].tokens.input, 10);
        assert_eq!(outcome.events[0].model, "claude-opus-4-6");
        assert_eq!(outcome.parse_failures, 0);
    }

    #[test]
    fn dedupes_by_request_and_message_id() {
        let l = line("2026-06-09T01:00:00.000Z", "r1", "m1", "claude-opus-4-6", 10, 20);
        let content = format!("{l}\n{l}"); // exact duplicate line
        let outcome = parse_str(&content);
        assert_eq!(outcome.events.len(), 1, "duplicate (requestId, id) collapsed");
    }

    #[test]
    fn keeps_distinct_messages() {
        let a = line("2026-06-09T01:00:00.000Z", "r1", "m1", "claude-opus-4-6", 10, 20);
        let b = line("2026-06-09T01:01:00.000Z", "r2", "m2", "claude-opus-4-6", 1, 2);
        let outcome = parse_str(&format!("{a}\n{b}"));
        assert_eq!(outcome.events.len(), 2);
    }

    #[test]
    fn skips_malformed_and_counts_failures() {
        let good = line("2026-06-09T01:00:00.000Z", "r1", "m1", "claude-opus-4-6", 10, 20);
        let content = format!("{good}\n{{not valid json\n\n");
        let outcome = parse_str(&content);
        assert_eq!(outcome.events.len(), 1);
        assert_eq!(outcome.parse_failures, 1);
    }

    #[test]
    fn skips_lines_without_usage() {
        let user = r#"{"type":"user","timestamp":"2026-06-09T01:00:00.000Z","message":{"role":"user","content":"hi"}}"#;
        let outcome = parse_str(user);
        assert_eq!(outcome.events.len(), 0);
        assert_eq!(outcome.parse_failures, 0);
    }

    #[test]
    fn bad_timestamp_is_a_failure() {
        let bad = r#"{"type":"assistant","timestamp":"not-a-date","requestId":"r1","message":{"id":"m1","model":"x","usage":{"input_tokens":5}}}"#;
        let outcome = parse_str(bad);
        assert_eq!(outcome.events.len(), 0);
        assert_eq!(outcome.parse_failures, 1);
    }
}
