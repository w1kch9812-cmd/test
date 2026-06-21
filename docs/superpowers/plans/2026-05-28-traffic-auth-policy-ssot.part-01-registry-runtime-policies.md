# Traffic/Auth Policy SSOT Plan - Part 01: Registry And Runtime Generated Policies

Parent index: [Traffic/Auth Policy SSOT Implementation Plan](./2026-05-28-traffic-auth-policy-ssot.md).

## Task 1: Registry And Drift Check

**Files:**

- Create: `docs/architecture/traffic-auth-policy-registry.v1.json`
- Create: `scripts/ci/check-traffic-auth-policy-registry`
- Verify: `apps/web/proxy.ts`
- Verify: `services/api/src/listing_marker_serving.rs`

- [x] **Step 1: Add the policy registry**

The registry must include:

```json
{
  "schema_version": "gongzzang.traffic_auth_policy_registry.v1",
  "public_route_policies": [
    {
      "id": "gongzzang.public_map.listing_marker_tile",
      "rate_policy": {
        "key_prefix": "public-map:listing-marker-tile",
        "limit": 600,
        "window_seconds": 60,
        "rejection_status": 429
      },
      "cache_policy": {
        "server_cache": "redis",
        "ttl_seconds": 30
      },
      "single_flight_policy": {
        "required": true,
        "lock_seconds": 5,
        "wait_attempts": 10,
        "wait_milliseconds": 50
      },
      "response_budget": {
        "max_tile_bytes": 262144,
        "max_features": 10000
      }
    }
  ]
}
```

- [x] **Step 2: Add the drift check**

Run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\check-traffic-auth-policy-registry -Root .
```

Expected:

```text
traffic-auth-policy-registry-ok routes=6 service_policies=2
```

- [x] **Step 3: Confirm current registry matches code**

The script must compare:

- `apps/web/proxy.ts` public map route key prefixes, limits, and windows.
- `services/api/src/listing_marker_serving.rs` cache TTL, single-flight lock/wait values, tile byte budget, feature budget, and mask id budget.
- `docs/architecture/platform-core-boundary.v1.json` service auth env contract.

## Task 2: Generate TypeScript Proxy Policy

**Files:**

- Create: `scripts/ci/generate-traffic-auth-policy`
- Create: `apps/web/lib/policies/traffic-auth-policy.generated.ts`
- Modify: `apps/web/proxy.ts`
- Test: `apps/web/tests/unit/platform-core-proxy.test.ts`

- [x] **Step 1: Add a generator that emits TypeScript policy**

Create `scripts/ci/generate-traffic-auth-policy` with a generator that reads `docs/architecture/traffic-auth-policy-registry.v1.json` and writes `apps/web/lib/policies/traffic-auth-policy.generated.ts`.

The generated TypeScript shape must be:

```ts
export type GeneratedPublicMapRoutePolicy = {
  readonly kind: "exact" | "prefix";
  readonly pathSource: string;
  readonly rate: {
    readonly keyPrefix: string;
    readonly limit: number;
    readonly windowSec: number;
  };
};

export const GENERATED_PUBLIC_MAP_ROUTE_POLICIES: readonly GeneratedPublicMapRoutePolicy[] = [
  {
    kind: "prefix",
    pathSource: "API.proxy.listingMarkerTilesPrefix",
    rate: { keyPrefix: "public-map:listing-marker-tile", limit: 600, windowSec: 60 },
  },
  {
    kind: "exact",
    pathSource: "API.proxy.listingMarkerCounts",
    rate: { keyPrefix: "public-map:listing-marker-count", limit: 120, windowSec: 60 },
  },
  {
    kind: "exact",
    pathSource: "API.proxy.listingMarkerFilters",
    rate: { keyPrefix: "public-map:listing-marker-filter", limit: 60, windowSec: 60 },
  },
  {
    kind: "prefix",
    pathSource: "LISTING_MARKER_MASK_PREFIX",
    rate: { keyPrefix: "public-map:listing-marker-mask", limit: 120, windowSec: 60 },
  },
];
```

- [x] **Step 2: Run the generator**

Run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\generate-traffic-auth-policy -Root .
```

