// @vitest-environment node

import { createHmac } from "node:crypto";
import { NextRequest } from "next/server";
import { afterAll, beforeEach, describe, expect, it, vi } from "vitest";
import { getPlatformCoreEventInboxRecord } from "@/lib/platform-core/event-inbox";
import { __resetRedisForTest, getRedis } from "@/lib/session/redis";

const mockRevalidatePath = vi.fn();
const mockRevalidateTag = vi.fn();

vi.mock("next/cache", () => ({
  revalidatePath: (...args: unknown[]) => mockRevalidatePath(...args),
  revalidateTag: (...args: unknown[]) => mockRevalidateTag(...args),
}));

const webhookSecret = "test-platform-core-webhook-secret-32-valid";
process.env.PLATFORM_CORE_WEBHOOK_SECRET = webhookSecret;

const { POST } = await import("@/app/platform-core/events/route");

const eventId = "0196f0b0-3e01-7000-8000-000000000001";
const eventType = "catalog.industrial_complex.gold_pointer.published.v1";
const anchorEventType = "catalog.parcel_marker_anchor.snapshot.published.v1";
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

function anchorEventBody(
  overrides: Record<string, unknown> = {},
  payloadOverrides: Record<string, unknown> = {},
) {
  return {
    event_id: eventId,
    event_type: anchorEventType,
    occurred_at: "2026-05-28T12:00:00Z",
    scope,
    payload: {
      type: anchorEventType,
      schema_version: 1,
      anchor_snapshot_id: "anchor-snapshot-20260528T120000Z",
      source_geometry_version: "silver.parcel_boundaries@20260528",
      artifact_manifest_url: "https://platform-core.example.com/artifacts/anchor-snapshot.json",
      artifact_checksum_sha256: "a".repeat(64),
      row_count: 1,
      published_at: "2026-05-28T12:00:00Z",
      ...payloadOverrides,
    },
    ...overrides,
  };
}

function makeRequest(body: unknown, headers: Record<string, string> = {}) {
  const bodyText = JSON.stringify(body);
  const timestamp = Math.floor(Date.now() / 1000).toString();
  return new NextRequest("http://localhost:3000/platform-core/events", {
    method: "POST",
    headers: {
      "content-type": "application/json",
      "x-platform-core-event-id": eventId,
      "x-platform-core-event-type": eventType,
      "x-platform-core-outbox-scope": scope,
      "x-platform-core-signature": signBody(timestamp, bodyText),
      "x-platform-core-timestamp": timestamp,
      ...headers,
    },
    body: bodyText,
  });
}

function makeUnsignedRequest(body: unknown, headers: Record<string, string> = {}) {
  const request = makeRequest(body, headers);
  request.headers.delete("x-platform-core-signature");
  return request;
}

function signBody(timestamp: string, bodyText: string) {
  return `v1=${createHmac("sha256", webhookSecret).update(`${timestamp}.${bodyText}`).digest("hex")}`;
}

