# CLAUDE.md — Claude Token Monitor

Project context for AI agents. Read this first; it tells you where things live and how to change them safely.

## What this is

Cross-platform (Win/Linux/macOS) **Tauri v2** desktop overlay showing remaining Claude Code token quota (5-hour + 7-day windows: remaining time/amount/percent, burn rate & ETA, cost $, per-model breakdown, session tokens, plan tier). Frameless always-on-top draggable widget + system tray. Hybrid data: local JSONL parsing + best-effort unofficial usage API.

## Architecture (the important mental model)

Two layers — keep them separate:

- **Pure logic (no I/O, deterministic, unit-tested)** — `src-tauri/src/analytics/` and `src-tauri/src/jsonl/`. Every function takes `now: DateTime<Utc>` as a parameter instead of calling `Utc::now()`, so tests are deterministic. **All time math is UTC.**
- **Tauri glue (I/O, app wiring)** — `state.rs` (orchestration), `commands.rs` (IPC), `refresher.rs` (timer), `watcher.rs` (file watch), `lib.rs` (builder/tray/plugins).

Data flow: `jsonl::scan_dir` → `jsonl::parser` (dedup) → `analytics::*` → `snapshot::build` (merges JSONL + API) → `UsageSnapshot` → emitted to frontend as `usage://update`.

### File map
| Area | File |
|------|------|
| Shared data contract (wire model) | `src-tauri/src/model.rs` ↔ mirrored in `src/types.ts` |
| 5-hour block algorithm | `src-tauri/src/analytics/blocks.rs` |
| 7-day / window status | `src-tauri/src/analytics/windows.rs` |
| Per-model + cost aggregation | `src-tauri/src/analytics/aggregate.rs`, `pricing.rs` |
| Burn rate / ETA | `src-tauri/src/analytics/burn_rate.rs` |
| JSONL parse + dedup | `src-tauri/src/jsonl/{records,parser,scanner}.rs` |
| Snapshot assembly | `src-tauri/src/snapshot.rs` |
| Usage API + token refresh | `src-tauri/src/api/{usage_client,token_refresh}.rs` |
| Credentials (token, redacted) | `src-tauri/src/credentials.rs` |
| Settings persistence | `src-tauri/src/config.rs` |
| IPC commands | `src-tauri/src/commands.rs` |
| Widget UI | `index.html`, `src/{main,render,format,ipc}.ts`, `src/styles.css` |
| Settings UI | `settings.html`, `src/settings.ts` |

## Commands

> Rust is installed at `%USERPROFILE%\.cargo\bin` and is **not on PATH in fresh shells** — prepend it: `$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"` (PowerShell).

| Task | Command |
|------|---------|
| Unit tests (60) | `cargo test --manifest-path src-tauri/Cargo.toml` |
| Format | `cargo fmt --manifest-path src-tauri/Cargo.toml` |
| Format check (CI gate) | `cargo fmt --manifest-path src-tauri/Cargo.toml --check` |
| Frontend type-check + build | `npm run build` |
| Run the app | `npm run tauri dev` |
| Build installers | `npm run tauri build` |

## Conventions (follow these)

- **TDD for pure logic.** Add/adjust tests in the relevant `analytics`/`jsonl` module's `#[cfg(test)]` block. Inject `now`; never call `Utc::now()` inside logic.
- **Immutability** — prefer returning new values (e.g. `TokenBreakdown::plus`) over mutation.
- **Wire model is mirrored.** `model.rs` uses `#[serde(rename_all = "camelCase")]`; any change there MUST be mirrored in `src/types.ts`.
- **Never log/serialize tokens.** No `println!`/logging of credentials; `AppError` carries no token; `Credentials`/`InMemoryToken` have redacting `Debug` + zeroize. Keep it that way (see [security.md] expectations).
- **Run `cargo fmt` before committing** — CI fails on unformatted code.
- **Limits are approximate** (`plan.rs`) and user-overridable; when the API is up, gauges use API percent, not these constants.

## Common change recipes

- **Add a tracked metric to the widget** → use the `ctm-add-metric` skill. Path: `model.rs` → `snapshot.rs` → `types.ts` → `render.ts`.
- **Update model pricing** → `src-tauri/src/analytics/pricing.rs` (`model_pricing`), update the test.
- **Add a setting** → `config.rs` (`Settings` + `Default` + `sanitized`) → apply in `commands.rs::update_settings` → `types.ts` → `src/settings.ts` form.
- **Cut a release** → use the `ctm-release` skill (bumps 3 version files, tags, CI publishes installers).
- **Run / verify locally** → use the `ctm-dev` skill.

## Release & CI

- CI (`.github/workflows/ci.yml`): tests + `fmt --check` + frontend build on every push/PR.
- Release (`.github/workflows/release.yml`): pushing a `v*` tag builds Windows + macOS (arm64/x64) + Linux and publishes installers.
- Version lives in **three** files that must stay in sync: `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json` (then refresh `Cargo.lock` via a build).

## Gotchas

- `tauri::generate_context!()` embeds `../dist` at compile time → run `npm run build` before `cargo test`/`cargo build` in clean checkouts (CI does this).
- The usage API endpoint is **undocumented** and rate-limited — always keep the JSONL fallback working; poll ≥60s.
- macOS transparency requires the `macos-private-api` Cargo feature (already set; matches `macOSPrivateApi: true`).
- Settings window: closing hides (not destroys) it; single-instance plugin prevents duplicate windows.
