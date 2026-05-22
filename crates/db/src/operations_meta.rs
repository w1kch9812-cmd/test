//! `PgOperationsMetaRepository` — `Postgres` 구현체. **No OCC** + transactional
//! `audit_log`/`outbox_event` 패턴 (SP5-iii T9).
//!
//! `FeaturedContent` + `SystemAlert` 두 Aggregate 를 한 trait 으로 묶어서 처리해요.
//! 둘 다 `version` 컬럼이 없어 OCC 가 필요 없고, `save_*` 는
//! `INSERT … ON CONFLICT (id) DO UPDATE` (조건 없음) 로 신규/업데이트를 모두 처리.
//! 같은 트랜잭션 안에서 `audit_log` row 와 `MutationContext::events` 의 각 도메인
//! 이벤트마다 `outbox_event` row 를 함께 `INSERT` 해 transactional 추적성을 보장해요.
//!
//! 흐름은 SP5-iii T8 [`crates/db/src/listing_report.rs`] 와 동일:
//!
//! 1. `pool.begin()` 으로 트랜잭션 시작
//! 2. `INSERT … ON CONFLICT (id) DO UPDATE` 로 row 저장 (no OCC)
//! 3. `audit_log` row `INSERT` (`resource_kind = 'featured_content'` 또는 `'system_alert'`)
//! 4. `ctx.events` 의 각 이벤트마다 `outbox_event` `INSERT`
//!    (`aggregate_kind = 'featured_content'` 또는 `'system_alert'`)
//! 5. `tx.commit()` — 어느 단계든 실패 시 자동 rollback (`tx` `Drop`)
//!
//! # `find_active_featured`
//!
//! `feature_kind = $1 AND starts_at <= $2 AND $2 < ends_at` 의 half-open
//! interval. weight 내림차순, tie-break `created_at` 오름차순.
//!
//! # `find_unacknowledged_alerts`
//!
//! `acknowledged_at IS NULL` 만 필터 후 severity 우선순위 (critical > error >
//! warning > info), tie-break `created_at` 내림차순. 부분 인덱스
//! `system_alert_unack_idx` 가 `acknowledged_at IS NULL` 조건을 커버해요.

#![allow(clippy::module_name_repetitions)]

mod repository;
mod rows;

use sqlx::PgPool;

/// `FeaturedContent` + `SystemAlert` 두 Aggregate 의 `Postgres` 저장소.
///
/// `save_*` 는 no-OCC + transactional `audit_log`/`outbox_event` 패턴.
#[derive(Debug, Clone)]
pub struct PgOperationsMetaRepository {
    pool: PgPool,
}

impl PgOperationsMetaRepository {
    /// 새 저장소를 만들어요.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
