//! Local JSONL transcript ingestion: scanning, tolerant parsing, dedup.

pub mod parser;
pub mod records;
pub mod scanner;

pub use parser::{parse_str, ParseOutcome, UsageEvent};
pub use scanner::scan_dir;
