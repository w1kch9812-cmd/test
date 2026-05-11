# T2: Allowlist 상수 + V-World source rename + Migration 30012

**Goal:** data.go.kr building / V-World parcel allowlist 상수를 정의하고, V-World source taxonomy 확장 마이그레이션(30012)을 적용하며, `crates/data-clients/vworld/src/reader.rs:71` 의 source literal `"vworld"` 를 `"vworld_parcel"` const 로 정정한다.

**Spec SSOT:** §5.2, §5.3, §11 (V-World reader 변경), §13 T2 ([design doc](../../specs/2026-05-08-sp10-5-b-backend-data-correctness-design.md))

**T1 inputs (already exported):** `AllowlistSanitizer::new`, `compute_schema_hash`, `RawSanitizer`, `SanitizedRaw`, `SanitizingRawCapture`.

**Files:**

- Create: `crates/data-clients/raw-capture/src/sources/mod.rs`
- Create: `crates/data-clients/raw-capture/src/sources/data_go_kr_building.rs`
- Create: `crates/data-clients/raw-capture/src/sources/vworld_parcel.rs`
- Create: `migrations/30012_source_taxonomy_expansion.sql`
- Modify: `crates/data-clients/raw-capture/src/sanitizer.rs` (add `for_source` factory)
- Modify: `crates/data-clients/raw-capture/src/lib.rs` (expose `sources` module + doc example)
- Modify: `crates/data-clients/vworld/src/reader.rs` (line 71 source const)

**Existing code refs:**

