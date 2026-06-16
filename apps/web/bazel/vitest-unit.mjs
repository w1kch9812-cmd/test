import { spawnSync } from "node:child_process";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";

const require = createRequire(import.meta.url);
const vitestBin = join(dirname(require.resolve("vitest/package.json")), "vitest.mjs");

const result = spawnSync(process.execPath, [vitestBin, ...process.argv.slice(2)], {
  env: {
    ...process.env,
    CI: process.env.CI ?? "true",
    NODE_ENV: process.env.NODE_ENV ?? "test",
  },
  stdio: "inherit",
});

if (result.error) {
  throw result.error;
}

process.exit(result.status ?? 1);
