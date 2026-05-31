# Local Sizing Test Results

Created: 2026-05-29

Scope: Platform Core local host-process read smoke. This document is a sizing
input only, not a Gongzzang production admission artifact.

## Decision Boundary

This is not production launch sizing evidence.

Accepted claim: Platform Core local route smoke looked healthy on the local
fixture, but a real perf/staging operator run remains required before any launch
sizing claim.

Rejected claims:

- Gongzzang Rust API is faster than the legacy Gongzzang API.
- The whole product can launch on a specific AWS instance size.
- Database, cache, worker, and map tile capacity are already proven.

Not covered: Gongzzang Rust API, Gongzzang legacy API, AWS Fargate/RDS hard
limits, nationwide anchor/read-model data, Bronze normalization workers.

## Tested Target

- Binary: `C:\Users\admin\Desktop\platform-core\target\release\platform-core-api.exe`
- PostgreSQL/PostGIS: local Docker container `platform-core-db`
- Redis: local Docker container `platform-core-redis`
- k6 scenario: `platform-core/scripts/load/platform-core-read-smoke.js`

Routes included:

- `/health`
- `/catalog/v1/vector-tiles/manifest`
- `/catalog/v1/pipeline-graph`
- `/map/v1/marker-tiles/contract`
- `/map/v1/marker-tiles/parcel_anchor/12/3494/1591.pbf?filter_hash=all-active-v1`

## Not Tested

- Gongzzang Rust API listing, search, write, auth, or marker serving paths.
- Legacy `gongzzang-develop` Kotlin/Spring API under comparable traffic.
- AWS Fargate CPU/RAM hard limits.
- AWS RDS, production-like PostGIS, or production-like Redis/Valkey.
- Nationwide anchor/read-model data volume.
- Bronze normalization worker throughput.
- End-to-end Gongzzang + Platform Core integration under load.

## Local Conditions

```text
DATABASE_URL=postgres://platform_core:platform_core_dev_2026@localhost:15434/platform_core
PLATFORM_CORE_API_BIND_ADDR=127.0.0.1:18080
PLATFORM_CORE_DB_MAX_CONNECTIONS=8
PLATFORM_CORE_DB_MIN_CONNECTIONS=1
PLATFORM_CORE_HTTP_MAX_CONCURRENCY=128
PLATFORM_CORE_HTTP_REQUEST_TIMEOUT_MS=3000
```

The first run failed because the binary was stale and the local marker anchor
fixture was missing. After rebuilding the current code and inserting one active
local marker anchor fixture, the target returned healthy responses for the
smoke routes.

Evidence:

```text
C:\Users\admin\Desktop\platform-core\target\load\local-sizing-20260529T013358Z\local-sizing-matrix.json
```

## Measurements

| Target read RPS | Actual read it/s | HTTP req/s | Dropped iterations | p95 | p99 | Max | Error rate | API memory | API CPU core equiv |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 5 | 7.00 | 22.00 | 0 | 3.10ms | 5.19ms | 15.13ms | 0% | 13.6MB | 0.011 |
| 20 | 21.99 | 81.88 | 0 | 2.47ms | 3.35ms | 14.42ms | 0% | 14.0MB | 0.024 |
| 50 | 51.89 | 201.47 | 0 | 2.42ms | 2.99ms | 7.55ms | 0% | 14.0MB | 0.066 |
| 100 | 101.68 | 400.74 | 0 | 2.51ms | 3.00ms | 19.44ms | 0% | 14.9MB | 0.159 |
| 200 | 201.36 | 799.37 | 0 | 2.09ms | 2.64ms | 14.85ms | 0% | 14.8MB | 0.281 |
| 500 | 499.16 | 1990.57 | 35 | 2.13ms | 2.88ms | 56.80ms | 0% | 15.8MB | 0.711 |
| 1000 | 804.35 | 3211.34 | 5809 | 44.47ms | 55.88ms | 89.48ms | 0% | 18.4MB | 1.564 |

## Interpretation

The local Platform Core route smoke had headroom through 200 target read RPS on
the local fixture. At 500 target read RPS, k6 began dropping iterations even
though observed HTTP errors stayed at 0%. At 1000 target read RPS, the runner
configuration became the likely limiter before the API itself.

That means this run is useful for confidence in the local Platform Core read
path and for choosing the next perf test matrix. It does not prove production
capacity.

## Next Required Test

1. Run Gongzzang Rust API and Platform Core behind production-like perf/staging
   infrastructure.
2. Use comparable DB, Redis/Valkey, data volume, and route mix.
3. Capture API CPU, memory, p95, p99, DB connections, DB latency, cache hit
   rate, and worker lag.
4. Compare `0.5 vCPU/1GB`, `1 vCPU/2GB`, and `2 vCPU/4GB` service sizes before
   making launch sizing claims.
