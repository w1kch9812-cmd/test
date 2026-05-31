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

  it("rejects invalid ports before Playwright can attach to a wrong server", () => {
    expect(() =>
      resolvePlaywrightRuntime({
        env: { PLAYWRIGHT_PORT: "3000; rm -rf ." },
        defaultPort: DEFAULT_E2E_PLAYWRIGHT_PORT,
      }),
    ).toThrow("PLAYWRIGHT_PORT must be an integer between 1 and 65535");
  });
});
