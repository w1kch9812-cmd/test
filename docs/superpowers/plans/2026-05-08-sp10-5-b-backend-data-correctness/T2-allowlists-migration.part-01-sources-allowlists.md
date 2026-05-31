# T2 Allowlists Migration - Part 01: Sources Module And Source Allowlists

Parent index: [T2 Allowlists Migration](./T2-allowlists-migration.md).

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
