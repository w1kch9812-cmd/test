# Traffic/Auth Policy SSOT Plan - Part 03: Platform Core Companion And Completion Gate

Parent index: [Traffic/Auth Policy SSOT Implementation Plan](./2026-05-28-traffic-auth-policy-ssot.md).


## Task 5: Platform Core Companion Registry

**Files:**

- Create in sibling repo: `../platform-core/docs/architecture/traffic-auth-policy-registry.v1.json`
- Create in sibling repo: `../platform-core/scripts/ci/check-traffic-auth-policy-registry`
- Modify in sibling repo: `../platform-core/services/api/src/traffic.rs`
- Modify in sibling repo: `../platform-core/services/api/src/routes/mod.rs`

- [x] **Step 1: Add platform-core registry**

The platform-core registry must declare:

- global HTTP timeout, body limit, and concurrency.
- public marker contract endpoint exposure.
- DB-backed marker tile route as `diagnostic_reference`, not launch runtime.
- required production edge/app route policy for public routes.
- service identity policy for Gongzzang callers.

- [x] **Step 2: Add drift check**

The drift check must compare registry values to:

- `services/api/src/traffic.rs`
- `services/api/src/routes/mod.rs`
- `docs/adr/0008-pnu-anchor-pbf-marker-tile-contract.md`

- [x] **Step 3: Verify platform-core check**

Run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\check-traffic-auth-policy-registry -Root C:\Users\admin\Desktop\platform-core
```

Expected:

```text
traffic-auth-policy-registry-ok
```

## Task 6: Completion Gate

**Files:**

- Verify: `docs/architecture/traffic-auth-policy-registry.v1.json`
- Verify: `apps/web/proxy.ts`
- Verify: `services/api/src/listing_marker_serving.rs`
- Verify: sibling `platform-core` registry and checks

- [x] **Step 1: Verify Gongzzang registry drift**

Run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\check-traffic-auth-policy-registry -Root .
```

Expected:

```text
traffic-auth-policy-registry-ok routes=6 service_policies=2
```

- [x] **Step 2: Verify focused web policy tests**

Run:

```powershell
pnpm --filter @gongzzang/web test -- tests/unit/platform-core-proxy.test.ts
```

Expected:

```text
Test Files  1 passed
Tests  7 passed
```

- [x] **Step 3: Verify Rust executable compile**

Run:

```powershell
C:\Users\admin\.cargo\bin\cargo.exe check --workspace --bins --all-features
```

Expected:

```text
Finished `dev` profile
```

- [x] **Step 4: Verify all-targets once existing unrelated test drift is resolved**

Run:

```powershell
C:\Users\admin\.cargo\bin\cargo.exe check --workspace --all-targets --all-features
```

Expected:

```text
Finished `dev` profile
```

Fresh local evidence on 2026-05-29: the broad all-targets workspace check
completed successfully.
