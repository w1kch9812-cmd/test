// @vitest-environment node

import { NextRequest } from "next/server";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@/lib/ratelimit", () => ({
  checkRate: vi.fn(),
}));

vi.mock("@/lib/session/store", () => ({
  getSession: vi.fn(),
}));

vi.mock("@/lib/map/vector-tile-manifest", () => ({
  resolveVectorTileAllowedOrigins: () => ["https://platform-core.example.com"],
}));

const { proxy } = await import("@/proxy");
const { checkRate } = await import("@/lib/ratelimit");
const checkRateMock = vi.mocked(checkRate);

describe("proxy platform-core receiver public access", () => {
  beforeEach(() => {
    checkRateMock.mockResolvedValue({ allowed: true, remaining: 99 });
  });

  it("allows /platform-core/events without sid", async () => {
    const req = new NextRequest("http://localhost:3000/platform-core/events", {
      method: "POST",
    });

    const res = await proxy(req);

    expect(res.status).toBe(200);
  });

  it("allows Gongzzang listing PBF marker tile proxy without sid", async () => {
    const req = new NextRequest(
      "http://localhost:3000/api/proxy/map/v1/marker-tiles/listing/14/8780/6345.pbf?filter_hash=all-active-v1",
    );

    const res = await proxy(req);

    expect(res.status).toBe(200);
    expect(checkRateMock).toHaveBeenCalledWith("public-map:listing-marker-tile:unknown", 600, 60);
  });

  it("rate limits Gongzzang listing PBF marker tile proxy before auth", async () => {
    checkRateMock.mockResolvedValueOnce({ allowed: false, remaining: 0 });
    const req = new NextRequest(
      "http://localhost:3000/api/proxy/map/v1/marker-tiles/listing/14/8780/6345.pbf?filter_hash=all-active-v1",
      { headers: { "x-forwarded-for": "203.0.113.10" } },
    );

    const res = await proxy(req);

    expect(res.status).toBe(429);
    expect(checkRateMock).toHaveBeenCalledWith(
      "public-map:listing-marker-tile:203.0.113.10",
      600,
      60,
    );
  });

  it("allows Gongzzang public marker state endpoints only through route policies", async () => {
    const requests = [
      new NextRequest(
        "http://localhost:3000/api/proxy/map/v1/marker-counts/listing?filter_hash=all-active-v1",
      ),
      new NextRequest("http://localhost:3000/api/proxy/map/v1/marker-filters/listing", {
        method: "POST",
      }),
      new NextRequest(
        "http://localhost:3000/api/proxy/map/v1/marker-masks/listing/14/8780/6345?filter_hash=all-active-v1&base_version=1",
      ),
    ];

    const responses = await Promise.all(requests.map((req) => proxy(req)));

    expect(responses.map((res) => res.status)).toEqual([200, 200, 200]);
    expect(checkRateMock).toHaveBeenCalledWith("public-map:listing-marker-count:unknown", 120, 60);
    expect(checkRateMock).toHaveBeenCalledWith("public-map:listing-marker-filter:unknown", 60, 60);
    expect(checkRateMock).toHaveBeenCalledWith("public-map:listing-marker-mask:unknown", 120, 60);
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

  it("allows platform-core vector tile manifest origin in production CSP", async () => {
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
