//! Watch `~/.claude/projects` for transcript writes and push a JSONL-only
//! refresh (debounced) so token/cost numbers update near-instantly during use
//! without calling the rate-limited API.

use std::time::Duration;

use notify::{RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter, Manager};

use crate::commands::EVENT_UPDATE;
use crate::state::AppState;

const DEBOUNCE: Duration = Duration::from_millis(750);

pub fn spawn(app: AppHandle) {
    let Some(dir) = crate::paths::projects_dir() else {
        return;
    };
    if !dir.exists() {
        return;
    }

    // notify's recv is blocking, so run it on a dedicated OS thread.
    std::thread::spawn(move || {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = match notify::recommended_watcher(move |res| {
            let _ = tx.send(res);
        }) {
            Ok(w) => w,
            Err(_) => return,
        };
        if watcher.watch(&dir, RecursiveMode::Recursive).is_err() {
            return;
        }

        loop {
            // Block until something changes.
            if rx.recv().is_err() {
                break;
            }
            // Debounce: drain a burst of events before rebuilding.
            while rx.recv_timeout(DEBOUNCE).is_ok() {}

            let app2 = app.clone();
            tauri::async_runtime::spawn(async move {
                let snap = {
                    let state = app2.state::<AppState>();
                    state.build_snapshot_local().await
                };
                let _ = app2.emit(EVENT_UPDATE, &snap);
            });
        }
    });
}
