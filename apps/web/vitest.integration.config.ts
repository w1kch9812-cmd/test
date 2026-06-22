import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

// Integration tests exercise full request flows (auth login -> callback -> session)
// and need live infrastructure (redis session store, auth upstreams, an app server).
// They run in a dedicated lane (`pnpm test:integration`), NOT in the default unit
// `vitest run` used by the frontend CI unit step, so the unit lane stays deterministic.
export default defineConfig({
  plugins: [react()],
  test: {
    environment: "happy-dom",
    setupFiles: ["./tests/unit/setup.ts"],
    include: ["tests/integration/**/*.test.{ts,tsx}"],
    globals: true,
  },
  resolve: {
    alias: {
      "@": new URL("./", import.meta.url).pathname,
    },
  },
});
