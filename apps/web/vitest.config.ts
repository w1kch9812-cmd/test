import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [react()],
  test: {
    environment: "happy-dom",
    setupFiles: ["./tests/unit/setup.ts"],
    // Unit lane (the CI `turbo test` step): deterministic, mocked tests only.
    // Integration tests need a live app server + redis/auth upstreams and run in a
    // dedicated lane via `pnpm test:integration` (vitest.integration.config.ts).
    include: ["tests/unit/**/*.test.{ts,tsx}", "lib/**/*.test.{ts,tsx}"],
    globals: true,
  },
  resolve: {
    alias: {
      "@": new URL("./", import.meta.url).pathname,
    },
  },
});
