# Platform Core Gongzzang Event Receiver Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor Gongzzang's `/platform-core/events` receiver into an explicit event registry and add the Platform Core PNU anchor snapshot event contract without changing the database schema.

**Architecture:** Keep one public receiver route, parse a shared envelope, validate headers once, then dispatch to per-event handlers. Existing industrial-complex cache invalidation remains unchanged. The new anchor snapshot event is accepted and acknowledged as an import enqueue contract, while actual durable import/backfill remains a later DB-approved slice.

**Tech Stack:** Next.js route handler, TypeScript strict mode, Zod, Vitest, NextRequest.

---

## File Structure

- Modify: `apps/web/app/platform-core/events/route.ts`
  - Owns the receiver route, shared header validation, event schema registry, and event effects.
- Modify: `apps/web/tests/unit/platform-core-events.test.ts`
  - Covers old behavior, new anchor event ack, unsupported event rejection, and header/body mismatch.

No migration is created in this plan.

## Task 1: Add Receiver Contract Tests

**Files:**
- Modify: `apps/web/tests/unit/platform-core-events.test.ts`

- [ ] **Step 1: Add failing tests for anchor event support and unsupported events**

Add an anchor event fixture and two tests:

```ts
const anchorEventType = "catalog.parcel_marker_anchor.snapshot.published.v1";

function anchorEventBody(overrides: Record<string, unknown> = {}) {
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
    },
    ...overrides,
  };
}
```

Expected new behavior:

```ts
it("accepts a platform-core parcel anchor snapshot event without invalidating listing page cache", async () => {
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

  expect(res.status).toBe(400);
  expect(json).toEqual({ reason: "unsupported_event_type", status: "rejected" });
  expect(mockRevalidatePath).not.toHaveBeenCalled();
  expect(mockRevalidateTag).not.toHaveBeenCalled();
});
```

- [ ] **Step 2: Run the focused test and verify RED**

Run:

```powershell
pnpm --filter @gongzzang/web test -- tests/unit/platform-core-events.test.ts
```

Expected: the anchor event test fails with `invalid_event` or a non-202 status because the route only accepts the industrial-complex gold pointer event.

## Task 2: Refactor Receiver Into A Registry

**Files:**
- Modify: `apps/web/app/platform-core/events/route.ts`

- [ ] **Step 1: Replace single-event parsing with shared envelope parsing and event registry**

Implement these constants:

```ts
const GOLD_POINTER_EVENT_TYPE = "catalog.industrial_complex.gold_pointer.published.v1";
const PARCEL_ANCHOR_SNAPSHOT_EVENT_TYPE = "catalog.parcel_marker_anchor.snapshot.published.v1";
const PLATFORM_CORE_SCOPE = "catalog";
const CATALOG_CACHE_TAG = "platform-core-catalog";
```

Add a shared envelope schema:

```ts
const PlatformCoreEventEnvelopeSchema = z
  .object({
    event_id: z.string().uuid(),
    event_type: z.string().min(1),
    occurred_at: z.string().datetime({ offset: true }),
    scope: z.literal(PLATFORM_CORE_SCOPE),
    payload: z.object({ type: z.string().min(1) }).passthrough(),
  })
  .passthrough();
```

Keep the gold pointer payload schema, and add:

```ts
const ParcelAnchorSnapshotEventSchema = PlatformCoreEventEnvelopeSchema.extend({
  event_type: z.literal(PARCEL_ANCHOR_SNAPSHOT_EVENT_TYPE),
  payload: z
    .object({
      type: z.literal(PARCEL_ANCHOR_SNAPSHOT_EVENT_TYPE),
      schema_version: z.number().int().min(1),
      anchor_snapshot_id: z.string().min(1),
      source_geometry_version: z.string().min(1),
      artifact_manifest_url: z.string().url(),
      artifact_checksum_sha256: z.string().regex(/^[a-f0-9]{64}$/),
      row_count: z.number().int().nonnegative(),
      published_at: z.string().datetime({ offset: true }),
    })
    .passthrough(),
});
```

Dispatch with an explicit handler map:

```ts
const EVENT_HANDLERS = {
  [GOLD_POINTER_EVENT_TYPE]: handleGoldPointerEvent,
  [PARCEL_ANCHOR_SNAPSHOT_EVENT_TYPE]: handleParcelAnchorSnapshotEvent,
} satisfies Record<string, EventHandler>;
```

The anchor handler returns:

```ts
{
  event_id: event.event_id,
  effect: "enqueue_anchor_projection_import",
  status: "accepted",
}
```

The handler must not call `revalidatePath` or `revalidateTag`.

- [ ] **Step 2: Run the focused test and verify GREEN**

Run:

```powershell
pnpm --filter @gongzzang/web test -- tests/unit/platform-core-events.test.ts
```

Expected: all tests in `platform-core-events.test.ts` pass.

## Task 3: Run Focused Contract Verification

**Files:**
- Verify only.

- [ ] **Step 1: Run platform-core related web tests**

Run:

```powershell
pnpm --filter @gongzzang/web test -- tests/unit/platform-core-events.test.ts tests/unit/platform-core-proxy.test.ts tests/unit/map/vector-tile-manifest.test.ts tests/unit/map/marker-tile-style.test.ts
```

Expected: all tests pass, except live manifest tests remain skipped unless `PLATFORM_CORE_MANIFEST_LIVE_BASE_URL` is set.

- [ ] **Step 2: Run PNU marker guardrail**

Run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\check-pnu-anchor-pbf-marker-contract -Root .
```

Expected:

```text
pnu-anchor-pbf-marker-contract-ok
```

## Self-Review

- Spec coverage: Covers Phase 1 from the design doc: receiver registry, existing invalidation behavior, anchor snapshot event support, no DB schema change.
- Placeholder scan: No placeholder implementation steps are left.
- Type consistency: Event names, ack `effect`, and route path match the design doc.
