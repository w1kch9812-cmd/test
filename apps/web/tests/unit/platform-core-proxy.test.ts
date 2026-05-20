// @vitest-environment node

import { NextRequest } from "next/server";
import { describe, expect, it, vi } from "vitest";

vi.mock("@/lib/ratelimit", () => ({
  checkRate: vi.fn(),
}));

vi.mock("@/lib/session/store", () => ({
  getSession: vi.fn(),
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
});
