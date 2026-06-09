//! Periodic snapshot refresh on a timer. The interval is read from settings
//! each cycle and floored at `MIN_REFRESH_SECS` to respect the API rate limit.

use tauri::{AppHandle, Emitter, Manager};

use crate::commands::EVENT_UPDATE;
use crate::config::MIN_REFRESH_SECS;
use crate::state::AppState;

pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            let snap = {
                let state = app.state::<AppState>();
                state.build_snapshot().await
            };
            let _ = app.emit(EVENT_UPDATE, &snap);

            let interval = {
                let state = app.state::<AppState>();
                state
                    .settings_snapshot()
                    .refresh_interval_secs
                    .max(MIN_REFRESH_SECS)
            };
            tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
        }
    });
}
