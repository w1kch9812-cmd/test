# Gongzzang Marker Data Plane SSS Plan - Part 04: Guardrails, Frontend, And Release Gate

Parent index: [Gongzzang Marker Data Plane SSS Implementation Plan](./2026-05-31-gongzzang-marker-data-plane-sss.md).


## Task 8: Update SSOT Registries And Guardrails

**Files:**

- Modify: `docs/architecture/traffic-auth-policy-registry.v1.json`
- Modify: `docs/architecture/platform-integration/route-exposure-policy.v1.json`
- Modify: `scripts/ci/check-pnu-anchor-pbf-marker-contract`
- Test: `scripts/ci/check-pnu-anchor-pbf-marker-contract.tests`

- [ ] **Step 1: Add route policies**

Add public derived route policies:

```json
{
  "id": "gongzzang.public_map.listing_marker_delta",
  "owner": "gongzzang",
  "backend_route": "/map/v1/marker-deltas/listing/{z}/{x}/{y}.pbf",
  "methods": ["GET"],
  "auth_policy": { "method": "anonymous_public", "session_required": false },
  "data_exposure_policy": {
    "exposure_class": "public_derived",
    "allowed_data_classes": ["derived_marker_tile"],
    "forbidden_data_classes": ["private_listing", "business_verified_listing_detail", "contact_data"]
  }
}
```

```json
{
  "id": "gongzzang.public_map.listing_marker_tombstone",
  "owner": "gongzzang",
  "backend_route": "/map/v1/marker-tombstones/listing/{z}/{x}/{y}",
  "methods": ["GET"],
  "auth_policy": { "method": "anonymous_public", "session_required": false },
  "data_exposure_policy": {
    "exposure_class": "public_derived",
    "allowed_data_classes": ["marker_id_mask"],
    "forbidden_data_classes": ["private_listing", "business_verified_listing_detail", "contact_data"]
  }
}
```

- [ ] **Step 2: Extend guardrail**

The guardrail must reject:

- `bbox`, `bounds`, `south`, `west`, `north`, `east` in public marker route shapes;
- listing coordinate ownership such as `listing.latitude`, `listing.longitude`, `geom_point`;
- platform-core direct database imports from Gongzzang;
- public listing tile routes that do not require `filter_hash`;
- public tombstone/delta routes returning private data fields.

- [ ] **Step 3: Run guardrail**

Run:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/ci/check-pnu-anchor-pbf-marker-contract
```

Expected: PASS.

---

## Task 9: Update Frontend Map Composition

**Files:**

- Modify: `apps/web/lib/map/marker-tile-style.ts`
- Modify: `apps/web/lib/map/vector-tile-manifest.ts`
- Modify: `apps/web/lib/map/listing-map-runtime.ts`
- Modify: `apps/web/components/listings/listing-map.tsx`
- Test: `apps/web/tests/unit/map/marker-tile-style.test.ts`
- Test: `apps/web/tests/unit/map/vector-tile-manifest.test.ts`

- [ ] **Step 1: Add composition model**

The frontend map state must track:

```ts
type ListingMarkerOverlayState = {
  baseVersion: number | null;
  tombstoneIds: Set<string>;
  deltaSourceId: string;
};
```

- [ ] **Step 2: Apply tombstones before display**

The rendered visible marker set must apply:

```text
visible = base + delta - tombstone
```

The client must never treat tombstone failure as permission to display a stale private/deleted
marker. If tombstones fail for a tile, the client should refresh the base tile or hide the affected
listing layer for that tile until a safe response arrives.

- [ ] **Step 3: Add delta source/layer**

Register a `listing_delta` vector source and layer. Use the same visual style as `listing`, with a
stable source id and layer id generated from the route policy or marker layer registry.

- [ ] **Step 4: Run frontend checks**

Run:

```powershell
pnpm --filter web test
pnpm --filter web exec playwright test
```

Expected: PASS. If no Playwright marker smoke exists, add a minimal smoke before claiming complete.

---

## Task 10: Verification And Release Gate

**Files:**

- Modify: `docs/testing/load.md`
- Modify: `scripts/load/run-k6`

- [ ] **Step 1: Run backend tests**

Run:

```powershell
cargo test -p listing-domain
cargo test -p db --features integration --test listing_marker_tile_integration
cargo test -p api listing_marker
```

Expected: PASS.

- [ ] **Step 2: Run guardrails**

Run:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/ci/check-pnu-anchor-pbf-marker-contract
powershell -ExecutionPolicy Bypass -File scripts/ci/check-platform-core-boundary
powershell -ExecutionPolicy Bypass -File scripts/ci/check-platform-core-dependency-boundary
```

Expected: PASS.

- [ ] **Step 3: Run load proof**

Run the existing map marker load scenario with:

```text
base tile
base + tombstone
base + delta
cache hit
cache miss
```

Acceptance:

- public stale delete/private marker exposure: 0 known cases;
- successful tile response with silent marker drop: 0;
- tombstone endpoint p95 within public route budget;
- delta endpoint p95 within public route budget;
- DB pool saturation absent under accepted launch RPS.

- [ ] **Step 4: Update docs**

Update:

- `docs/frontend/listings-search.md`
- `docs/runbooks/platform-core-integration-operations.md`
- `docs/testing/load.md`

Mention:

```text
visible markers = base tile + delta overlay - tombstone overlay - unauthorized records
```

---

## Execution Order

1. Task 1 and Task 2 establish schema and typed contracts.
2. Task 3 makes write paths emit structural facts.
3. Task 4 ships tombstone first to prevent stale private/deleted exposure.
4. Task 5 ships delta after stale exposure is controlled.
5. Task 6 makes low zoom truthful.
6. Task 7 adds rebuild/backlog control.
7. Task 8 locks SSOT and guardrails.
8. Task 9 updates frontend composition.
9. Task 10 verifies the full path.

Do not start artifact promote/rollback before tombstone, delta, aggregation, and dirty queue are
working. Static artifacts without tombstones can make stale exposure harder to correct.
