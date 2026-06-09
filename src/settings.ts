// Settings window: load current settings, edit, save.

import { getSettings, updateSettings } from "./ipc";
import type { PlanTier, Settings } from "./types";

const root = document.getElementById("settings-root")!;

const PLANS: { value: PlanTier | ""; label: string }[] = [
  { value: "", label: "Auto-detect" },
  { value: "pro", label: "Pro" },
  { value: "max5x", label: "Max 5x" },
  { value: "max20x", label: "Max 20x" },
  { value: "team", label: "Team" },
];

function render(s: Settings) {
  const planOpts = PLANS.map(
    (p) =>
      `<option value="${p.value}" ${
        (s.planOverride ?? "") === p.value ? "selected" : ""
      }>${p.label}</option>`,
  ).join("");

  root.innerHTML = `
    <h1>Settings</h1>
    <label class="row">
      <span>Refresh interval (seconds, min 60)</span>
      <input id="refresh" type="number" min="60" value="${s.refreshIntervalSecs}" />
    </label>
    <label class="row checkbox">
      <input id="useApi" type="checkbox" ${s.useApi ? "checked" : ""} />
      <span>Use live usage API (falls back to local estimates)</span>
    </label>
    <label class="row checkbox">
      <input id="alwaysOnTop" type="checkbox" ${s.alwaysOnTop ? "checked" : ""} />
      <span>Always on top</span>
    </label>
    <label class="row checkbox">
      <input id="autostart" type="checkbox" ${s.autostart ? "checked" : ""} />
      <span>Start automatically on system login</span>
    </label>
    <label class="row">
      <span>Plan</span>
      <select id="plan">${planOpts}</select>
    </label>
    <fieldset class="overrides">
      <legend>Limit overrides (tokens, optional)</legend>
      <label class="row">
        <span>5-hour limit</span>
        <input id="lim5" type="number" min="0" value="${s.planLimitOverrides?.fiveHourTokens ?? ""}" placeholder="auto" />
      </label>
      <label class="row">
        <span>7-day limit</span>
        <input id="lim7" type="number" min="0" value="${s.planLimitOverrides?.sevenDayTokens ?? ""}" placeholder="auto" />
      </label>
      <label class="row">
        <span>Context window</span>
        <input id="ctxLimit" type="number" min="0" value="${s.contextLimitOverride ?? ""}" placeholder="auto (e.g. 1000000)" />
      </label>
    </fieldset>
    <label class="row">
      <span>Opacity</span>
      <input id="opacity" type="range" min="0.2" max="1" step="0.05" value="${s.opacity}" />
    </label>
    <label class="row">
      <span>Theme</span>
      <select id="theme">
        <option value="dark" ${s.theme === "dark" ? "selected" : ""}>Dark</option>
        <option value="light" ${s.theme === "light" ? "selected" : ""}>Light</option>
      </select>
    </label>
    <div class="actions">
      <button id="save">Save</button>
      <span id="status"></span>
    </div>
    <p class="note">Note: the live API uses an undocumented endpoint and may break. Local estimates always work. Limits are approximate — calibrate against <code>/usage</code>.</p>
  `;

  document.getElementById("save")!.addEventListener("click", () => save(s));
}

function num(id: string): number | null {
  const v = (document.getElementById(id) as HTMLInputElement).value.trim();
  if (v === "") return null;
  const n = Number(v);
  return Number.isFinite(n) ? n : null;
}

async function save(prev: Settings) {
  const planVal = (document.getElementById("plan") as HTMLSelectElement)
    .value as PlanTier | "";
  const lim5 = num("lim5");
  const lim7 = num("lim7");
  const overrides =
    lim5 != null && lim7 != null
      ? { fiveHourTokens: lim5, sevenDayTokens: lim7 }
      : null;

  const next: Settings = {
    ...prev,
    refreshIntervalSecs: num("refresh") ?? prev.refreshIntervalSecs,
    useApi: (document.getElementById("useApi") as HTMLInputElement).checked,
    alwaysOnTop: (document.getElementById("alwaysOnTop") as HTMLInputElement)
      .checked,
    autostart: (document.getElementById("autostart") as HTMLInputElement).checked,
    planOverride: planVal === "" ? null : planVal,
    planLimitOverrides: overrides,
    contextLimitOverride: num("ctxLimit"),
    opacity: Number((document.getElementById("opacity") as HTMLInputElement).value),
    theme: (document.getElementById("theme") as HTMLSelectElement).value,
  };

  const status = document.getElementById("status")!;
  try {
    const saved = await updateSettings(next);
    status.textContent = "Saved ✓";
    render(saved);
  } catch (e) {
    status.textContent = "Save failed";
    console.error(e);
  }
}

getSettings()
  .then(render)
  .catch((e) => {
    root.innerHTML = `<p>Failed to load settings.</p>`;
    console.error(e);
  });
