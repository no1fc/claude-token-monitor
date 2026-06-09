// Thin wrappers around the Tauri IPC surface.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { Settings, UsageSnapshot, WireError, PlanTier } from "./types";

export const EVENT_UPDATE = "usage://update";
export const EVENT_ERROR = "usage://error";

export function getSnapshot(): Promise<UsageSnapshot> {
  return invoke<UsageSnapshot>("get_snapshot");
}

export function forceRefresh(): Promise<UsageSnapshot> {
  return invoke<UsageSnapshot>("force_refresh");
}

export function getSettings(): Promise<Settings> {
  return invoke<Settings>("get_settings");
}

export function updateSettings(settings: Settings): Promise<Settings> {
  return invoke<Settings>("update_settings", { settings });
}

export function setPlanOverride(
  tier: PlanTier | null,
  limits: { fiveHourTokens: number; sevenDayTokens: number } | null,
): Promise<Settings> {
  return invoke<Settings>("set_plan_override", { tier, limits });
}

export function toggleWindow(): Promise<void> {
  return invoke("toggle_window");
}

export function openSettingsWindow(): Promise<void> {
  return invoke("open_settings_window");
}

export function quitApp(): Promise<void> {
  return invoke("quit_app");
}

export function onUpdate(cb: (s: UsageSnapshot) => void): Promise<UnlistenFn> {
  return listen<UsageSnapshot>(EVENT_UPDATE, (e) => cb(e.payload));
}

export function onError(cb: (e: WireError) => void): Promise<UnlistenFn> {
  return listen<WireError>(EVENT_ERROR, (e) => cb(e.payload));
}
