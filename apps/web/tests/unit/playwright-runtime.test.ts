import { describe, expect, it } from "vitest";

import {
  DEFAULT_E2E_PLAYWRIGHT_PORT,
  DEFAULT_PROBE_PLAYWRIGHT_PORT,
  resolvePlaywrightRuntime,
} from "../../playwright-runtime";

describe("playwright runtime SSOT", () => {
  it("uses an isolated default e2e port instead of the shared app dev port", () => {
    const runtime = resolvePlaywrightRuntime({
      env: {},
      defaultPort: DEFAULT_E2E_PLAYWRIGHT_PORT,
    });

    expect(runtime.baseURL).toBe("http://127.0.0.1:3100");
    expect(runtime.port).toBe(3100);
    expect(runtime.reuseExistingServer).toBe(false);
    expect(runtime.webServerUrl).toBe(runtime.baseURL);
    expect(runtime.zitadelRedirectUri).toBe("http://127.0.0.1:3100/api/auth/callback");
  });

  it("uses a separate default probe port", () => {
    const runtime = resolvePlaywrightRuntime({
      env: {},
      defaultPort: DEFAULT_PROBE_PLAYWRIGHT_PORT,
    });

    expect(runtime.baseURL).toBe("http://127.0.0.1:3101");
    expect(runtime.port).toBe(3101);
  });

  it("lets explicit env override the managed endpoint", () => {
    const runtime = resolvePlaywrightRuntime({
      env: {
        PLAYWRIGHT_HOST: "localhost",
        PLAYWRIGHT_PORT: "4100",
        PLAYWRIGHT_REUSE_EXISTING_SERVER: "1",
      },
      defaultPort: DEFAULT_E2E_PLAYWRIGHT_PORT,
    });

    expect(runtime.baseURL).toBe("http://localhost:4100");
    expect(runtime.port).toBe(4100);
    expect(runtime.reuseExistingServer).toBe(true);
    expect(runtime.zitadelRedirectUri).toBe("http://localhost:4100/api/auth/callback");
  });

  it("uses the declared Next launcher and Bazel output directories in Bazel mode", () => {
    const runtime = resolvePlaywrightRuntime({
      env: {
        GONGZZANG_BAZEL_PLAYWRIGHT: "1",
        PLAYWRIGHT_NEXT_CLI_PATH: "/workspace/apps/web/bazel/next-cli.mjs",
        PLAYWRIGHT_NODE_EXECUTABLE: "/bazel/node/bin/node",
        TEST_UNDECLARED_OUTPUTS_DIR: "/tmp/bazel-test-outputs",
      },
      defaultPort: DEFAULT_E2E_PLAYWRIGHT_PORT,
    });

    expect(runtime.command).toBe(
      "/bazel/node/bin/node /workspace/apps/web/bazel/next-cli.mjs dev -H 127.0.0.1 -p 3100",
    );
    expect(runtime.outputDir).toBe("/tmp/bazel-test-outputs/test-results");
    expect(runtime.reportDir).toBe("/tmp/bazel-test-outputs/playwright-report");
  });

  it("shell-quotes declared Bazel launcher paths before Playwright starts the server", () => {
    const runtime = resolvePlaywrightRuntime({
      env: {
        GONGZZANG_BAZEL_PLAYWRIGHT: "1",
        PLAYWRIGHT_NEXT_CLI_PATH: "/workspace/apps/web/bazel/next cli.mjs",
        PLAYWRIGHT_NODE_EXECUTABLE: "/bazel/node's/bin/node",
      },
      defaultPort: DEFAULT_E2E_PLAYWRIGHT_PORT,
    });

    expect(runtime.command).toBe(
      "'/bazel/node'\"'\"'s/bin/node' '/workspace/apps/web/bazel/next cli.mjs' dev -H 127.0.0.1 -p 3100",
    );
  });

  it("rejects invalid ports before Playwright can attach to a wrong server", () => {
    expect(() =>
      resolvePlaywrightRuntime({
        env: { PLAYWRIGHT_PORT: "3000; rm -rf ." },
        defaultPort: DEFAULT_E2E_PLAYWRIGHT_PORT,
      }),
    ).toThrow("PLAYWRIGHT_PORT must be an integer between 1 and 65535");
  });
});
