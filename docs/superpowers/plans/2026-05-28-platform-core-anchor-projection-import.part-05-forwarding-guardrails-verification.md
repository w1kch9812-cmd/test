# Platform Core Anchor Projection Import Plan - Part 05: Forwarding, Guardrails, And Verification

Parent index: [Platform Core Anchor Projection Import Implementation Plan](./2026-05-28-platform-core-anchor-projection-import.md).


## Task 5: Next.js Public Receiver Forwarding

**Files:**
- Modify: `apps/web/app/platform-core/events/route.ts`
- Modify: `apps/web/tests/unit/platform-core-events.test.ts`

- [ ] **Step 1: Write failing forwarding tests**

In `apps/web/tests/unit/platform-core-events.test.ts`, mock `global.fetch` and add:

```ts
it("returns retryable failure when Rust API inbox write fails for anchor events", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ reason: "inbox_write_failed", status: "rejected" }), {
        status: 500,
        headers: { "content-type": "application/json" },
      }),
    ),
  );

  const res = await POST(
    makeRequest(anchorEventBody(), {
      "x-platform-core-event-type": anchorEventType,
    }),
  );
  const json = await res.json();

  expect(res.status).toBe(503);
  expect(json).toEqual({ reason: "durable_inbox_unavailable", status: "rejected" });
});
```

- [ ] **Step 2: Run the focused test and verify RED**

Run:

```powershell
pnpm --filter @gongzzang/web test -- tests/unit/platform-core-events.test.ts
```

Expected: new test fails because the route currently acknowledges anchor events without forwarding.

- [ ] **Step 3: Forward accepted events to Rust API**

In `apps/web/app/platform-core/events/route.ts`, import `env`:

```ts
import { env } from "@/lib/env";
```

Add:

```ts
async function persistPlatformCoreEvent(event: PlatformCoreEventEnvelope): Promise<boolean> {
  const res = await fetch(`${env.NEXT_PUBLIC_API_BASE_URL}/internal/platform-core/events`, {
    method: "POST",
    headers: {
      "content-type": "application/json",
      "x-internal-auth": env.INTERNAL_AUTH_SECRET,
    },
    body: JSON.stringify(event),
  });
  return res.ok;
}
```

In `handleParcelAnchorSnapshotEvent`, persist before returning the ack:

```ts
async function handleParcelAnchorSnapshotEvent(
  value: unknown,
): Promise<AcceptedResponse | undefined | "durable_inbox_unavailable"> {
  const parsed = ParcelAnchorSnapshotEventSchema.safeParse(value);
  if (!parsed.success) return undefined;

  const persisted = await persistPlatformCoreEvent(parsed.data);
  if (!persisted) return "durable_inbox_unavailable";

  return accepted(parsed.data, "enqueue_anchor_projection_import");
}
```

Change `EventHandler` and `POST` to await async handlers and return status 503 for `"durable_inbox_unavailable"`.

- [ ] **Step 4: Run tests and verify GREEN**

Run:

```powershell
pnpm --filter @gongzzang/web test -- tests/unit/platform-core-events.test.ts
```

Expected: all receiver tests pass.

## Task 6: Boundary Guardrails

**Files:**
- Modify: `docs/architecture/platform-core-boundary.v1.json`
- Modify: `scripts/ci/check-platform-core-boundary`
- Modify: `scripts/ci/check-platform-core-boundary.tests`

- [ ] **Step 1: Add failing boundary test fixtures**

In `scripts/ci/check-platform-core-boundary.tests`, add required ownership entries for:

```json
{"path":"migrations/30016_platform_core_event_inbox_anchor_import.sql","owner":"gongzzang","classification":"platform_core_event_inbox"},
{"path":"crates/db/src/platform_core_anchor.rs","owner":"gongzzang","classification":"platform_core_read_model_import"},
{"path":"services/api/src/routes/platform_core_events.rs","owner":"gongzzang","classification":"platform_core_event_receiver"},
{"path":"services/api/src/bin/platform_core_anchor_import.rs","owner":"gongzzang","classification":"platform_core_read_model_importer"}
```

- [ ] **Step 2: Run boundary tests and verify RED**

Run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\check-platform-core-boundary.tests
```

Expected: fail because checker does not require the new entries.

- [ ] **Step 3: Update boundary SSOT and checker**

Add the same entries to `docs/architecture/platform-core-boundary.v1.json` and `$RequiredPathOwnership` in `scripts/ci/check-platform-core-boundary`.

- [ ] **Step 4: Run boundary tests and verify GREEN**

Run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\check-platform-core-boundary.tests
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\check-platform-core-boundary -Root .
```

Expected:

```text
check-platform-core-boundary-tests-ok
platform-core-boundary-ok
```

## Task 7: Focused Verification

**Files:**
- Verify only.

- [ ] **Step 1: Run Gongzzang web receiver tests**

Run:

```powershell
pnpm --filter @gongzzang/web test -- tests/unit/platform-core-events.test.ts tests/unit/platform-core-proxy.test.ts tests/unit/map/vector-tile-manifest.test.ts tests/unit/map/marker-tile-style.test.ts
```

Expected: all non-live tests pass; live manifest test remains skipped unless `PLATFORM_CORE_MANIFEST_LIVE_BASE_URL` is set.

- [ ] **Step 2: Run web typecheck**

Run:

```powershell
pnpm --filter @gongzzang/web typecheck
```

Expected: `tsc --noEmit` exits 0.

- [ ] **Step 3: Run Rust focused tests**

Run:

```powershell
cargo test -p api platform_core_events
cargo test -p api platform_core_anchor_import
cargo test -p db --features integration --test platform_core_anchor_import_integration
```

Expected: all tests pass when `DATABASE_URL` points at a migrated PostGIS database.

- [ ] **Step 4: Run guardrails**

Run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\check-platform-core-boundary.tests
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\check-platform-core-boundary -Root .
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\check-pnu-anchor-pbf-marker-contract.tests
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\check-pnu-anchor-pbf-marker-contract -Root .
```

Expected: all four commands exit 0.

- [ ] **Step 5: Run diff whitespace checks**

Run:

```powershell
git diff --check -- migrations/30016_platform_core_event_inbox_anchor_import.sql crates/db/src/platform_core_anchor.rs crates/db/src/lib.rs crates/db/tests/platform_core_anchor_import_integration.rs services/api/src/routes/platform_core_events.rs services/api/src/main.rs services/api/src/bin/platform_core_anchor_import.rs services/api/src/platform_core_anchor_import.rs services/api/Cargo.toml apps/web/app/platform-core/events/route.ts apps/web/tests/unit/platform-core-events.test.ts docs/architecture/platform-core-boundary.v1.json scripts/ci/check-platform-core-boundary scripts/ci/check-platform-core-boundary.tests scripts/ci/check-pnu-anchor-pbf-marker-contract scripts/ci/check-pnu-anchor-pbf-marker-contract.tests
```

Expected: no output and exit 0.

## Self-Review

- Spec coverage: Covers durable event idempotency, Platform Core anchor artifact import, checksum/row validation entry points, read-model upsert, listing projection refresh, receiver forwarding, and guardrails.
- Approval constraints: Migration creation is explicitly blocked until user DB approval. The plan does not require new external package versions.
- Boundary consistency: Platform Core owns anchor coordinates and artifact publication; Gongzzang owns listing projection and listing marker tiles. No Platform Core database access is introduced.