Expected:

```text
traffic-auth-policy-generated ts=apps/web/lib/policies/traffic-auth-policy.generated.ts
```

- [x] **Step 3: Replace hardcoded proxy rate constants with generated values**

Modify `apps/web/proxy.ts` so `PUBLIC_MAP_ROUTE_POLICIES` maps generated path sources to concrete route paths:

```ts
import { GENERATED_PUBLIC_MAP_ROUTE_POLICIES } from "@/lib/policies/traffic-auth-policy.generated";

function resolvePublicMapPath(pathSource: string): string {
  switch (pathSource) {
    case "API.proxy.listingMarkerTilesPrefix":
      return API.proxy.listingMarkerTilesPrefix;
    case "API.proxy.listingMarkerCounts":
      return API.proxy.listingMarkerCounts;
    case "API.proxy.listingMarkerFilters":
      return API.proxy.listingMarkerFilters;
    case "LISTING_MARKER_MASK_PREFIX":
      return LISTING_MARKER_MASK_PREFIX;
    default:
      throw new Error(`Unknown public map route policy path source: ${pathSource}`);
  }
}

const PUBLIC_MAP_ROUTE_POLICIES: readonly PublicMapRoutePolicy[] =
  GENERATED_PUBLIC_MAP_ROUTE_POLICIES.map((policy) => ({
    kind: policy.kind,
    path: resolvePublicMapPath(policy.pathSource),
    rate: policy.rate,
  }));
```

- [x] **Step 4: Verify proxy tests**

Run:

```powershell
pnpm --filter @gongzzang/web test -- tests/unit/platform-core-proxy.test.ts
```

Expected:

```text
Test Files  1 passed
Tests  7 passed
```

## Task 3: Generate Rust Listing Marker Serving Policy

**Files:**

- Modify: `scripts/ci/generate-traffic-auth-policy`
- Create: `services/api/src/listing_marker_policy.rs`
- Modify: `services/api/src/main.rs`
- Modify: `services/api/src/listing_marker_serving.rs`
- Test: `services/api/src/listing_marker_serving.rs`

- [x] **Step 1: Extend generator for Rust constants**

The generator must emit `services/api/src/listing_marker_policy.rs`:

```rust
//! Generated listing marker serving policy from docs/architecture/traffic-auth-policy-registry.v1.json.

pub const MAX_LISTING_MARKER_TILE_BYTES: usize = 262_144;
pub const MAX_LISTING_MARKER_TILE_FEATURES: i64 = 10_000;
pub const MAX_LISTING_MARKER_MASK_IDS: usize = 20_000;
pub const LISTING_MARKER_CACHE_TTL_SECONDS: u64 = 30;
pub const LISTING_MARKER_SINGLE_FLIGHT_LOCK_SECONDS: u64 = 5;
pub const LISTING_MARKER_SINGLE_FLIGHT_WAIT_ATTEMPTS: usize = 10;
pub const LISTING_MARKER_SINGLE_FLIGHT_WAIT_MS: u64 = 50;
```

- [x] **Step 2: Register generated module**

Modify `services/api/src/main.rs`:

```rust
mod listing_marker_policy;
mod listing_marker_serving;
```

- [x] **Step 3: Consume generated constants**

Modify `services/api/src/listing_marker_serving.rs`:

```rust
use crate::listing_marker_policy::{
    LISTING_MARKER_CACHE_TTL_SECONDS, LISTING_MARKER_SINGLE_FLIGHT_LOCK_SECONDS,
    LISTING_MARKER_SINGLE_FLIGHT_WAIT_ATTEMPTS, LISTING_MARKER_SINGLE_FLIGHT_WAIT_MS,
    MAX_LISTING_MARKER_MASK_IDS, MAX_LISTING_MARKER_TILE_BYTES,
    MAX_LISTING_MARKER_TILE_FEATURES,
};
```

Then replace local constants with imported generated constants.

- [x] **Step 4: Verify Rust API binary compile**

Run:

```powershell
C:\Users\admin\.cargo\bin\cargo.exe check --workspace --bins --all-features
```

Expected:

```text
Finished `dev` profile
```
