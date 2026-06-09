---
name: ctm-dev
description: Use when building, running, testing, or verifying Claude Token Monitor locally (Tauri v2 + Rust + TS).
---

# Local dev & verify

## Setup
Rust lives at `%USERPROFILE%\.cargo\bin` and is NOT on PATH in fresh shells. Prepend it first (PowerShell):
```
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
```

## Loop
| Goal | Command |
|------|---------|
| Run unit tests (60) | `cargo test --manifest-path src-tauri/Cargo.toml` |
| Format (do before commit) | `cargo fmt --manifest-path src-tauri/Cargo.toml` |
| Type-check + build FE | `npm run build` |
| Run the app | `npm run tauri dev` |
| Build installers | `npm run tauri build` → `src-tauri/target/release/bundle/` |

## Verifying the running app
- `npm run tauri dev` starts Vite + the app. The debug exe loads from the Vite dev server, so don't run the debug exe standalone — use `tauri dev` or a release build.
- Check it's alive: `Get-Process claude-token-monitor` (window title `Claude Token Monitor`); WebView2 child processes confirm UI rendering.
- **Screenshots:** a full-screen capture only works if the workstation is unlocked; on the lock screen you'll capture the lock screen, not the widget. Win32 P/Invoke screen capture may be blocked by antivirus — prefer `System.Windows.Forms`/`System.Drawing` `CopyFromScreen`.

## Behavior to sanity-check
- Widget is frameless, always-on-top, draggable by the header.
- Tray: left-click toggles show/hide; right-click menu = Show/Hide · Force refresh · Settings · Quit.
- Settings window: hidden at launch; ⚙ opens it; minimize/close then ⚙ reopens it.
- Source dot: green = live API, amber = local estimate (with a warning banner).

## Common mistakes
- Running `cargo test`/`build` in a clean checkout without `npm run build` first → `generate_context!` fails (needs `../dist`).
- Forgetting `cargo fmt` → CI `fmt --check` fails.
