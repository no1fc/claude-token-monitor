//! Tolerant deserialization of a single Claude Code transcript line.
//!
//! Transcript lines vary (user/assistant/system/tool events, summaries, etc.).
//! We only care about assistant lines carrying a `message.usage` object, and we
//! must never crash on lines we don't understand — unknown shapes deserialize to
//! `None` usage and are skipped by the parser.

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct RawLine {
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(default, rename = "sessionId")]
    pub session_id: Option<String>,
    #[serde(default, rename = "requestId")]
    pub request_id: Option<String>,
    #[serde(default)]
    pub message: Option<RawMessage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawMessage {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub usage: Option<RawUsage>,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct RawUsage {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
    #[serde(default)]
    pub cache_read_input_tokens: u64,
}

impl RawUsage {
    /// True when the line records any token activity worth counting.
    pub fn has_activity(&self) -> bool {
        self.input_tokens
            + self.output_tokens
            + self.cache_creation_input_tokens
            + self.cache_read_input_tokens
            > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_assistant_usage_line() {
        let line = r#"{"type":"assistant","timestamp":"2026-06-09T01:00:00.000Z","sessionId":"s1","requestId":"req_1","message":{"id":"msg_1","model":"claude-opus-4-6","usage":{"input_tokens":10,"output_tokens":20,"cache_creation_input_tokens":5,"cache_read_input_tokens":100}}}"#;
        let r: RawLine = serde_json::from_str(line).unwrap();
        let m = r.message.unwrap();
        let u = m.usage.unwrap();
        assert_eq!(u.input_tokens, 10);
        assert_eq!(u.cache_read_input_tokens, 100);
        assert_eq!(m.model.as_deref(), Some("claude-opus-4-6"));
        assert_eq!(r.request_id.as_deref(), Some("req_1"));
    }

    #[test]
    fn tolerates_missing_usage_fields() {
        // Only some usage fields present; others default to 0.
        let line = r#"{"message":{"usage":{"output_tokens":7}}}"#;
        let r: RawLine = serde_json::from_str(line).unwrap();
        let u = r.message.unwrap().usage.unwrap();
        assert_eq!(u.output_tokens, 7);
        assert_eq!(u.input_tokens, 0);
        assert!(u.has_activity());
    }

    #[test]
    fn tolerates_non_usage_lines() {
        // A user line with no usage — should parse, message.usage is None.
        let line = r#"{"type":"user","timestamp":"2026-06-09T01:00:00.000Z","message":{"role":"user","content":"hi"}}"#;
        let r: RawLine = serde_json::from_str(line).unwrap();
        assert!(r.message.unwrap().usage.is_none());
    }

    #[test]
    fn tolerates_unknown_top_level_shapes() {
        let line = r#"{"type":"summary","summary":"...","leafUuid":"x"}"#;
        let r: RawLine = serde_json::from_str(line).unwrap();
        assert!(r.message.is_none());
    }
}
