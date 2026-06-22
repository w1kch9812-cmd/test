import react from "@vitejs/plugin-react";
import { configDefaults, defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [react()],
  test: {
    environment: "happy-dom",
    setupFiles: ["./tests/unit/setup.ts"],
    // Unit lane (the CI `turbo test` step): deterministic, mocked tests only.
    // Redis-backed request/session flows run in the dedicated integration lane.
    include: ["tests/unit/**/*.test.{ts,tsx}", "lib/**/*.test.{ts,tsx}"],
    exclude: [
      ...configDefaults.exclude,
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
