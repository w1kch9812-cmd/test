# T4: expires_at TTL Constraint + Tokio Cleanup Task

**Goal:** PIPA 보유기간 + 파기 원칙의 자동 강제. `parcel_external_data.expires_at` 을 NOT NULL + CHECK (> fetched_at) + index. Tokio 주기 task 가 `expires_at < now()` row 를 자동 DELETE.

**Spec SSOT:** §7.1 (TTL), §7.2 (NOT NULL constraint), §7.3 (Tokio task), §13 T4 ([design doc](../../specs/2026-05-08-sp10-5-b-backend-data-correctness-design.md))

**T3 inputs:** `parcel_external_data_pii_vault` 테이블 + lineage 컬럼 — 30016 마이그가 두 테이블 모두에 index 추가.

**Files:**

- Create: `migrations/30016_external_data_expires_constraint.sql`
- Create: `services/api/src/cleanup.rs`
- Modify: `services/api/src/main.rs` (Tokio task 등록)
- Modify: `services/api/src/lib.rs` (cleanup module 노출 — T7 에서 분리)

---

## Step 4.1: Migration 30016 — expires_at NOT NULL + CHECK + index

- [ ] **Step 4.1.1: Preflight — NULL expires_at row count check**

```bash
psql gongzzang_dev -c "SELECT count(*) AS null_expires_count FROM parcel_external_data WHERE expires_at IS NULL;"
# Expected: v1 운영 시점 다수 (기존 row 는 NULL 가능)
# 마이그가 backfill 으로 fetched_at + 30 days 채움
```

- [ ] **Step 4.1.2: Create `migrations/30016_external_data_expires_constraint.sql`**

```sql
-- V003_16: expires_at NOT NULL + CHECK > fetched_at + cleanup index.
--
-- Spec SSOT: design.md §7.2.
--
-- PIPA 21조 "보유기간 경과 / 목적 달성 후 파기" 의 시스템 강제. expires_at NULL
-- 저장은 SSS 미완료 — 신규 INSERT 부터 NOT NULL. 기존 NULL row 는 fetched_at +
-- 30 days backfill (가장 짧은 TTL 기준 — 보수적 파기).
--
-- Lock safety: SET NOT NULL 은 PostgreSQL 12+ 에서 instant (기존 CHECK 가
-- 모든 row 에 대해 NOT NULL 보장 시). backfill UPDATE 가 모든 NULL 채운 후
-- SET NOT NULL.

BEGIN;

-- 1. Backfill NULL expires_at (30일 TTL — data_go_kr_building / vworld_parcel 공통)
UPDATE parcel_external_data
   SET expires_at = fetched_at + INTERVAL '30 days'
 WHERE expires_at IS NULL;

-- 2. NOT NULL 강제
ALTER TABLE parcel_external_data
    ALTER COLUMN expires_at SET NOT NULL;

-- 3. CHECK constraint (expires_at 가 fetched_at 보다 미래 강제)
-- parcel_external_data 의 실제 컬럼은 fetched_at (captured_at 아님 — migrations/30006:21)
ALTER TABLE parcel_external_data
    ADD CONSTRAINT check_expires_future
    CHECK (expires_at > fetched_at);

-- 4. Cleanup task 효율 위한 partial index
CREATE INDEX idx_external_data_expires_partial
    ON parcel_external_data (expires_at)
    WHERE expires_at IS NOT NULL;

-- pii_vault 도 cleanup index (vault 의 expires_at 은 30013 마이그 시점에 이미
-- NOT NULL — composite FK 의 ON DELETE CASCADE 가 main 의 DELETE 와 함께 정리)
CREATE INDEX idx_pii_vault_expires_partial
    ON parcel_external_data_pii_vault (expires_at)
    WHERE expires_at IS NOT NULL;

COMMENT ON CONSTRAINT check_expires_future ON parcel_external_data IS
    'PIPA 보유기간 — fetched_at + TTL (source 별 30~90일) 가 expires_at.';

COMMIT;
```

- [ ] **Step 4.1.3: Run forward migration**

```bash
DATABASE_URL=postgres://localhost/gongzzang_dev cargo sqlx migrate run
# Expected: Applied 30016/migrate external_data_expires_constraint
```

- [ ] **Step 4.1.4: Verify constraints + indexes**

