//! Filesystem locations for Claude Code data and our own config.
//! Cross-platform via the `dirs` crate; honours `$CLAUDE_HOME`.

use std::path::PathBuf;

const APP_DIR_NAME: &str = "claudeTokenMonitor";

/// `~/.claude` (or `$CLAUDE_HOME` when set).
pub fn claude_home() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("CLAUDE_HOME") {
        if !p.trim().is_empty() {
            return Some(PathBuf::from(p));
        }
    }
    dirs::home_dir().map(|h| h.join(".claude"))
}

/// `~/.claude/.credentials.json`
pub fn credentials_path() -> Option<PathBuf> {
    claude_home().map(|c| c.join(".credentials.json"))
}

/// `~/.claude/projects`
pub fn projects_dir() -> Option<PathBuf> {
    claude_home().map(|c| c.join("projects"))
}

/// `~/.claude/stats-cache.json`
pub fn stats_cache_path() -> Option<PathBuf> {
    claude_home().map(|c| c.join("stats-cache.json"))
}

/// Config directory for this app, e.g. `%APPDATA%/claudeTokenMonitor` or
/// `~/.config/claudeTokenMonitor`.
pub fn settings_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|c| c.join(APP_DIR_NAME))
}

/// `<settings_dir>/settings.json`
pub fn settings_path() -> Option<PathBuf> {
    settings_dir().map(|d| d.join("settings.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_home_respects_env_override() {
        // Save & restore to avoid cross-test contamination.
        let prev = std::env::var("CLAUDE_HOME").ok();
        std::env::set_var("CLAUDE_HOME", "/tmp/custom-claude");
        assert_eq!(claude_home(), Some(PathBuf::from("/tmp/custom-claude")));
        assert_eq!(
            credentials_path(),
            Some(PathBuf::from("/tmp/custom-claude/.credentials.json"))
        );
        match prev {
            Some(v) => std::env::set_var("CLAUDE_HOME", v),
            None => std::env::remove_var("CLAUDE_HOME"),
        }
    }
}
