// @vitest-environment node

import { NextRequest } from "next/server";
import { beforeEach, describe, expect, it, vi } from "vitest";

const { apiMock, checkRateMock, getSessionMock } = vi.hoisted(() => ({
  apiMock: vi.fn(),
  checkRateMock: vi.fn(),
  getSessionMock: vi.fn(),
}));

vi.mock("next-intl/server", () => ({
  getTranslations: vi.fn(async () => (key: string) => key),
}));

vi.mock("@/lib/api", () => ({
  createServerApi: vi.fn(() => apiMock),
}));

vi.mock("@/lib/session/store", () => ({
  getSession: getSessionMock,
}));

vi.mock("@/lib/ratelimit", () => ({
  checkRate: checkRateMock,
}));

const { GET, POST } = await import("@/app/api/proxy/[...path]/route");

function session(role = "Buyer") {
  return {
    sub: "user_1",
    jti: "jti_1",
    role,
    access_token: "access-token-1",
    refresh_token: "refresh-token-1",
    id_token: "id-token-1",
    exp: 4_102_444_800,
  };
}

describe("api proxy route", () => {
  beforeEach(() => {
    apiMock.mockReset();
    checkRateMock.mockReset();
    checkRateMock.mockResolvedValue({ allowed: true, remaining: 99 });
    getSessionMock.mockReset();
    getSessionMock.mockResolvedValue(null);
  });

  it("rejects unregistered backend proxy paths before upstream call", async () => {
    const response = await GET(
      new NextRequest("http://localhost:3000/api/proxy/internal/raw-listing-export"),
      {
        params: Promise.resolve({
          path: ["internal", "raw-listing-export"],
        }),
      },
    );

    expect(response.status).toBe(404);
    expect(await response.json()).toMatchObject({
      type: "https://gongzzang.com/errors/proxy/route-not-allowed",
      status: 404,
    });
    expect(apiMock).not.toHaveBeenCalled();
  });

  it("preserves Mapbox vector tile responses as binary", async () => {
    const bytes = new Uint8Array([0x1a, 0x02, 0x78, 0x79]);
    apiMock.mockResolvedValueOnce(
      new Response(bytes, {
        status: 200,
        headers: {
          "content-type": "application/vnd.mapbox-vector-tile",
          "x-request-id": "req_BINARYTILE00000000000000",
        },
      }),
    );

    const response = await GET(
      new NextRequest(
        "http://localhost:3000/api/proxy/map/v1/marker-tiles/listing/14/8780/6345.pbf?filter_hash=all-active-v1",
      ),
      {
        params: Promise.resolve({
          path: ["map", "v1", "marker-tiles", "listing", "14", "8780", "6345.pbf"],
        }),
      },
    );

    expect(response.status).toBe(200);
    expect(response.headers.get("content-type")).toBe("application/vnd.mapbox-vector-tile");
    expect(response.headers.get("x-request-id")).toBe("req_BINARYTILE00000000000000");
    expect(new Uint8Array(await response.arrayBuffer())).toEqual(bytes);
    expect(apiMock).toHaveBeenCalledWith(
      "map/v1/marker-tiles/listing/14/8780/6345.pbf",
      expect.objectContaining({
        searchParams: { filter_hash: "all-active-v1" },
      }),
    );
  });

  it("preserves image responses as binary", async () => {
    getSessionMock.mockResolvedValueOnce(session());
    const bytes = new Uint8Array([0xff, 0xd8, 0xff, 0xdb]);
    apiMock.mockResolvedValueOnce(
      new Response(bytes, {
        status: 200,
        headers: {
          "content-type": "image/jpeg",
          "x-request-id": "req_BINARYIMAGE0000000000000",
        },
      }),
    );

    const response = await GET(
      new NextRequest("http://localhost:3000/api/proxy/listings/lst_1/photos/lph_1", {
        headers: { cookie: "sid=sid_1" },
      }),
      {
        params: Promise.resolve({
          path: ["listings", "lst_1", "photos", "lph_1"],
        }),
      },
    );

    expect(response.status).toBe(200);
    expect(response.headers.get("content-type")).toBe("image/jpeg");
    expect(new Uint8Array(await response.arrayBuffer())).toEqual(bytes);
    expect(apiMock).toHaveBeenCalledWith(
      "listings/lst_1/photos/lph_1",
      expect.objectContaining({
        headers: expect.objectContaining({
          Authorization: "Bearer access-token-1",
        }),
      }),
    );
  });

  it("rejects authenticated proxy paths without a valid session", async () => {
    const response = await GET(
      new NextRequest("http://localhost:3000/api/proxy/listings/lst_private_1/photos/lph_1"),
      {
        params: Promise.resolve({
          path: ["listings", "lst_private_1", "photos", "lph_1"],
        }),
      },
    );

    expect(response.status).toBe(401);
    expect(await response.json()).toMatchObject({
      type: "https://gongzzang.com/errors/proxy/session-required",
      status: 401,
    });
    expect(apiMock).not.toHaveBeenCalled();
  });

  it("rate limits authenticated proxy paths from the generated route policy", async () => {
    getSessionMock.mockResolvedValueOnce(session());
    checkRateMock.mockResolvedValueOnce({ allowed: false, remaining: 0 });

    const response = await GET(
      new NextRequest("http://localhost:3000/api/proxy/listings", {
        headers: {
          cookie: "sid=sid_1",
          "x-forwarded-for": "2.2.2.2",
        },
      }),
      {
        params: Promise.resolve({
          path: ["listings"],
        }),
      },
    );

    expect(response.status).toBe(429);
    expect(await response.json()).toMatchObject({
      type: "https://gongzzang.com/errors/proxy/too-many-requests",
      status: 429,
    });
    expect(checkRateMock).toHaveBeenCalledWith("api-proxy:authenticated-read:user_1", 240, 60);
    expect(apiMock).not.toHaveBeenCalled();
  });

  it("rejects privileged proxy paths for non-privileged roles", async () => {
    getSessionMock.mockResolvedValueOnce(session("Buyer"));

    const response = await POST(
      new NextRequest("http://localhost:3000/api/proxy/listings", {
        method: "POST",
        headers: { cookie: "sid=sid_1" },
      }),
      {
        params: Promise.resolve({
          path: ["listings"],
        }),
      },
    );

    expect(response.status).toBe(403);
    expect(await response.json()).toMatchObject({
      type: "https://gongzzang.com/errors/proxy/insufficient-role",
      status: 403,
    });
    expect(apiMock).not.toHaveBeenCalled();
  });

  it("allows privileged proxy paths for broker role", async () => {
    getSessionMock.mockResolvedValueOnce(session("Broker"));
    apiMock.mockResolvedValueOnce(
      new Response(JSON.stringify({ id: "lst_1" }), {
        status: 201,
        headers: { "content-type": "application/json" },
      }),
    );

    const response = await POST(
      new NextRequest("http://localhost:3000/api/proxy/listings", {
        method: "POST",
        headers: { cookie: "sid=sid_1" },
        body: JSON.stringify({ title: "listing" }),
      }),
      {
        params: Promise.resolve({
          path: ["listings"],
        }),
      },
    );

    expect(response.status).toBe(201);
    expect(apiMock).toHaveBeenCalledWith(
      "listings",
      expect.objectContaining({
        headers: expect.objectContaining({
          Authorization: "Bearer access-token-1",
        }),
      }),
    );
  });
});
