import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

// Integration tests exercise Redis-backed request/session flows. They run in a
// dedicated lane (`pnpm test:integration`), not in the default unit `vitest run`
// used by the frontend CI unit step, so the unit lane stays deterministic.
export default defineConfig({
  plugins: [react()],
  test: {
    environment: "happy-dom",
    setupFiles: ["./tests/unit/setup.ts"],
    include: [
      "tests/integration/**/*.test.{ts,tsx}",
      "tests/unit/platform-core-events.test.ts",
      "tests/unit/proxy.test.ts",
      "tests/unit/ratelimit.test.ts",
      "tests/unit/session/store.test.ts",
      "tests/unit/session/single-flight.test.ts",
    ],
    globals: true,
  },
  resolve: {
    alias: {
      "@": new URL("./", import.meta.url).pathname,
    },
  },
});
