---
name: ctm-release
description: Use when cutting/publishing a new version of Claude Token Monitor — bumping the version, tagging, and triggering the cross-platform installer release.
---

# Releasing Claude Token Monitor

Pushing a `v*` tag makes GitHub Actions (`release.yml`) build and publish Windows + macOS (arm64/x64) + Linux installers. Get the repo green first, bump the version in **all three** manifests, then tag.

## Steps

1. **Pre-flight (must be green):**
   ```
   $env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
   cargo fmt --manifest-path src-tauri/Cargo.toml --check
   cargo test --manifest-path src-tauri/Cargo.toml
   npm run build
   ```
2. **Bump the version in all three files** (keep identical, e.g. `0.1.3` → `0.1.4`):
   - `package.json` → `"version"`
   - `src-tauri/Cargo.toml` → `[package] version`
   - `src-tauri/tauri.conf.json` → `"version"`
3. **Refresh `Cargo.lock`** so it matches: `cargo build --manifest-path src-tauri/Cargo.toml` (any cargo build/test works).
4. **Commit & push** to `main` (CI runs):
   ```
   git add -A
   git commit -m "release: vX.Y.Z"
   git push origin main
   ```
5. **Tag & push the tag** (triggers the release build):
   ```
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```
6. **Watch it** and confirm assets:
   ```
   gh run list -R no1fc/claude-token-monitor --limit 3
   gh release view vX.Y.Z -R no1fc/claude-token-monitor --json assets --jq '.assets[].name'
   ```
   Expect: `.exe`, `.msi`, two `.dmg` (aarch64 + x64), `.AppImage`, `.deb`, `.rpm`.

## Common mistakes

- **Version mismatch across the 3 files** → bundle names/tag disagree. Bump all three.
- **Forgot `cargo fmt`** → CI fails on `fmt --check`. Always format before pushing.
- **Tag already exists** (e.g. retrying same version) → either bump to a new version or delete the old tag/release first (`gh release delete vX.Y.Z --cleanup-tag`).
- **`cargo` not found** → prepend `%USERPROFILE%\.cargo\bin` to PATH (fresh shells don't have it).
