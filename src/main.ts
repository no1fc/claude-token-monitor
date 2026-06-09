// Widget bootstrap: paint once from cache, then update on pushed events.

import {
  forceRefresh,
  getSnapshot,
  onError,
  onUpdate,
  openSettingsWindow,
} from "./ipc";
import { renderEmpty, renderLoading, renderSnapshot, tickTimes } from "./render";
import type { UsageSnapshot } from "./types";

const root = document.getElementById("app")!;
let latest: UsageSnapshot | null = null;

function paint(s: UsageSnapshot) {
  latest = s;
  renderSnapshot(root, s);
  wireButtons();
}

function wireButtons() {
  document.getElementById("btn-refresh")?.addEventListener("click", () => {
    forceRefresh().catch((e) => console.error("refresh failed", e));
  });
  document.getElementById("btn-settings")?.addEventListener("click", () => {
    openSettingsWindow().catch((e) => console.error("open settings failed", e));
  });
}

async function init() {
  renderLoading(root);

  // Push updates from the backend (timer + file watcher).
  await onUpdate(paint);
  await onError((e) => console.warn("usage error", e.kind, e.message));

  try {
    const snap = await getSnapshot();
    paint(snap);
  } catch (e) {
    renderEmpty(root, "No Claude data found.");
    console.error(e);
  }

  // Smooth local countdown without IPC chatter.
  setInterval(() => {
    if (latest) tickTimes(root);
  }, 1000);
}

init();
