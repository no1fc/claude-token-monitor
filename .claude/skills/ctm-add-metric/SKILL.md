---
name: ctm-add-metric
description: Use when adding a new tracked field or metric to the Claude Token Monitor widget that must flow from the Rust backend through to the displayed UI.
---

# Adding a tracked metric end-to-end

A displayed value crosses the Rust↔TS boundary. Touch these layers in order; the wire model (`model.rs`) and `types.ts` must stay mirrored (serde `camelCase`).

## Order of changes

1. **Compute it (pure logic, TDD first).** Add/extend a function in `src-tauri/src/analytics/` (or `jsonl/`). Inject `now`; add a `#[cfg(test)]` test for it. Reuse helpers: `aggregate::{events_in_window, sum_tokens, per_model, total_cost}`, `blocks::active_block`, `pricing::cost`.
2. **Add the field to the wire model** — `src-tauri/src/model.rs` (the relevant struct, e.g. `UsageSnapshot`, `BurnStats`, `CostStats`). It's `#[serde(rename_all = "camelCase")]`.
3. **Populate it** — `src-tauri/src/snapshot.rs::build(...)` where the snapshot is assembled. Add/adjust the test there.
4. **Mirror the type** — `src/types.ts` (matching interface; camelCase name).
5. **Render it** — `src/render.ts` (`renderSnapshot`; add to a `.stat` row, a gauge, or the model list). Use `src/format.ts` helpers: `compact`, `percent`, `usd`, `duration`, `clock`.
6. **If it's time-relative** (a countdown), update `tickTimes` in `render.ts` and tag the element with a `data-*` attribute so the 1s local tick refreshes it without IPC.

## Verify

```
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
cargo test --manifest-path src-tauri/Cargo.toml
npm run build        # type-checks types.ts ↔ render.ts
npm run tauri dev    # eyeball the widget
```

## Common mistakes

- **Forgot the `types.ts` mirror** → `npm run build` fails type-check, or the field is `undefined` at runtime.
- **Computed `now` inside logic** → breaks deterministic tests; pass `now` in.
- **Cost without per-model context** → cost must be summed per event by its model (`aggregate::total_cost`), not on aggregated totals.
