import { describe, expect, it } from "vitest";

import e2eConfig from "../../playwright.config";
import probesConfig from "../../playwright.probes.config";

function singleWebServer(config: unknown): Record<string, unknown> {
  const value = (config as { webServer?: unknown }).webServer;
  if (Array.isArray(value)) {
    throw new Error("expected a single Playwright webServer config");
  }
  if (typeof value !== "object" || value === null) {
    throw new Error("expected Playwright webServer config");
  }
  return value as Record<string, unknown>;
}

describe("playwright config", () => {
  it("runs e2e against the managed 3100 endpoint without implicit reuse", () => {
    expect((e2eConfig as { use?: { baseURL?: string } }).use?.baseURL).toBe(
      "http://127.0.0.1:3100",
    );

    const webServer = singleWebServer(e2eConfig);
    expect(webServer.url).toBe("http://127.0.0.1:3100");
    expect(webServer.reuseExistingServer).toBe(false);
    expect(webServer.command).toBe("pnpm dev -H 127.0.0.1 -p 3100");
  });

  it("runs probes on a separate managed endpoint", () => {
    expect((probesConfig as { use?: { baseURL?: string } }).use?.baseURL).toBe(
      "http://127.0.0.1:3101",
    );

    const webServer = singleWebServer(probesConfig);
    expect(webServer.url).toBe("http://127.0.0.1:3101");
    expect(webServer.reuseExistingServer).toBe(false);
    expect(webServer.command).toBe("pnpm dev -H 127.0.0.1 -p 3101");
  });
});
