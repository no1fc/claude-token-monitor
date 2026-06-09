//! Estimated cost from token counts.
//!
//! Rates are USD per million tokens, by model family. APPROX — update when
//! Anthropic changes pricing. Cache-write is billed at 1.25x base input and
//! cache-read at 0.1x base input (the standard 5-minute-cache multipliers).

use crate::model::TokenBreakdown;

const PER_MILLION: f64 = 1_000_000.0;
const CACHE_WRITE_MULT: f64 = 1.25;
const CACHE_READ_MULT: f64 = 0.10;

#[derive(Debug, Clone, Copy)]
pub struct Pricing {
    pub input: f64,
    pub output: f64,
}

/// Resolve per-family pricing from a model id (e.g. `claude-opus-4-6`).
pub fn model_pricing(model: &str) -> Pricing {
    let m = model.to_ascii_lowercase();
    if m.contains("opus") {
        Pricing {
            input: 15.0,
            output: 75.0,
        }
    } else if m.contains("sonnet") {
        Pricing {
            input: 3.0,
            output: 15.0,
        }
    } else if m.contains("haiku") {
        Pricing {
            input: 1.0,
            output: 5.0,
        }
    } else {
        // Conservative default for unknown models: Sonnet-class rates.
        Pricing {
            input: 3.0,
            output: 15.0,
        }
    }
}

/// Estimated USD cost for a token breakdown under a given model.
pub fn cost(model: &str, tokens: &TokenBreakdown) -> f64 {
    let p = model_pricing(model);
    let input = tokens.input as f64 * p.input;
    let output = tokens.output as f64 * p.output;
    let cache_write = tokens.cache_creation as f64 * p.input * CACHE_WRITE_MULT;
    let cache_read = tokens.cache_read as f64 * p.input * CACHE_READ_MULT;
    (input + output + cache_write + cache_read) / PER_MILLION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opus_rates_applied() {
        let t = TokenBreakdown {
            input: 1_000_000,
            output: 0,
            cache_creation: 0,
            cache_read: 0,
        };
        assert!((cost("claude-opus-4-6", &t) - 15.0).abs() < 1e-9);
    }

    #[test]
    fn output_more_expensive_than_input() {
        let inp = TokenBreakdown {
            input: 1_000_000,
            ..Default::default()
        };
        let out = TokenBreakdown {
            output: 1_000_000,
            ..Default::default()
        };
        assert!(cost("claude-sonnet-4-6", &out) > cost("claude-sonnet-4-6", &inp));
    }

    #[test]
    fn cache_write_and_read_multipliers() {
        let write = TokenBreakdown {
            cache_creation: 1_000_000,
            ..Default::default()
        };
        let read = TokenBreakdown {
            cache_read: 1_000_000,
            ..Default::default()
        };
        // sonnet input = 3.0 -> write 3.75, read 0.30
        assert!((cost("claude-sonnet-4-6", &write) - 3.75).abs() < 1e-9);
        assert!((cost("claude-sonnet-4-6", &read) - 0.30).abs() < 1e-9);
    }

    #[test]
    fn unknown_model_defaults_to_sonnet() {
        let t = TokenBreakdown {
            input: 1_000_000,
            ..Default::default()
        };
        assert!((cost("some-future-model", &t) - 3.0).abs() < 1e-9);
    }
}