```bash
psql gongzzang_dev -c "\d parcel_external_data" | grep -E "expires_at|check_expires"
# Expected: expires_at timestamp with time zone NOT NULL
#           "check_expires_future" CHECK ((expires_at > fetched_at))
psql gongzzang_dev -c "\di idx_external_data_expires_partial"
# Expected: index exists
psql gongzzang_dev -c "SELECT count(*) FROM parcel_external_data WHERE expires_at IS NULL;"
# Expected: 0
```

- [ ] **Step 4.1.5: Commit**

```bash
git add migrations/30016_external_data_expires_constraint.sql
git commit -m "feat(sp10-5-b-T4): migration 30016 — expires_at NOT NULL + CHECK + partial index"
```

---

## Step 4.2: Cleanup task struct + run_once (TDD)

Spec §7.3 — Tokio interval task.

- [ ] **Step 4.2.1: Create `services/api/src/cleanup.rs` with failing test (struct + signature ONLY)**

```rust
//! Tokio interval-based TTL cleanup. `expires_at < now()` row 를 DELETE.
//!
//! Spec SSOT: design.md §7.3.
//!
//! ON DELETE CASCADE 가 vault row 도 함께 정리. cleanup 횟수/삭제 row 수 는
//! `tracing::info!(target = "cleanup.expires_at", ...)` 로 발행.

use sqlx::PgPool;
use std::time::Duration;
use tokio::time::interval;

pub struct CleanupTask {
    pool: PgPool,
    interval: Duration,
}

impl CleanupTask {
    pub fn new(pool: PgPool, interval: Duration) -> Self {
        Self { pool, interval }
    }

    /// 한 사이클 실행. tier1_deleted + tier2_deleted 반환.
    pub async fn run_once(&self) -> Result<CleanupReport, sqlx::Error> {
        unimplemented!("Step 4.2.4 에서 impl")
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CleanupReport {
    pub tier1_deleted: u64,
    pub tier2_deleted: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration as ChronoDuration, Utc};

    #[tokio::test]
    #[ignore = "requires test DB with 30013/30014/30016 migrations"]
    async fn run_once_deletes_expired() {
        let pool = PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
            .await
            .unwrap();
        // Insert: 만료 1개 + 미만료 1개
        let now = Utc::now();
        sqlx::query(
            "INSERT INTO parcel_external_data
                (pnu, source, raw_response, fetched_at, expires_at, sanitizer_version)
             VALUES
                ('1111010100100010001', 'vworld_parcel', '{}', $1, $2, 1),
                ('1111010100100010002', 'vworld_parcel', '{}', $1, $3, 1)
             ON CONFLICT (pnu, source) DO UPDATE
                SET fetched_at = EXCLUDED.fetched_at,
                    expires_at = EXCLUDED.expires_at",
        )
        .bind(now - ChronoDuration::days(40))
        .bind(now - ChronoDuration::days(1)) // expired
        .bind(now + ChronoDuration::days(10)) // alive
        .execute(&pool)
        .await
        .unwrap();

        let task = CleanupTask::new(pool.clone(), Duration::from_secs(3600));
        let report = task.run_once().await.unwrap();
        assert!(report.tier1_deleted >= 1, "expired row 가 삭제되어야 함");

        let remaining: (i64,) = sqlx::query_as(
            "SELECT count(*) FROM parcel_external_data WHERE pnu = '1111010100100010001'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(remaining.0, 0, "expired row 0");

        let alive: (i64,) = sqlx::query_as(
            "SELECT count(*) FROM parcel_external_data WHERE pnu = '1111010100100010002'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(alive.0, 1, "미만료 row 보존");
    }
}
```

- [ ] **Step 4.2.2: Modify `services/api/src/main.rs` or `lib.rs` — expose cleanup module**

`services/api/src/main.rs` 의 최상단에 `mod cleanup;` 추가 (lib.rs 가 없으면 main.rs 가 module owner).

T7 에서 `services/api/src/lib.rs` 분리 시 `pub mod cleanup;` 으로 이동.

- [ ] **Step 4.2.3: Run — verify FAIL (run_once unimplemented)**

```bash
cargo test -p api --lib cleanup::tests::run_once_deletes_expired -- --ignored
# Expected: panic — "not implemented: Step 4.2.4 에서 impl"
```

- [ ] **Step 4.2.4: Implement `run_once`**

Replace `unimplemented!()` with:

