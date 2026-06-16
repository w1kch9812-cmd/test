import { spawnSync } from "node:child_process";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";

const require = createRequire(import.meta.url);
const sizeLimitBin = join(dirname(require.resolve("size-limit/package.json")), "bin.js");

const result = spawnSync(process.execPath, [sizeLimitBin, ...process.argv.slice(2)], {
  env: {
    ...process.env,
    CI: process.env.CI ?? "true",
    GONGZZANG_BAZEL_SIZE_LIMIT: "1",
  },
  stdio: "inherit",
});

if (result.error) {
  throw result.error;
}

process.exit(result.status ?? 1);
