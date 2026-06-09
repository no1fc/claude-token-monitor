//! Tauri IPC commands. All return `Result<_, AppError>`; errors are serialized
//! to a `{kind, message}` shape that never contains credential material.

use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri_plugin_autostart::ManagerExt;

use crate::config::{self, Settings};
use crate::error::{AppError, AppResult};
use crate::model::UsageSnapshot;
use crate::plan::{PlanLimits, PlanTier};
use crate::state::AppState;

pub const EVENT_UPDATE: &str = "usage://update";
pub const EVENT_ERROR: &str = "usage://error";

/// Return the cached snapshot, building one if none exists yet.
#[tauri::command]
pub async fn get_snapshot(state: State<'_, AppState>) -> AppResult<UsageSnapshot> {
    if let Some(s) = state.last_snapshot.lock().unwrap().clone() {
        return Ok(s);
    }
    Ok(state.build_snapshot().await)
}

/// Rebuild immediately and push the result to all windows.
#[tauri::command]
pub async fn force_refresh(app: AppHandle, state: State<'_, AppState>) -> AppResult<UsageSnapshot> {
    let snap = state.build_snapshot().await;
    let _ = app.emit(EVENT_UPDATE, &snap);
    Ok(snap)
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> AppResult<Settings> {
    Ok(state.settings_snapshot())
}

#[tauri::command]
pub async fn update_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> AppResult<Settings> {
    let sanitized = settings.sanitized();
    config::save(&sanitized)?;
    *state.settings.lock().unwrap() = sanitized.clone();

    // Apply window-affecting settings immediately.
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.set_always_on_top(sanitized.always_on_top);
    }

    // Apply OS autostart registration.
    let autolaunch = app.autolaunch();
    let _ = if sanitized.autostart {
        autolaunch.enable()
    } else {
        autolaunch.disable()
    };

    // Rebuild so changes (plan, api toggle) reflect right away.
    let snap = state.build_snapshot().await;
    let _ = app.emit(EVENT_UPDATE, &snap);
    Ok(sanitized)
}

#[tauri::command]
pub async fn set_plan_override(
    app: AppHandle,
    state: State<'_, AppState>,
    tier: Option<PlanTier>,
    limits: Option<PlanLimits>,
) -> AppResult<Settings> {
    let mut settings = state.settings_snapshot();
    settings.plan_override = tier;
    settings.plan_limit_overrides = limits;
    update_settings(app, state, settings).await
}

#[tauri::command]
pub fn toggle_window(app: AppHandle) -> AppResult<()> {
    let win = app
        .get_webview_window("main")
        .ok_or_else(|| AppError::Other("main window missing".into()))?;
    if win.is_visible().unwrap_or(false) {
        let _ = win.hide();
    } else {
        let _ = win.show();
        let _ = win.set_focus();
    }
    Ok(())
}

/// Hide the settings window instead of destroying it on close, so it keeps its
/// state. Best-effort: `prevent_close` isn't fully reliable on all platforms, so
/// `open_settings_window` also recreates the window if it ends up destroyed.
pub(crate) fn attach_hide_on_close(win: &WebviewWindow) {
    let w = win.clone();
    win.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = w.hide();
        }
    });
}

#[tauri::command]
pub fn open_settings_window(app: AppHandle) -> AppResult<()> {
    // If the window still exists (hidden or minimized), just reveal it.
    if let Some(win) = app.get_webview_window("settings") {
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
        return Ok(());
    }

    // Otherwise it was destroyed on a previous close — recreate it. This makes
    // the gear button work reliably regardless of platform close behavior.
    let win = WebviewWindowBuilder::new(&app, "settings", WebviewUrl::App("settings.html".into()))
        .title("Settings — Claude Token Monitor")
        .inner_size(420.0, 520.0)
        .resizable(true)
        .build()
        .map_err(|e| AppError::Other(format!("failed to create settings window: {e}")))?;
    attach_hide_on_close(&win);
    let _ = win.set_focus();
    Ok(())
}

#[tauri::command]
pub fn quit_app(app: AppHandle) -> AppResult<()> {
    app.exit(0);
    Ok(())
}
