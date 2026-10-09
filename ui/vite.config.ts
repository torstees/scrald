import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Tauri expects a fixed dev-server port (see devUrl in crates/scrald-app/tauri.conf.json).
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    watch: {
      // Rust sources are rebuilt by the Tauri CLI, not Vite.
      ignored: ["**/crates/**", "**/target/**"],
    },
  },
  build: {
    target: "es2022",
    // Mermaid is a large, lazily loaded chunk; that's expected, not a warning.
    chunkSizeWarningLimit: 1500,
  },
  // Pre-bundle the Tauri API modules and the lazy renderers up front. Otherwise Vite discovers them
  // at runtime and reloads the page mid-startup, which aborts in-flight IPC
  // calls (dev mode only).
  optimizeDeps: {
    include: ["@tauri-apps/api/core", "@tauri-apps/api/event", "katex", "mermaid", "abcjs"],
  },
  test: {
    include: ["src/**/*.test.ts"],
  },
});
