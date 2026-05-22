// @vitest-environment node

import { NextRequest } from "next/server";
import { describe, expect, it, vi } from "vitest";

vi.mock("@/lib/ratelimit", () => ({
  checkRate: vi.fn(),
}));

vi.mock("@/lib/session/store", () => ({
  getSession: vi.fn(),
}));

vi.mock("@/lib/map/vector-tile-manifest", () => ({
  resolveVectorTileAllowedOrigins: () => [],
}));

vi.mock("@/lib/map/marker-tile-contract", () => ({
  resolveMarkerTileAllowedOrigins: () => ["https://platform-core.example.com"],
}));

const { proxy } = await import("@/proxy");

describe("proxy platform-core receiver public access", () => {
  it("allows /platform-core/events without sid", async () => {
    const req = new NextRequest("http://localhost:3000/platform-core/events", {
      method: "POST",
    });

    const res = await proxy(req);

    expect(res.status).toBe(200);
  });

  it("allows Gongzzang listing PBF marker tile proxy without sid", async () => {
    const req = new NextRequest(
      "http://localhost:3000/api/proxy/map/v1/marker-tiles/listing/0/0/0.pbf?filter_hash=all-active-v1",
    );

    const res = await proxy(req);

    expect(res.status).toBe(200);
  });

  it("does not expose dev-x9-test in production", async () => {
    vi.stubEnv("NODE_ENV", "production");
    try {
      const req = new NextRequest("https://gongzzang.com/dev-x9-test");

      const res = await proxy(req);

      expect(res.status).toBe(404);
    } finally {
      vi.unstubAllEnvs();
    }
  });

  it("allows platform-core PBF marker tile origin in production CSP", async () => {
    vi.stubEnv("NODE_ENV", "production");
    try {
      const req = new NextRequest("http://localhost:3000/login");

      const res = await proxy(req);

      expect(res.headers.get("content-security-policy")).toContain(
        "https://platform-core.example.com",
      );
    } finally {
      vi.unstubAllEnvs();
    }
  });

  it("allows Naver HTTP resources only for local production preview CSP", async () => {
    vi.stubEnv("NODE_ENV", "production");
    try {
      const localReq = new NextRequest("http://localhost:3900/login");
      const localRes = await proxy(localReq);
      const localCsp = localRes.headers.get("content-security-policy") ?? "";

      expect(cspDirective(localCsp, "script-src")).toContain(" http: https:");
      expect(cspDirective(localCsp, "img-src")).toContain(" http: https:");
      expect(cspDirective(localCsp, "connect-src")).toContain(" http: https:");

      const productionReq = new NextRequest("https://gongzzang.com/login");
      const productionRes = await proxy(productionReq);
      const productionCsp = productionRes.headers.get("content-security-policy") ?? "";

      expect(cspDirective(productionCsp, "script-src")).not.toContain(" http:");
      expect(cspDirective(productionCsp, "img-src")).not.toContain(" http:");
    } finally {
      vi.unstubAllEnvs();
    }
  });
});

function cspDirective(csp: string, name: string): string {
  return (
    csp
      .split(";")
      .map((part) => part.trim())
      .find((part) => part.startsWith(name)) ?? ""
  );
}
