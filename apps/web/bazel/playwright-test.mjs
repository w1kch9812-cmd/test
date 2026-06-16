import { spawnSync } from "node:child_process";
import { createRequire } from "node:module";
import { dirname, join, resolve } from "node:path";

const require = createRequire(import.meta.url);
const playwrightCli = join(dirname(require.resolve("@playwright/test/package.json")), "cli.js");
const nextDistDir = join(".next-bazel-dev", String(process.pid));

const result = spawnSync(process.execPath, [playwrightCli, "test", ...process.argv.slice(2)], {
  env: {
    ...process.env,
    CI: process.env.CI ?? "true",
    GONGZZANG_BAZEL_NEXT_DIST_DIR: process.env.GONGZZANG_BAZEL_NEXT_DIST_DIR ?? nextDistDir,
    GONGZZANG_BAZEL_PLAYWRIGHT: "1",
    PLAYWRIGHT_NEXT_CLI_PATH:
      process.env.PLAYWRIGHT_NEXT_CLI_PATH ?? resolve(process.cwd(), "bazel/next-cli.mjs"),
    PLAYWRIGHT_NODE_EXECUTABLE: process.env.PLAYWRIGHT_NODE_EXECUTABLE ?? process.execPath,
  },
  stdio: "inherit",
});

if (result.error) {
  throw result.error;
}

process.exit(result.status ?? 1);