```rust
    pub async fn run_once(&self) -> Result<CleanupReport, sqlx::Error> {
        // RLS 우회 (vault DELETE 위해)
        let mut tx = self.pool.begin().await?;
        sqlx::query("SET LOCAL app.role = 'admin'")
            .execute(&mut *tx)
            .await?;

        // Tier 1 — parcel_external_data. ON DELETE CASCADE 가 vault row 도 정리
        let r1 = sqlx::query("DELETE FROM parcel_external_data WHERE expires_at < now()")
            .execute(&mut *tx)
            .await?;

        // Tier 2 — orphaned vault row (FK CASCADE 가 누락된 경우 보장)
        let r2 = sqlx::query(
            "DELETE FROM parcel_external_data_pii_vault WHERE expires_at < now()",
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        let report = CleanupReport {
            tier1_deleted: r1.rows_affected(),
            tier2_deleted: r2.rows_affected(),
        };
        tracing::info!(
            target: "cleanup.expires_at",
            tier1_deleted = report.tier1_deleted,
            tier2_deleted = report.tier2_deleted,
            "TTL cleanup cycle completed"
        );
        Ok(report)
    }
```

- [ ] **Step 4.2.5: Run integration test — verify PASS (test DB required)**

```bash
cargo test -p api --lib cleanup::tests::run_once_deletes_expired -- --ignored
# Expected: ok. 1 passed (with test DB)
```

- [ ] **Step 4.2.6: Commit**

```bash
git add services/api/src/cleanup.rs services/api/src/main.rs
git commit -m "feat(sp10-5-b-T4): CleanupTask::run_once (Tier1 DELETE + CASCADE Tier2)"
```

---

## Step 4.3: spawn_loop + main.rs registration

- [ ] **Step 4.3.1: Append `spawn_loop` to `cleanup.rs`**

```rust
impl CleanupTask {
    /// Tokio interval loop. graceful shutdown 은 CancellationToken 으로 (T7).
    pub async fn spawn_loop(self) {
        let mut ticker = interval(self.interval);
        loop {
            ticker.tick().await;
            match self.run_once().await {
                Ok(report) => {
                    if report.tier1_deleted == 0 && report.tier2_deleted == 0 {
                        tracing::debug!(target: "cleanup.expires_at", "no rows to delete");
                    }
                }
                Err(e) => {
                    tracing::error!(
                        target: "cleanup.expires_at",
                        error = %e,
                        "cleanup cycle failed — will retry next tick"
                    );
                }
            }
        }
    }
}
```

- [ ] **Step 4.3.2: Register Tokio task in `services/api/src/main.rs`**

DB pool 초기화 직후, axum router 시작 *직전* 에 추가:

```rust
// TTL cleanup task — spec §7.3 (PIPA 파기 자동 강제)
let cleanup_task = cleanup::CleanupTask::new(pool.clone(), std::time::Duration::from_secs(3600));
tokio::spawn(cleanup_task.spawn_loop());
```

- [ ] **Step 4.3.3: Verify build**

```bash
cargo check -p api
# Expected: Finished
```

- [ ] **Step 4.3.4: Commit**

```bash
git add services/api/src/cleanup.rs services/api/src/main.rs
git commit -m "feat(sp10-5-b-T4): cleanup task spawn_loop + main.rs registration (1h interval)"
```

---

## Step 4.4: Final workspace verification

- [ ] **Step 4.4.1: Run full test suite + lint**

```bash
cargo test --workspace --lib
# Expected: all unit tests pass (ignored tests skipped without --ignored)
cargo clippy --workspace -- -D warnings
# Expected: no warnings
cargo fmt --check
# Expected: no diff
```

---

## Acceptance — T4 완료 기준

- [ ] `migrations/30016_external_data_expires_constraint.sql` 적용 (NOT NULL + CHECK + 2 partial index)
- [ ] `cargo test -p api --lib cleanup::tests -- --ignored` (test DB 있을 시) — run_once_deletes_expired PASS
- [ ] `services/api/src/cleanup.rs` 의 `CleanupTask::spawn_loop` 가 main.rs 에서 1시간 interval 로 spawn
- [ ] `tracing::info!(target = "cleanup.expires_at", ...)` 가 매 cycle 마다 발행
- [ ] T5 가 사용할 인터페이스 export: `api::cleanup::CleanupTask` (T7 lib.rs 분리 후 public path 정정)

**다음 task:** [T5-building-reader-live.md](T5-building-reader-live.md) — Building reader live wiring (NoOp swap + has_key/fail_fast_production 패턴 유지).
