# T2 Allowlists Migration - Part 02: Sanitizer Factory And Source Taxonomy Migration

Parent index: [T2 Allowlists Migration](./T2-allowlists-migration.md).


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
