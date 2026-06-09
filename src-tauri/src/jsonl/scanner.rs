//! Walk `~/.claude/projects/**/*.jsonl` and parse every transcript into a
//! single de-duplicated set of usage events.

use std::collections::HashSet;
use std::path::Path;

use walkdir::WalkDir;

use super::parser::{parse_into, ParseOutcome};

/// Scan a projects directory and return all usage events.
/// Missing directory yields an empty outcome (not an error) — the caller decides
/// whether "no data" is meaningful.
pub fn scan_dir(projects_dir: &Path) -> ParseOutcome {
    let mut seen: HashSet<String> = HashSet::new();
    let mut out = ParseOutcome::default();

    if !projects_dir.exists() {
        return out;
    }

    for entry in WalkDir::new(projects_dir)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }
        match std::fs::read_to_string(path) {
            Ok(content) => parse_into(&content, &mut seen, &mut out),
            Err(_) => out.parse_failures += 1,
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_when_dir_missing() {
        let out = scan_dir(Path::new("/definitely/not/here/xyz"));
        assert_eq!(out.events.len(), 0);
        assert_eq!(out.parse_failures, 0);
    }
}
