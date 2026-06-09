import { defineConfig } from "vite";

// Tauri expects a fixed dev port and looks for the frontend on 1420.
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  // Multi-page: the main widget (index.html) and the settings window.
  build: {
    target: "esnext",
    rollupOptions: {
      input: {
        main: "index.html",
        settings: "settings.html",
      },
    },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? { protocol: "ws", host, port: 1421 }
      : undefined,
    watch: {
      // Don't watch the Rust backend; cargo handles that.
      ignored: ["**/src-tauri/**"],
    },
  },
});