- [`crates/data-clients/vworld/src/reader.rs:71`](../../../crates/data-clients/vworld/src/reader.rs#L71) — `.capture(pnu.as_str(), "vworld", &raw, now)` (literal — fix target)
- [`crates/data-clients/data-go-kr/src/building_register/reader.rs:37`](../../../crates/data-clients/data-go-kr/src/building_register/reader.rs#L37) — `const RAW_CAPTURE_SOURCE: &str = "data_go_kr_building";` (이미 const 패턴 — V-World 도 따라가야 함)
- [`migrations/30006_parcel_external_data.sql:13-19`](../../../migrations/30006_parcel_external_data.sql#L13-L19) — 기존 CHECK 제약 (`vworld` enum 포함)

---

## Step 2.1: sources/ module skeleton

- [ ] **Step 2.1.1: Create `crates/data-clients/raw-capture/src/sources/mod.rs`**

```rust
//! Source-specific allowlist 상수 정의. 각 외부 API 별로 정제 후 보존할
//! JSON path 를 명시한다. PIPA 최소수집 원칙의 SSOT.

pub mod data_go_kr_building;
pub mod vworld_parcel;
```

- [ ] **Step 2.1.2: Modify `crates/data-clients/raw-capture/src/lib.rs` — expose sources module**

기존 module declarations 옆에 추가:

```rust
pub mod sources;
```

- [ ] **Step 2.1.3: Create intentionally empty module files**

```bash
touch crates/data-clients/raw-capture/src/sources/data_go_kr_building.rs
touch crates/data-clients/raw-capture/src/sources/vworld_parcel.rs
```

빈 파일 2 개 — Step 2.1.1 의 `pub mod` 선언이 컴파일 가능하도록 scaffold. 본문 추가는 Step 2.2 / 2.3 에서.

- [ ] **Step 2.1.4: Verify scaffold compiles**

```bash
cargo check -p raw-capture-client
# Expected: Finished (sources::data_go_kr_building, sources::vworld_parcel 모듈 선언만, 본문 0)
```

- [ ] **Step 2.1.5: Commit scaffold**

```bash
git add crates/data-clients/raw-capture/src/sources/ crates/data-clients/raw-capture/src/lib.rs
git commit -m "feat(sp10-5-b-T2): scaffold sources/ module (empty data_go_kr_building/vworld_parcel files)"
```

---

## Step 2.2: data_go_kr_building allowlist (TDD)

Spec §5.2 — 7-path allowlist + envelope.

- [ ] **Step 2.2.1: Write failing test (test ONLY — const not yet defined)**

Append to `crates/data-clients/raw-capture/src/sources/data_go_kr_building.rs`:

```rust
//! data.go.kr 건축물대장 (`getBrTitleInfo`) allowlist.
//!
//! Spec §5.2 SSOT. PII 후보 필드 (ownerNm, regstrKindCd 등) 는 모두 비포함.

// Implementation comes in Step 2.2.3

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn building_allowlist_has_7_paths() {
        assert_eq!(BUILDING_ALLOWLIST.len(), 7);
    }

    #[test]
    fn building_allowlist_contains_required_fields() {
        let paths: Vec<&str> = BUILDING_ALLOWLIST.iter().copied().collect();
        // Spec §5.2 — 7 required paths
        assert!(paths.contains(&"/response/header/resultCode"));
        assert!(paths.contains(&"/response/header/resultMsg"));
        assert!(paths.contains(&"/response/body/items/item/*/mgmBldrgstPk"));
        assert!(paths.contains(&"/response/body/items/item/*/bldNm"));
        assert!(paths.contains(&"/response/body/items/item/*/mainPurpsCdNm"));
        assert!(paths.contains(&"/response/body/items/item/*/totArea"));
        assert!(paths.contains(&"/response/body/items/item/*/useAprDay"));
    }

    #[test]
    fn building_allowlist_excludes_pii_candidates() {
        let paths: Vec<&str> = BUILDING_ALLOWLIST.iter().copied().collect();
        // PII 후보 — 절대 포함되면 안 됨
        assert!(!paths.iter().any(|p| p.contains("ownerNm")));
        assert!(!paths.iter().any(|p| p.contains("regstrKind")));
        assert!(!paths.iter().any(|p| p.contains("phone")));
        assert!(!paths.iter().any(|p| p.contains("RRN")));
    }

    #[test]
    fn building_source_id_const() {
        assert_eq!(SOURCE_ID, "data_go_kr_building");
    }
}
```

- [ ] **Step 2.2.2: Run — verify FAIL (const undefined)**

```bash
cargo test -p raw-capture-client --lib sources::data_go_kr_building
# Expected: error[E0425]: cannot find value `BUILDING_ALLOWLIST` in this scope
```

- [ ] **Step 2.2.3: Implement allowlist const + SOURCE_ID**

Replace `// Implementation comes in Step 2.2.3` with:

```rust
/// `parcel_external_data.source` 컬럼 값 (CHECK enum). 마이그레이션 30006 +
/// 30012 에서 정의된 enum 과 정확히 일치해야 함.
pub const SOURCE_ID: &str = "data_go_kr_building";

/// 정제 후 `parcel_external_data.raw_response` 에 보존할 JSON path.
///
/// Spec §5.2 SSOT (design.md:213-221). `getBrTitleInfo` 응답 (`/response/body/items/item/*`)
/// 의 7 path. PII 후보 (소유자명, 주민등록번호, 연락처, 등기종류) 는 비포함.
pub const BUILDING_ALLOWLIST: &[&str] = &[
    "/response/header/resultCode",
    "/response/header/resultMsg",
    "/response/body/items/item/*/mgmBldrgstPk",
    "/response/body/items/item/*/bldNm",
    "/response/body/items/item/*/mainPurpsCdNm",
    "/response/body/items/item/*/totArea",
    "/response/body/items/item/*/useAprDay",
];
```

- [ ] **Step 2.2.4: Run all building allowlist tests — verify PASS**

```bash
cargo test -p raw-capture-client --lib sources::data_go_kr_building
# Expected: 4 passed (length, contains_required, excludes_pii, source_id)
```

- [ ] **Step 2.2.5: Commit**

```bash
git add crates/data-clients/raw-capture/src/sources/data_go_kr_building.rs
git commit -m "feat(sp10-5-b-T2): data_go_kr_building 7-path allowlist (spec §5.2)"
```

---

## Step 2.3: vworld_parcel allowlist (TDD)

Spec §5.3 — 9 properties + 5 envelope paths (status / service / error / record / page).

- [ ] **Step 2.3.1: Write failing test**

Append to `crates/data-clients/raw-capture/src/sources/vworld_parcel.rs`:

```rust
//! V-World 연속지적도 (`LP_PA_CBND_BUBUN`) allowlist.
//!
//! Spec §5.3 SSOT — properties 9 path + envelope 5 path.
//! Fixture: `crates/data-clients/vworld/tests/fixtures/real_parcel_boundary_*.json`.
//! 공시지가 (`jiga`) 는 PIPA 상 공개 행정정보 → allowlist 포함.

// Implementation comes in Step 2.3.3

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vworld_parcel_source_id_const() {
        // 30012 마이그레이션 이후 신규 INSERT 가 사용할 source 값
        assert_eq!(SOURCE_ID, "vworld_parcel");
    }

    #[test]
    fn vworld_parcel_allowlist_has_properties_and_envelope() {
        // 9 properties + 5 envelope = 14
        assert_eq!(VWORLD_PARCEL_ALLOWLIST.len(), 14);
    }

    #[test]
    fn vworld_parcel_allowlist_contains_jiga() {
        // PIPA 검토 — 공시지가는 공개정보, 패널 핵심 표시 필드
        let paths: Vec<&str> = VWORLD_PARCEL_ALLOWLIST.iter().copied().collect();
        assert!(paths.iter().any(|p| p.ends_with("/jiga")));
    }

    #[test]
    fn vworld_parcel_allowlist_contains_geometry() {
        let paths: Vec<&str> = VWORLD_PARCEL_ALLOWLIST.iter().copied().collect();
        assert!(paths.iter().any(|p| p.ends_with("/geometry")));
    }

    #[test]
    fn vworld_parcel_allowlist_envelope_status() {
        let paths: Vec<&str> = VWORLD_PARCEL_ALLOWLIST.iter().copied().collect();
        assert!(paths.contains(&"/response/status"));
    }

    #[test]
    fn vworld_parcel_allowlist_excludes_hypothetical_pii() {
        let paths: Vec<&str> = VWORLD_PARCEL_ALLOWLIST.iter().copied().collect();
        // V-World 응답엔 보통 PII 없음 — 미래 schema drift 대비
        assert!(!paths.iter().any(|p| p.contains("OWNER_RRN")));
        assert!(!paths.iter().any(|p| p.contains("ownerName")));
    }
}
```

- [ ] **Step 2.3.2: Run — verify FAIL**

```bash
cargo test -p raw-capture-client --lib sources::vworld_parcel
# Expected: error[E0425]: cannot find value `VWORLD_PARCEL_ALLOWLIST`
```

- [ ] **Step 2.3.3: Implement allowlist const + SOURCE_ID**

```rust
/// `parcel_external_data.source` 컬럼 값. 30012 마이그 후 신규 INSERT 가 사용.
/// (legacy 'vworld' row 는 30012 backfill UPDATE 가 'vworld_parcel' 로 변경)
pub const SOURCE_ID: &str = "vworld_parcel";

/// V-World `LP_PA_CBND_BUBUN` 응답 정제 후 보존할 path.
///
/// Spec §5.3 SSOT — 9 properties + 5 envelope.
///
/// Properties (geometry + 8 properties.*):
///   - geometry — MultiPolygon coordinates (EPSG:4326)
///   - pnu, jibun, bonbun, bubun, addr — 식별/주소
///   - jiga — 공시지가 (₩/m²); 공개 행정정보 (PIPA 검토)
///   - gosi_year, gosi_month — 공시 시점
///
/// Envelope (drift 진단 + status 분기):
///   - status — OK / NOT_FOUND / ERROR
///   - service.* — operation, version, time
///   - error.* — code, text (ERROR 케이스)
///   - record.* / page.* — pagination
pub const VWORLD_PARCEL_ALLOWLIST: &[&str] = &[
    // 9 properties
    "/response/result/featureCollection/features/*/geometry",
    "/response/result/featureCollection/features/*/properties/pnu",
    "/response/result/featureCollection/features/*/properties/jibun",
    "/response/result/featureCollection/features/*/properties/bonbun",
    "/response/result/featureCollection/features/*/properties/bubun",
    "/response/result/featureCollection/features/*/properties/addr",
    "/response/result/featureCollection/features/*/properties/jiga",
    "/response/result/featureCollection/features/*/properties/gosi_year",
    "/response/result/featureCollection/features/*/properties/gosi_month",
    // 5 envelope — spec §5.3 literal (와일드카드 `/*` 로 sub-tree 전체 보존)
    "/response/status",
    "/response/service/*",
    "/response/error/*",
    "/response/record/*",
    "/response/page/*",
];
```

- [ ] **Step 2.3.4: Run all vworld_parcel allowlist tests — verify PASS**

```bash
cargo test -p raw-capture-client --lib sources::vworld_parcel
# Expected: 6 passed (source_id, length, contains_jiga, contains_geometry, envelope_status, excludes_pii)
```

- [ ] **Step 2.3.5: Commit**

```bash
git add crates/data-clients/raw-capture/src/sources/vworld_parcel.rs
git commit -m "feat(sp10-5-b-T2): vworld_parcel 14-path allowlist (spec §5.3 + envelope)"
```

---

## Step 2.4: AllowlistSanitizer::for_source factory (TDD)

T1 의 `AllowlistSanitizer::new` 위에 source 기반 factory 메서드.

- [ ] **Step 2.4.1: Append failing test to `crates/data-clients/raw-capture/src/sanitizer.rs`**

```rust
    #[test]
    fn for_source_data_go_kr_building() {
        let san = AllowlistSanitizer::for_source("data_go_kr_building").unwrap();
        assert_eq!(san.source(), "data_go_kr_building");
        assert_eq!(san.allowed_paths().len(), 7);
        assert_eq!(san.sanitizer_version(), 1);
    }

    #[test]
    fn for_source_vworld_parcel() {
        let san = AllowlistSanitizer::for_source("vworld_parcel").unwrap();
        assert_eq!(san.source(), "vworld_parcel");
        assert_eq!(san.allowed_paths().len(), 14);
    }

    #[test]
    fn for_source_unknown_returns_err() {
        let result = AllowlistSanitizer::for_source("unknown_source");
        assert!(result.is_err());
    }

    #[test]
    fn for_source_legacy_vworld_returns_err() {
        // 30012 마이그 이후 'vworld' 는 deprecated → 직접 인스턴스화 차단
        let result = AllowlistSanitizer::for_source("vworld");
        assert!(result.is_err());
    }
```

- [ ] **Step 2.4.2: Run — verify FAIL (for_source not defined)**

```bash
cargo test -p raw-capture-client --lib sanitizer::tests::for_source
# Expected: error[E0599]: no function or associated item named `for_source` found
```

- [ ] **Step 2.4.3: Add `SanitizerError` enum + implement `for_source`**

Append to `sanitizer.rs`:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SanitizerError {
    #[error("unknown source: {0}")]
    UnknownSource(String),
}

impl AllowlistSanitizer {
    /// source ID 로 allowlist 를 lookup 하여 sanitizer 인스턴스화.
    /// 등록되지 않은 source 는 `Err(UnknownSource)` — fail-safe 거부.
    pub fn for_source(source: &str) -> Result<Self, SanitizerError> {
        use crate::sources::{data_go_kr_building, vworld_parcel};

        let (allowed_paths, version) = match source {
            data_go_kr_building::SOURCE_ID => (
                data_go_kr_building::BUILDING_ALLOWLIST
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>(),
                1,
            ),
            vworld_parcel::SOURCE_ID => (
                vworld_parcel::VWORLD_PARCEL_ALLOWLIST
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>(),
                1,
            ),
            _ => return Err(SanitizerError::UnknownSource(source.to_string())),
        };
        Ok(Self::new(source.to_string(), allowed_paths, version))
    }
}
```

- [ ] **Step 2.4.4: Run all for_source tests — verify PASS**

```bash
cargo test -p raw-capture-client --lib sanitizer::tests::for_source
# Expected: 4 passed
```

- [ ] **Step 2.4.5: Add re-export to `lib.rs`**

```rust
pub use sanitizer::{
    AllowlistSanitizer, RawSanitizer, SanitizedRaw, SanitizerError, compute_schema_hash,
};
```

- [ ] **Step 2.4.6: Re-run tests + verify export compiles**

```bash
cargo test -p raw-capture-client --lib sanitizer::tests::for_source
# Expected: 4 passed (PASS run after re-export)
cargo check --workspace
# Expected: Finished — SanitizerError 가 외부 crate 에서도 사용 가능
```

- [ ] **Step 2.4.7: Commit**

```bash
git add crates/data-clients/raw-capture/src/sanitizer.rs crates/data-clients/raw-capture/src/lib.rs
git commit -m "feat(sp10-5-b-T2): AllowlistSanitizer::for_source factory + SanitizerError"
```

---

## Step 2.5: Migration 30012 — source taxonomy expansion

- [ ] **Step 2.5.1: Create `migrations/30012_source_taxonomy_expansion.sql`**

```sql
-- V003_12: source taxonomy expansion — V-World 다중 endpoint 대비.
--
-- Spec SSOT: docs/superpowers/specs/2026-05-08-sp10-5-b-backend-data-correctness-design.md §5, §11.
--
-- 'vworld' 는 legacy alias 로 enum 에 유지하되, 신규 INSERT 는 구체 endpoint
-- name (`vworld_parcel`) 을 사용. 기존 'vworld' row 는 backfill UPDATE 로
-- 'vworld_parcel' 로 rename — Reader 코드 (`crates/data-clients/vworld/src/
-- reader.rs:71`) 도 동일 PR 에 같이 변경되어야 backfill 직후 재오염 방지.
--
-- Lock safety: parcel_external_data 는 v1 운영 단계 row 수가 적다 (배포 직후
-- 시점에 raw_response 가 매 패널 hit 마다 INSERT 되지만 cumulative row count 가
-- 만 단위 미만). DROP/ADD CHECK 는 short-duration AccessExclusiveLock 만 잡고
-- 즉시 해제 → CONCURRENTLY 옵션 없이 acceptable. row 수가 100만+ 되면 down-
-- time migration 또는 CHECK NOT VALID + 별도 검증 패턴 검토.

BEGIN;

-- 1. 기존 CHECK 제거
ALTER TABLE parcel_external_data
    DROP CONSTRAINT parcel_external_data_source_check;

-- 2. 확장된 CHECK 추가 (vworld_parcel + future endpoints)
ALTER TABLE parcel_external_data ADD CONSTRAINT parcel_external_data_source_check
    CHECK (source IN (
        'vworld',                          -- legacy alias (backfill 이전 row 보존용)
        'vworld_parcel',                   -- LP_PA_CBND_BUBUN (지적 폴리곤 endpoint)
        'data_go_kr_building',
        'data_go_kr_land',
        'data_go_kr_realtransaction',
        'korean_law'
    ));

-- 3. 기존 'vworld' row 를 'vworld_parcel' 로 rename
UPDATE parcel_external_data SET source = 'vworld_parcel' WHERE source = 'vworld';

COMMIT;
```

- [ ] **Step 2.5.1a: Preflight — 'vworld' row count check (마이그 실행 *전*)**

마이그레이션 실행 직전 backfill 영향 범위 확인. row 수가 예상보다 크면 lock 시간 검토.

```bash
psql gongzzang_dev -c "SELECT count(*) AS vworld_legacy_rows FROM parcel_external_data WHERE source = 'vworld';"
# Expected (v1 운영 시점): 0 ~ 수천 row (만 단위 미만)
# 만약 만 단위 초과: 별도 batched UPDATE 또는 점검 시간대 적용 검토 (block before migrate)
```

- [ ] **Step 2.5.2: Run forward migration**

```bash
DATABASE_URL=postgres://localhost/gongzzang_dev cargo sqlx migrate run
# Expected: Applied 30012/migrate source taxonomy expansion
```

- [ ] **Step 2.5.3: Verify CHECK enum updated**

```bash
psql gongzzang_dev -c "SELECT conname, pg_get_constraintdef(oid) FROM pg_constraint WHERE conname = 'parcel_external_data_source_check';"
# Expected: CHECK (source = ANY (ARRAY['vworld', 'vworld_parcel', 'data_go_kr_building', ...]))
```

- [ ] **Step 2.5.4: Verify backfill**

```bash
psql gongzzang_dev -c "SELECT source, count(*) FROM parcel_external_data GROUP BY source;"
# Expected: legacy 'vworld' row 들이 'vworld_parcel' 로 모두 rename됨 (vworld count = 0)
```

- [ ] **Step 2.5.5: Test rollback safety (manual)**

마이그레이션 sqlx 가 down 스크립트 자동 생성 안 한다면, manual rollback 명령으로 검증:

```bash
psql gongzzang_dev -c "
BEGIN;
ALTER TABLE parcel_external_data DROP CONSTRAINT parcel_external_data_source_check;
ALTER TABLE parcel_external_data ADD CONSTRAINT parcel_external_data_source_check
    CHECK (source IN ('vworld', 'data_go_kr_building', 'data_go_kr_land', 'data_go_kr_realtransaction', 'korean_law'));
UPDATE parcel_external_data SET source = 'vworld' WHERE source = 'vworld_parcel';
ROLLBACK;
"
# Expected: BEGIN ... ROLLBACK (검증 후 rollback — 실제 변경 없음)
```

- [ ] **Step 2.5.6: Commit**

```bash
git add migrations/30012_source_taxonomy_expansion.sql
git commit -m "feat(sp10-5-b-T2): migration 30012 — source taxonomy + vworld backfill"
```

---

## Step 2.6: V-World reader source const (BLOCKER — 마이그과 동일 PR)

Spec §11 — `crates/data-clients/vworld/src/reader.rs:71` literal `"vworld"` → const `RAW_CAPTURE_SOURCE`.

- [ ] **Step 2.6.1: Modify `crates/data-clients/vworld/src/reader.rs` — add const at top**

기존 imports 아래 (struct 선언 위) 에 추가 (data-go-kr building reader 의 line 37 패턴 따름):

```rust
/// `parcel_external_data.source` 컬럼 값 (CHECK enum 일치).
///
/// 30012 마이그레이션 이후 'vworld' (legacy alias) 대신 'vworld_parcel' 사용.
/// 마이그레이션과 *동일 PR* 에 묶여야 함 — 마이그만 적용되고 코드가 'vworld'
/// 그대로 INSERT 시 backfill 직후 다시 'vworld' row 가 생긴다.
pub const RAW_CAPTURE_SOURCE: &str = "vworld_parcel";
```

- [ ] **Step 2.6.2: Modify line 71 — replace literal with const**

기존:
```rust
.capture(pnu.as_str(), "vworld", &raw, now)
```

→
```rust
.capture(pnu.as_str(), RAW_CAPTURE_SOURCE, &raw, now)
```

- [ ] **Step 2.6.3: Build + run vworld tests**

```bash
cargo check -p vworld
# Expected: Finished
cargo test -p vworld --lib
# Expected: all tests pass (literal change 만이라 logic 동일)
```

- [ ] **Step 2.6.4: Commit**

```bash
git add crates/data-clients/vworld/src/reader.rs
git commit -m "feat(sp10-5-b-T2): vworld reader source literal → RAW_CAPTURE_SOURCE const"
```

---

## Step 2.7: raw-capture lib.rs doc example update

- [ ] **Step 2.7.1: Modify `crates/data-clients/raw-capture/src/lib.rs:7-18` doc example**

기존:
```rust
//! capture.capture("1111010100100010000", "vworld", &raw_json, Utc::now()).await?;
```

→
```rust
//! capture.capture("1111010100100010000", "vworld_parcel", &raw_json, Utc::now()).await?;
//! // legacy 'vworld' source 는 migration 30012 backfill 후 'vworld_parcel' 로 통일됨
```

- [ ] **Step 2.7.2: Verify doc test still parses**

```bash
cargo doc -p raw-capture-client --no-deps
# Expected: Finished (doc example 은 `ignore` 라 실행 안 됨)
```

- [ ] **Step 2.7.3: Commit**

```bash
git add crates/data-clients/raw-capture/src/lib.rs
git commit -m "docs(sp10-5-b-T2): lib.rs example source vworld → vworld_parcel"
```

---

## Step 2.8: End-to-end verification (sanitize 실제 V-World fixture)

T2 의 모든 산출물 (allowlist + factory + reader const) 이 실 V-World fixture 와 호환되는지 통합 검증.

- [ ] **Step 2.8.1: Append failing integration test to `crates/data-clients/raw-capture/src/sources/vworld_parcel.rs`**

```rust
    #[test]
    fn sanitize_real_vworld_fixture_retains_jiga() {
        use crate::{AllowlistSanitizer, RawSanitizer};

        // 실 V-World 응답 fixture (gangnam yeoksam 737, jiga=67300000)
        let raw = serde_json::json!({
            "response": {
                "service": {"name": "data", "version": "2.0"},
                "status": "OK",
                "record": {"total": "1", "current": "1"},
                "page": {"total": "1", "current": "1", "size": "10"},
                "result": {
                    "featureCollection": {
                        "type": "FeatureCollection",
                        "features": [{
                            "type": "Feature",
                            "geometry": {"type": "MultiPolygon", "coordinates": []},
                            "properties": {
                                "pnu": "1168010100107370000",
                                "jibun": "737 대",
                                "addr": "서울특별시 강남구 역삼동 737",
                                "jiga": "67300000",
                                "gosi_year": "2025",
                                "gosi_month": "01"
                            }
                        }]
                    }
                }
            }
        });

        let san = AllowlistSanitizer::for_source("vworld_parcel").unwrap();
        let r = san.sanitize(&raw);

        // 공시지가 보존
        let jiga = r.value
            ["response"]["result"]["featureCollection"]["features"][0]["properties"]["jiga"]
            .clone();
        assert_eq!(jiga, "67300000");
        // status envelope 보존
        assert_eq!(r.value["response"]["status"], "OK");
    }
```

- [ ] **Step 2.8.2: Run — verify PASS (no drift, all allowlist paths match)**

```bash
cargo test -p raw-capture-client --lib sources::vworld_parcel::tests::sanitize_real_vworld_fixture
# Expected: ok. 1 passed
```

- [ ] **Step 2.8.3: Run full raw-capture test suite + clippy**

```bash
cargo test -p raw-capture-client --lib
# Expected: all tests pass (sanitizer + capture + sources = 20+ tests)
cargo clippy -p raw-capture-client -- -D warnings
# Expected: no warnings
cargo fmt --check
# Expected: no diff
```

- [ ] **Step 2.8.4: Commit**

```bash
git add crates/data-clients/raw-capture/src/sources/vworld_parcel.rs
git commit -m "test(sp10-5-b-T2): vworld_parcel sanitize real fixture (jiga retained)"
```

---

## Acceptance — T2 완료 기준

- [ ] `cargo test -p raw-capture-client --lib` 20+ test 모두 통과
- [ ] `cargo test -p vworld --lib` 회귀 0 (literal → const 변경만이라 logic 동일)
- [ ] `cargo clippy --workspace -- -D warnings` 통과
- [ ] migration 30012 forward + rollback 검증 완료
- [ ] `parcel_external_data` 의 모든 'vworld' row 가 'vworld_parcel' 로 backfill됨
- [ ] `RAW_CAPTURE_SOURCE` const 가 `crates/data-clients/vworld/src/reader.rs` 에 정의됨
- [ ] T3 에서 사용할 인터페이스 export: `sources::data_go_kr_building::{SOURCE_ID, BUILDING_ALLOWLIST}`, `sources::vworld_parcel::{SOURCE_ID, VWORLD_PARCEL_ALLOWLIST}`, `AllowlistSanitizer::for_source`, `SanitizerError`

**다음 task:** [T3-vault-kms-lineage.md](T3-vault-kms-lineage.md) — Two-tier vault migration (30013, 30014) + PgPiiVaultCapture + AWS KMS envelope encryption + DualTierCapture composer.
