// @vitest-environment node

import { NextRequest } from "next/server";
import { describe, expect, it, vi } from "vitest";

const { apiMock } = vi.hoisted(() => ({
  apiMock: vi.fn(),
}));

vi.mock("next-intl/server", () => ({
  getTranslations: vi.fn(async () => (key: string) => key),
}));

vi.mock("@/lib/api", () => ({
  createServerApi: vi.fn(() => apiMock),
}));

vi.mock("@/lib/session/store", () => ({
  getSession: vi.fn(async () => null),
}));

const { GET } = await import("@/app/api/proxy/[...path]/route");

describe("api proxy route", () => {
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
        "http://localhost:3000/api/proxy/map/v1/marker-tiles/listing/0/0/0.pbf?filter_hash=all-active-v1",
      ),
      {
        params: Promise.resolve({
          path: ["map", "v1", "marker-tiles", "listing", "0", "0", "0.pbf"],
        }),
      },
    );

    expect(response.status).toBe(200);
    expect(response.headers.get("content-type")).toBe("application/vnd.mapbox-vector-tile");
    expect(response.headers.get("x-request-id")).toBe("req_BINARYTILE00000000000000");
    expect(new Uint8Array(await response.arrayBuffer())).toEqual(bytes);
    expect(apiMock).toHaveBeenCalledWith(
      "map/v1/marker-tiles/listing/0/0/0.pbf",
      expect.objectContaining({
        searchParams: { filter_hash: "all-active-v1" },
      }),
    );
  });
});