describe("POST /platform-core/events", () => {
  beforeEach(async () => {
    vi.clearAllMocks();
    vi.unstubAllGlobals();
    await getRedis().select(5);
    await getRedis().flushdb();
  });

  afterAll(() => __resetRedisForTest());

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

  it("acknowledges duplicate platform-core events without applying side effects twice", async () => {
    const first = await POST(makeRequest(eventBody()));
    const second = await POST(makeRequest(eventBody()));
    const json = await second.json();

    expect(first.status).toBe(202);
    expect(second.status).toBe(200);
    expect(json).toEqual({
      event_id: eventId,
      effect: "invalidate_catalog_cache",
      status: "duplicate",
    });
    expect(mockRevalidatePath).toHaveBeenCalledTimes(1);
    expect(mockRevalidateTag).toHaveBeenCalledTimes(1);
  });

  it("acknowledges duplicate platform-core event bursts without repeated side effects", async () => {
    const first = await POST(makeRequest(eventBody()));
    const duplicates = await Promise.all(
      Array.from({ length: 25 }, () => POST(makeRequest(eventBody()))),
    );

    expect(first.status).toBe(202);
    expect(duplicates.map((response) => response.status)).toEqual(Array(25).fill(200));
    expect(mockRevalidatePath).toHaveBeenCalledTimes(1);
    expect(mockRevalidateTag).toHaveBeenCalledTimes(1);
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

  it("rejects platform-core events without a valid webhook signature", async () => {
    const res = await POST(makeUnsignedRequest(eventBody()));
    const json = await res.json();

    expect(res.status).toBe(401);
    expect(json).toEqual({ reason: "invalid_signature", status: "rejected" });
    expect(mockRevalidatePath).not.toHaveBeenCalled();
    expect(mockRevalidateTag).not.toHaveBeenCalled();
  });

  it("rejects platform-core events with stale webhook signatures", async () => {
    const body = eventBody();
    const bodyText = JSON.stringify(body);
    const staleTimestamp = (Math.floor(Date.now() / 1000) - 600).toString();
    const res = await POST(
      new NextRequest("http://localhost:3000/platform-core/events", {
        method: "POST",
        headers: {
          "content-type": "application/json",
          "x-platform-core-event-id": eventId,
          "x-platform-core-event-type": eventType,
          "x-platform-core-outbox-scope": scope,
          "x-platform-core-signature": signBody(staleTimestamp, bodyText),
          "x-platform-core-timestamp": staleTimestamp,
        },
        body: bodyText,
      }),
    );
    const json = await res.json();

    expect(res.status).toBe(401);
    expect(json).toEqual({ reason: "invalid_signature", status: "rejected" });
    expect(mockRevalidatePath).not.toHaveBeenCalled();
    expect(mockRevalidateTag).not.toHaveBeenCalled();
  });

  it("accepts a platform-core parcel anchor snapshot event without invalidating listing page cache", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(
        new Response(JSON.stringify({ status: "accepted" }), {
          headers: { "content-type": "application/json" },
          status: 202,
        }),
      ),
    );

    const res = await POST(
      makeRequest(anchorEventBody(), {
        "x-platform-core-event-type": anchorEventType,
      }),
    );
    const json = await res.json();

    expect(res.status).toBe(202);
    expect(json).toEqual({
      event_id: eventId,
      effect: "enqueue_anchor_projection_import",
      status: "accepted",
    });
    expect(mockRevalidatePath).not.toHaveBeenCalled();
    expect(mockRevalidateTag).not.toHaveBeenCalled();
  });

  it("returns retryable failure when Rust API inbox write fails for anchor events", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(
        new Response(JSON.stringify({ reason: "inbox_write_failed", status: "rejected" }), {
          headers: { "content-type": "application/json" },
          status: 500,
        }),
      ),
    );

    const res = await POST(
      makeRequest(anchorEventBody(), {
        "x-platform-core-event-type": anchorEventType,
      }),
    );
    const json = await res.json();
    const inbox = await getPlatformCoreEventInboxRecord(eventId);

    expect(res.status).toBe(503);
    expect(json).toEqual({ reason: "durable_inbox_unavailable", status: "rejected" });
    expect(inbox).toBeUndefined();
  });

  it("rejects parcel anchor snapshot events with non-HTTPS artifact manifest URLs", async () => {
    const res = await POST(
      makeRequest(
        anchorEventBody(
          {},
          { artifact_manifest_url: "http://platform-core.example.com/anchors.json" },
        ),
        {
          "x-platform-core-event-type": anchorEventType,
        },
      ),
    );
    const json = await res.json();
    const inbox = await getPlatformCoreEventInboxRecord(eventId);

    expect(res.status).toBe(400);
    expect(json).toEqual({ reason: "invalid_event", status: "rejected" });
    expect(inbox).toMatchObject({
      event_id: eventId,
      event_type: anchorEventType,
      reason: "invalid_event",
      status: "dead_letter",
    });
    expect(mockRevalidatePath).not.toHaveBeenCalled();
    expect(mockRevalidateTag).not.toHaveBeenCalled();
  });

  it("rejects unsupported platform-core event types", async () => {
    const unsupportedType = "catalog.unsupported.v1";
    const res = await POST(
      makeRequest(
        {
          event_id: eventId,
          event_type: unsupportedType,
          occurred_at: "2026-05-28T12:00:00Z",
          scope,
          payload: { type: unsupportedType },
        },
        {
          "x-platform-core-event-type": unsupportedType,
        },
      ),
    );
    const json = await res.json();
    const inbox = await getPlatformCoreEventInboxRecord(eventId);

    expect(res.status).toBe(400);
    expect(json).toEqual({ reason: "unsupported_event_type", status: "rejected" });
    expect(inbox).toMatchObject({
      event_id: eventId,
      event_type: unsupportedType,
      reason: "unsupported_event_type",
      status: "dead_letter",
    });
    expect(mockRevalidatePath).not.toHaveBeenCalled();
    expect(mockRevalidateTag).not.toHaveBeenCalled();
  });
});
