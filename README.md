# Claude Token Monitor

**English** | [한국어](README.ko.md)

> A tiny, always-on-top desktop overlay that shows how much of your **Claude Code**
> token quota remains — at a glance, without running `/usage` in the terminal.

[![CI](https://github.com/no1fc/claude-token-monitor/actions/workflows/ci.yml/badge.svg)](https://github.com/no1fc/claude-token-monitor/actions/workflows/ci.yml)
![platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-blue)
![stack](https://img.shields.io/badge/built%20with-Tauri%20v2%20%2B%20Rust-orange)
![license](https://img.shields.io/badge/license-MIT-green)

Cross-platform, built with **Tauri v2** (Rust backend + plain TypeScript frontend).
The packaged app is a single small binary (~5 MB) with a frameless, draggable widget
and a system-tray icon.

> ⚠️ **Unofficial / community tool.** Not affiliated with or endorsed by Anthropic.
> See [Caveats](#️-caveat-on-the-live-api).

---

## ✨ Features

For both the **5-hour rolling window** and the **7-day (weekly) window**:

- **Percent used / remaining** with a colour-coded gauge (green → amber → red).
- **Live "resets in hh:mm" countdown**.

Plus:

- **Burn rate** (tokens/min) and **limit-hit ETA**.
- **Estimated cost ($)** for the current session and the week.
- **Per-model breakdown** (Opus / Sonnet / Haiku …).
- **Current session tokens** and your **plan tier** badge (Pro / Max 5x / Max 20x).

A coloured dot shows the data source: **🟢 green = live API**, **🟠 amber = local estimate**.

The widget floats over other apps, can be dragged anywhere, and hides/shows from the tray.
Window position and all settings persist across restarts.

---

## 📦 Install

### From a release (recommended)

Grab the latest installer from the [**Releases**](../../releases) page:

| OS | File |
|----|------|
| Windows | `Claude Token Monitor_<ver>_x64-setup.exe` (NSIS) or `..._x64_en-US.msi` |
| macOS | `Claude Token Monitor_<ver>_x64.dmg` (build from source, see below) |
| Linux | `claude-token-monitor_<ver>_amd64.AppImage` / `.deb` (build from source) |

> Only Windows binaries are published initially. macOS/Linux users build from source
> (one command — see below); the codebase is fully cross-platform.

### Build from source

Prerequisites:

- **Node.js** 18+
- **Rust** (stable) — install via <https://rustup.rs> (MSVC toolchain on Windows)
- Tauri system dependencies for your OS — see the
  [Tauri prerequisites guide](https://v2.tauri.app/start/prerequisites/)
  (WebView2 on Windows — preinstalled on Win 11; `webkit2gtk` + `libayatana-appindicator` on Linux; Xcode CLT on macOS)

```bash
git clone <this-repo-url>
cd claudeTokenCheckApp
npm install

npm run tauri dev      # run in development
npm run tauri build    # produce installers/bundles for the current OS
```

Bundles are written to `src-tauri/target/release/bundle/`.

---

## 🕹 Usage

- **Move it:** drag the empty area in the widget header.
- **⟳** refresh now · **⚙** open Settings.
- **Tray icon:** left-click toggles show/hide; right-click for
  Show/Hide · Force refresh · Settings · Quit.
- **Settings:** refresh interval (min 60 s), live-API toggle, plan + limit overrides,
  always-on-top, **start on login**, opacity, theme. Saved to your OS config dir.

### Run instantly & auto-start (Windows)

Convenience scripts live in [`scripts/`](scripts):

| Script | What it does |
|--------|--------------|
| `scripts\run.bat` | Double-click to launch the built app instantly (falls back to dev mode if not yet built). |
| `scripts\enable-autostart.bat` | Adds the app to the Windows **Startup** folder so it launches at login. |
| `scripts\disable-autostart.bat` | Removes that Startup entry. |

**Auto-start (all platforms):** open **Settings → "Start automatically on system login"**.
This is the recommended, cross-platform way (Windows registry / macOS LaunchAgent / Linux
`.desktop` autostart) and works with the installed app. Running both the in-app toggle and
the batch script is safe — the app is single-instance and won't open twice.

---

## 🧠 How it works (data sources)

The app is **hybrid** and works fully offline if needed:

1. **Local JSONL — always works (primary/fallback).** Parses Claude Code's transcripts
   under `~/.claude/projects/**/*.jsonl`, de-duplicates by `(requestId, message.id)`,
   reconstructs the 5-hour billing "blocks" (same approach as
   [ccusage](https://github.com/ryoppippi/ccusage)), and sums the 7-day window.
   Token counts, cost, burn rate and per-model data all come from here.
2. **Unofficial usage API — best-effort (for exact percentages).** When enabled and
   credentials are present, it calls the same endpoint Claude Code's `/usage` uses to
   get authoritative percent-used and reset times, and merges those over the estimate.

Limits for the local estimate are **approximate** and **user-overridable** in Settings —
calibrate them against your own `/usage` output.

---

## 🔒 Security

- Your OAuth token is **read locally** and sent **only** to Anthropic's own endpoints
  over HTTPS (rustls). It is never sent to any third party.
- The token is **never logged**, never passed to the frontend, and never embedded in
  error messages. The backend has no logging of credentials at all.
- Refreshed tokens are kept **in memory only** and are **not** written back to
  `~/.claude/.credentials.json` by default (so the app never races Claude Code's own
  token management).
- The repository contains **no secrets**; the only Claude data the app touches lives in
  your local `~/.claude` directory at runtime.

---

## ⚠️ Caveat on the live API

The exact remaining-quota numbers come from an **undocumented** endpoint
(`GET /api/oauth/usage`) using your local OAuth token — the same data `/usage` shows.
However:

- It is **not an official, supported API** and may change or break at any time.
- Anthropic's Terms restrict reuse of OAuth tokens — use at your own discretion.
- The endpoint is aggressively rate-limited, so the app polls **no more than once per
  60 seconds** and backs off on errors.

If the API is disabled or unavailable, the app falls back to **local estimates** from
your transcripts, which always work offline. You can turn the API off entirely in Settings.

---

## 🧪 Development

```bash
cargo test --manifest-path src-tauri/Cargo.toml   # 60 unit tests (analytics, parsing, security)
npm run build                                      # type-check + bundle frontend
```

Architecture: pure analytics logic lives in `src-tauri/src/analytics/` and
`src-tauri/src/jsonl/` (no I/O, deterministic — `now` is injected — and fully unit-tested).
The Tauri layer (`commands`, `state`, `refresher`, `watcher`) orchestrates it and pushes
`usage://update` events to the frontend.

**Contributor guide:** see [`CLAUDE.md`](CLAUDE.md) for the architecture map, conventions,
and change recipes. Project workflows are codified as skills in
[`.claude/skills/`](.claude/skills) — `ctm-dev` (build/run/verify), `ctm-add-metric`
(add a tracked field end-to-end), and `ctm-release` (cut a cross-platform release).

---

## 📄 License

[MIT](LICENSE) © 2026 no1fc. Unofficial community tool, not affiliated with Anthropic.
