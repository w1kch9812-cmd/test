import { spawnSync } from "node:child_process";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const biomeBin = require.resolve("@biomejs/biome/bin/biome");
const mode = process.argv[2] ?? "check";

const result = spawnSync(process.execPath, [biomeBin, mode, "."], {
  env: {
    ...process.env,
    CI: process.env.CI ?? "true",
  },
  stdio: "inherit",
});

if (result.error) {
  throw result.error;
}

process.exit(result.status ?? 1);
