// Render the widget DOM from a UsageSnapshot. No framework — plain DOM strings.

import type { UsageSnapshot, WindowStatus } from "./types";
import {
  ago,
  clock,
  compact,
  duration,
  modelShort,
  percent,
  planLabel,
  secsUntil,
  usd,
} from "./format";

function level(p: number): string {
  if (p >= 90) return "danger";
  if (p >= 70) return "warn";
  return "ok";
}

function esc(s: string): string {
  return s.replace(/[&<>"]/g, (c) =>
    ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c] as string,
  );
}

function gauge(title: string, w: WindowStatus): string {
  const p = w.percentUsed;
  const lvl = level(p);
  const resetAttr = w.resetsAt ? `data-reset="${w.resetsAt}"` : "";
  const resetText = w.resetsAt ? `resets in ${duration(secsUntil(w.resetsAt))}` : "—";
  const tokens = w.tokensUsed != null ? ` · ${compact(w.tokensUsed)} tok` : "";
  return `
    <div class="gauge">
      <div class="gauge-head">
        <span class="gauge-title">${title}</span>
        <span class="gauge-pct ${lvl}">${percent(p)}</span>
      </div>
      <div class="bar"><div class="fill ${lvl}" style="width:${Math.min(100, p)}%"></div></div>
      <div class="gauge-sub">
        <span>${percent(w.remainingPercent)} left${tokens}</span>
        <span class="reset" ${resetAttr}>${resetText}</span>
      </div>
    </div>`;
}

function perModelRows(s: UsageSnapshot): string {
  if (s.perModel.length === 0) return "";
  const rows = s.perModel
    .slice(0, 5)
    .map(
      (m) =>
        `<div class="model-row"><span>${esc(modelShort(m.model))}</span>` +
        `<span>${compact(m.tokens.total)} · ${usd(m.costUsd)}</span></div>`,
    )
    .join("");
  return `<details class="models"><summary>Models (7d)</summary>${rows}</details>`;
}

export function renderSnapshot(root: HTMLElement, s: UsageSnapshot): void {
  const sourceTitle =
    s.source === "hybrid" ? "Live (API)" : "Estimated (local)";
  const dotClass = s.source === "hybrid" ? "live" : "est";
  const burnEta = s.burn.fiveHourEta
    ? `ETA ${clock(s.burn.fiveHourEta)}`
    : "no 5h ETA";
  const warn =
    s.warnings.length > 0
      ? `<div class="warnbar">${esc(s.warnings[0])}</div>`
      : "";

  root.innerHTML = `
    <div class="header">
      <span class="dot ${dotClass}" title="${sourceTitle}"></span>
      <span class="plan">${planLabel(s.plan)}</span>
      <span class="drag" data-tauri-drag-region></span>
      <button class="icon" id="btn-refresh" title="Refresh">⟳</button>
      <button class="icon" id="btn-settings" title="Settings">⚙</button>
    </div>
    ${warn}
    ${gauge("5-HOUR", s.fiveHour)}
    ${gauge("WEEKLY", s.sevenDay)}
    <div class="stats">
      <div class="stat"><span>Burn</span><span>${compact(s.burn.tokensPerMin)}/min · ${burnEta}</span></div>
      <div class="stat"><span>Cost</span><span>${usd(s.cost.sessionUsd)} sess · ${usd(s.cost.sevenDayUsd)} wk</span></div>
      <div class="stat"><span>Session</span><span>${compact(s.currentSession.tokens.total)} tok</span></div>
    </div>
    ${perModelRows(s)}
    <div class="footer"><span>updated <span id="updated" data-updated="${s.generatedAt}">${ago(s.generatedAt)}</span></span></div>
  `;
}

/** Update only the time-relative bits (countdowns + "updated Xs ago"). */
export function tickTimes(root: HTMLElement): void {
  root.querySelectorAll<HTMLElement>(".reset[data-reset]").forEach((el) => {
    const iso = el.getAttribute("data-reset");
    el.textContent = `resets in ${duration(secsUntil(iso))}`;
  });
  const upd = root.querySelector<HTMLElement>("#updated[data-updated]");
  if (upd) {
    upd.textContent = ago(upd.getAttribute("data-updated") || "");
  }
}

export function renderLoading(root: HTMLElement): void {
  root.innerHTML = `<div class="loading">Loading usage…</div>`;
}

export function renderEmpty(root: HTMLElement, msg: string): void {
  root.innerHTML = `<div class="empty">${esc(msg)}</div>`;
}
