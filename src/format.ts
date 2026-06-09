// Small formatting helpers for the widget. Pure functions, no DOM.

/** Compact token counts: 1234 -> "1.2K", 3_500_000 -> "3.5M". */
export function compact(n: number): string {
  if (n < 1000) return String(Math.round(n));
  if (n < 1_000_000) return `${(n / 1000).toFixed(1)}K`;
  if (n < 1_000_000_000) return `${(n / 1_000_000).toFixed(2)}M`;
  return `${(n / 1_000_000_000).toFixed(2)}B`;
}

export function percent(p: number): string {
  return `${p.toFixed(p < 10 ? 1 : 0)}%`;
}

export function usd(n: number): string {
  if (n === 0) return "$0.00";
  if (n < 0.01) return "<$0.01";
  if (n < 100) return `$${n.toFixed(2)}`;
  return `$${Math.round(n).toLocaleString()}`;
}

/** Seconds -> "Dd Hh Mm" / "Hh Mm" / "Mm" (for "resets in" countdowns). */
export function duration(secs: number): string {
  if (secs <= 0) return "now";
  const d = Math.floor(secs / 86400);
  const h = Math.floor((secs % 86400) / 3600);
  const m = Math.floor((secs % 3600) / 60);
  if (d > 0) return `${d}d ${h}h ${m}m`;
  if (h > 0) return `${h}h ${m}m`;
  if (m > 0) return `${m}m`;
  return `${secs}s`;
}

/** Seconds remaining until an ISO timestamp, relative to now. */
export function secsUntil(iso: string | null, now: number = Date.now()): number {
  if (!iso) return 0;
  const t = Date.parse(iso);
  if (Number.isNaN(t)) return 0;
  return Math.max(0, Math.round((t - now) / 1000));
}

/** "12:34" local clock for an ISO timestamp. */
export function clock(iso: string | null): string {
  if (!iso) return "—";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "—";
  return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

/** "just now" / "3m ago" for the last-updated footer. */
export function ago(iso: string, now: number = Date.now()): string {
  const t = Date.parse(iso);
  if (Number.isNaN(t)) return "—";
  const s = Math.max(0, Math.round((now - t) / 1000));
  if (s < 10) return "just now";
  if (s < 60) return `${s}s ago`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${m}m ago`;
  return `${Math.floor(m / 60)}h ago`;
}

export function planLabel(plan: string): string {
  switch (plan) {
    case "pro":
      return "Pro";
    case "max5x":
      return "Max 5x";
    case "max20x":
      return "Max 20x";
    case "team":
      return "Team";
    default:
      return "Unknown";
  }
}

/** Short model family name from a model id. */
export function modelShort(model: string): string {
  const m = model.toLowerCase();
  if (m.includes("opus")) return "Opus";
  if (m.includes("sonnet")) return "Sonnet";
  if (m.includes("haiku")) return "Haiku";
  return model;
}
