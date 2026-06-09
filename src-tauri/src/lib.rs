//! Claude Token Monitor — library entrypoint. Exposes the pure-logic modules
//! (for tests) and wires up the Tauri application in `run()`.

pub mod analytics;
pub mod api;
pub mod commands;
pub mod config;
pub mod credentials;
pub mod error;
pub mod jsonl;
pub mod model;
pub mod paths;
pub mod plan;
pub mod refresher;
pub mod snapshot;
pub mod state;
pub mod watcher;

use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager};

use state::AppState;

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let toggle = MenuItemBuilder::with_id("toggle", "Show / Hide").build(app)?;
    let refresh = MenuItemBuilder::with_id("refresh", "Force refresh").build(app)?;
    let settings = MenuItemBuilder::with_id("settings", "Settings").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
    let menu = MenuBuilder::new(app)
        .items(&[&toggle, &refresh, &settings, &quit])
        .build()?;

    let mut builder = TrayIconBuilder::new()
        .tooltip("Claude Token Monitor")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "toggle" => {
                let _ = commands::toggle_window(app.clone());
            }
            "refresh" => {
                let app2 = app.clone();
                tauri::async_runtime::spawn(async move {
                    let snap = {
                        let state = app2.state::<AppState>();
                        state.build_snapshot().await
                    };
                    let _ = app2.emit(commands::EVENT_UPDATE, &snap);
                });
            }
            "settings" => {
                let _ = commands::open_settings_window(app.clone());
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = commands::toggle_window(tray.app_handle().clone());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::force_refresh,
            commands::get_settings,
            commands::update_settings,
            commands::set_plan_override,
            commands::toggle_window,
            commands::open_settings_window,
            commands::quit_app,
        ])
        .setup(|app| {
            build_tray(app)?;

            // Apply persisted always-on-top preference to the widget.
            let settings = app.state::<AppState>().settings_snapshot();
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.set_always_on_top(settings.always_on_top);
            }

            refresher::spawn(app.handle().clone());
            watcher::spawn(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
