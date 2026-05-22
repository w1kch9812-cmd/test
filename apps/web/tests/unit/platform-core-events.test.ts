// @vitest-environment node

import { NextRequest } from "next/server";
import { beforeEach, describe, expect, it, vi } from "vitest";

const mockRevalidatePath = vi.fn();
const mockRevalidateTag = vi.fn();

vi.mock("next/cache", () => ({
  revalidatePath: (...args: unknown[]) => mockRevalidatePath(...args),
  revalidateTag: (...args: unknown[]) => mockRevalidateTag(...args),
}));

const { POST } = await import("@/app/platform-core/events/route");

const eventId = "0196f0b0-3e01-7000-8000-000000000001";
const eventType = "catalog.industrial_complex.gold_pointer.published.v1";
const scope = "catalog";

function eventBody(overrides: Record<string, unknown> = {}) {
  return {
    event_id: eventId,
    event_type: eventType,
    occurred_at: "2026-05-18T12:00:00Z",
    scope,
    payload: {
      type: eventType,
      schema_version: 1,
      complex_id: "018f0000-0000-7000-8000-000000000001",
      current_version: "gold/v1/industrial-complex/profile.json",
      source_snapshot_id: "bronze:molit-industrial-complex:2026-05-18T00:00:00Z",
      iceberg_snapshot_id: "987654321",
    },
    ...overrides,
  };
}

function makeRequest(body: unknown, headers: Record<string, string> = {}) {
  return new NextRequest("http://localhost:3000/platform-core/events", {
    method: "POST",
    headers: {
      "content-type": "application/json",
      "x-platform-core-event-id": eventId,
      "x-platform-core-event-type": eventType,
      "x-platform-core-outbox-scope": scope,
      ...headers,
    },
    body: JSON.stringify(body),
  });
}

describe("POST /platform-core/events", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("accepts a platform-core gold pointer event and invalidates catalog cache", async () => {
    const res = await POST(makeRequest(eventBody()));
    const json = await res.json();

    expect(res.status).toBe(202);
    expect(json).toEqual({
      event_id: eventId,
      effect: "invalidate_catalog_cache",
      status: "accepted",
    });
    expect(mockRevalidatePath).toHaveBeenCalledWith("/listings", "page");
    expect(mockRevalidatePath).not.toHaveBeenCalledWith("/dev-x9-test", "page");
    expect(mockRevalidateTag).toHaveBeenCalledWith("platform-core-catalog", { expire: 0 });
  });

  it("rejects events whose required header values do not match the body", async () => {
    const res = await POST(
      makeRequest(eventBody(), {
        "x-platform-core-event-id": "0196f0b0-3e01-7000-8000-000000000999",
      }),
    );
    const json = await res.json();

    expect(res.status).toBe(400);
    expect(json.status).toBe("rejected");
    expect(mockRevalidatePath).not.toHaveBeenCalled();
    expect(mockRevalidateTag).not.toHaveBeenCalled();
  });
});
