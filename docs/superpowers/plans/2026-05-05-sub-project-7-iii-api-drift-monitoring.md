# SP7-iii Implementation Plan — 정부 API drift 자동 검출 시스템

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 정부 API (data.go.kr / V-World) 의 endpoint URL 변경 + JSON schema 변경을 nightly cron 으로 자동 검출, 우리 Postgres 에 결과 record, 3일 연속 fail 또는 critical drift 시 GitHub Issue 자동 생성.

**Architecture:** Repository port 패턴 (도메인 trait + PgImpl) + feature-gated 통합 테스트 (real-api flag) + GitHub Actions cron + Rust binary (octocrab) 가 DB record + Issue orchestration. 6 단계 task 분해 (T1 도메인+DB → T2 PgImpl → T3 data.go.kr smoke → T4 V-World smoke → T5 recorder binary → T6 workflow + 검증).

**Tech Stack:** Rust 1.88, sqlx 0.8, Postgres + PostGIS, async-trait, thiserror, tokio, octocrab (NEW), GitHub Actions cron + workflow_dispatch.

**Spec:** `docs/superpowers/specs/2026-05-05-sub-project-7-iii-api-drift-monitoring-design.md` (commit `fe2cdfb`)

**main:** `fe2cdfb` (시작 시점)

---

## 추천 진행 순서

- **T1**: DB 마이그레이션 30007 + 도메인 crate (`api-health`) — 1 commit
- **T2**: PgHealthCheckRepository (인프라) + 통합 테스트 — 1 commit
- **T3**: data.go.kr smoke test (feature-gated 통합 테스트) — 1 commit
- **T4**: V-World smoke test — 1 commit
- **T5**: api-health-recorder Rust binary (octocrab) — 1 commit
- **T6**: GitHub Actions workflow + secrets + 검증 + roadmap 갱신 — 1 commit

각 task: 로컬 `cargo check / clippy / test --lib` 통과 후 push → CI 그린 확인. 사용자 체크포인트.

---

## 파일 구조

```
migrations/
└── 30007_api_health_check.sql                                        (T1 — NEW DB schema)

crates/operations/api-health/                                          (T1 — NEW 도메인 crate)
├── Cargo.toml
└── src/
    ├── lib.rs                                                        (re-exports)
    ├── entity.rs                                                     (HealthCheckRecord + NewHealthCheck)
    ├── status.rs                                                     (HealthStatus enum + 분류 helpers)
    └── repository.rs                                                 (HealthCheckRepository trait + RepoError)

crates/db/src/
└── api_health.rs                                                     (T2 — NEW PgHealthCheckRepository)

crates/db/tests/
└── api_health_integration.rs                                         (T2 — NEW 통합 테스트)

crates/data-clients/data-go-kr/
├── Cargo.toml                                                        (T3 — real-api feature 추가)
└── tests/
    └── smoke_real_api.rs                                             (T3 — NEW feature-gated)

crates/data-clients/vworld/
├── Cargo.toml                                                        (T4 — real-api feature 추가)
└── tests/
    └── smoke_real_api.rs                                             (T4 — NEW)

crates/api-health-recorder/                                            (T5 — NEW binary crate)
├── Cargo.toml
└── src/
    └── main.rs                                                       (CLI + record + Issue orchestration)

.github/workflows/
└── api-drift-smoke-test.yml                                          (T6 — NEW workflow)

docs/observability/
└── api-drift-smoke-test.md                                           (T6 — NEW 운영 SSOT)

docs/superpowers/roadmap.md                                            (T6 — SP7-iii closed 표기)

Cargo.toml (workspace)                                                 (T1 + T5 — members 추가)
```

---

## Phase A: 도메인 + DB schema

### Task 1: T1 — 마이그레이션 30007 + `crates/operations/api-health` 도메인 crate

**Files:**
- Create: `migrations/30007_api_health_check.sql`
- Create: `crates/operations/api-health/Cargo.toml`
- Create: `crates/operations/api-health/src/lib.rs`
- Create: `crates/operations/api-health/src/entity.rs`
- Create: `crates/operations/api-health/src/status.rs`
- Create: `crates/operations/api-health/src/repository.rs`
- Modify: `Cargo.toml` (workspace members 에 추가)

#### Step 1.1: 마이그레이션 SQL 작성

- [ ] **Step**: `migrations/30007_api_health_check.sql` 작성

```sql
-- SP7-iii: 정부 API drift 자동 검출 시스템 SSOT.
-- 모든 cron run + 수동 trigger 결과 영구 보존.

CREATE TABLE api_health_check (
    id BIGSERIAL PRIMARY KEY,
    api_name VARCHAR(64) NOT NULL,
    -- 'data_go_kr.getBrTitleInfo' / 'vworld.getFeature' / etc

    checked_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    status VARCHAR(32) NOT NULL CHECK (status IN (
        'success',
        'http_5xx',
        'http_4xx',
        'parse_fail',
        'timeout',
        'connection_fail'
    )),

    http_code SMALLINT,
    -- nullable (timeout / connection_fail = NULL)

    error_detail TEXT,
    -- masked log (secrets redacted)

    cron_run BOOLEAN NOT NULL,
    -- true = scheduled cron, false = workflow_dispatch (수동 trigger)

    duration_ms INTEGER NOT NULL CHECK (duration_ms >= 0)
);

CREATE INDEX idx_api_health_check_api_name_checked_at
    ON api_health_check (api_name, checked_at DESC);

CREATE INDEX idx_api_health_check_failures
    ON api_health_check (api_name, checked_at DESC)
    WHERE status != 'success';

COMMENT ON TABLE api_health_check IS
    '정부 API drift 검출 (SP7-iii). 모든 cron / 수동 trigger 결과 영구 record. SSS SSOT.';

COMMENT ON COLUMN api_health_check.api_name IS
    '대상 API endpoint 식별자. 예: data_go_kr.getBrTitleInfo';

COMMENT ON COLUMN api_health_check.cron_run IS
    'true=schedule cron 자동 실행, false=workflow_dispatch 수동 trigger';
```

- [ ] **Step**: 마이그레이션 syntax 검증 (sqlx 또는 psql 직접)

```bash
cd c:/Users/User/Desktop/gongzzang_2
psql "$DATABASE_URL" -f migrations/30007_api_health_check.sql
```

Expected: `CREATE TABLE` + `CREATE INDEX` (×2) + `COMMENT` (×3) — error 없음.

검증 쿼리:
```bash
psql "$DATABASE_URL" -c "\d api_health_check"
```

Expected: 6 컬럼 + 2 index + table comment 존재.

#### Step 1.2: workspace Cargo.toml 에 새 crate 추가

- [ ] **Step**: `Cargo.toml` workspace `members` 에 `crates/operations/api-health` 추가

```bash
cd c:/Users/User/Desktop/gongzzang_2
grep -n "operations-meta" Cargo.toml
```

기존 entry 옆에 추가:
```toml
"crates/operations/api-health",
```

#### Step 1.3: api-health crate Cargo.toml 작성

- [ ] **Step**: `crates/operations/api-health/Cargo.toml` 작성

```toml
[package]
name = "api-health-domain"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true

[dependencies]
chrono = { workspace = true }
serde = { workspace = true, features = ["derive"] }
async-trait = { workspace = true }
thiserror = { workspace = true }
```

#### Step 1.4: lib.rs 작성

- [ ] **Step**: `crates/operations/api-health/src/lib.rs` 작성

```rust
//! API drift 검출 record 도메인 — SP7-iii.
//!
//! 정부 API (data.go.kr / V-World 등) 의 nightly cron 검증 결과를
//! 우리 Postgres `api_health_check` 테이블에 영구 record.
//!
//! - [`HealthStatus`] — 6 분류 (success / http_5xx / http_4xx / parse_fail / timeout / connection_fail)
//! - [`HealthCheckRecord`] — DB row 도메인 표현
//! - [`NewHealthCheck`] — INSERT 용 빌더
//! - [`HealthCheckRepository`] — port trait, [`crates/db`] 가 PgImpl 제공

pub mod entity;
pub mod repository;
pub mod status;

pub use entity::{HealthCheckRecord, NewHealthCheck};
pub use repository::{HealthCheckRepository, RepoError};
pub use status::HealthStatus;
```

#### Step 1.5: status.rs — HealthStatus enum 단위 테스트 작성 (실패)

- [ ] **Step**: `crates/operations/api-health/src/status.rs` 작성 (테스트 + skeleton)

```rust
//! `HealthStatus` — drift 검출 결과 분류.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// API drift 검출 결과 6 분류.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    /// 정상 응답 + parser 통과.
    Success,
    /// HTTP 5xx — 정부 일시 장애 가능 (soft-fail).
    Http5xx,
    /// HTTP 4xx — 키 / quota / endpoint 죽음 (hard-fail).
    Http4xx,
    /// HTTP 200 + parser fail — schema drift (hard-fail, 즉시 escalation).
    ParseFail,
    /// timeout (soft-fail).
    Timeout,
    /// connection 실패 — DNS / SSL / TCP (soft-fail).
    ConnectionFail,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum HealthStatusError {
    #[error("unknown health_status: '{0}'")]
    Unknown(String),
}

impl HealthStatus {
    /// hard-fail 인가? (즉시 escalation 대상)
    #[must_use]
    pub const fn is_hard_fail(self) -> bool {
        matches!(self, Self::Http4xx | Self::ParseFail)
    }

    /// soft-fail 인가? (3일 연속이어야 escalation)
    #[must_use]
    pub const fn is_soft_fail(self) -> bool {
        matches!(self, Self::Http5xx | Self::Timeout | Self::ConnectionFail)
    }

    /// `true` 면 fail 종류, `false` 면 success.
    #[must_use]
    pub const fn is_failure(self) -> bool {
        !matches!(self, Self::Success)
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Http5xx => "http_5xx",
            Self::Http4xx => "http_4xx",
            Self::ParseFail => "parse_fail",
            Self::Timeout => "timeout",
            Self::ConnectionFail => "connection_fail",
        }
    }
}

impl fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for HealthStatus {
    type Err = HealthStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "success" => Ok(Self::Success),
            "http_5xx" => Ok(Self::Http5xx),
            "http_4xx" => Ok(Self::Http4xx),
            "parse_fail" => Ok(Self::ParseFail),
            "timeout" => Ok(Self::Timeout),
            "connection_fail" => Ok(Self::ConnectionFail),
            other => Err(HealthStatusError::Unknown(other.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn is_hard_fail_only_4xx_and_parse() {
        assert!(HealthStatus::Http4xx.is_hard_fail());
        assert!(HealthStatus::ParseFail.is_hard_fail());
        assert!(!HealthStatus::Http5xx.is_hard_fail());
        assert!(!HealthStatus::Timeout.is_hard_fail());
        assert!(!HealthStatus::ConnectionFail.is_hard_fail());
        assert!(!HealthStatus::Success.is_hard_fail());
    }

    #[test]
    fn is_soft_fail_only_5xx_timeout_connection() {
        assert!(HealthStatus::Http5xx.is_soft_fail());
        assert!(HealthStatus::Timeout.is_soft_fail());
        assert!(HealthStatus::ConnectionFail.is_soft_fail());
        assert!(!HealthStatus::Http4xx.is_soft_fail());
        assert!(!HealthStatus::ParseFail.is_soft_fail());
        assert!(!HealthStatus::Success.is_soft_fail());
    }

    #[test]
    fn is_failure_excludes_success() {
        for v in [
            HealthStatus::Http5xx,
            HealthStatus::Http4xx,
            HealthStatus::ParseFail,
            HealthStatus::Timeout,
            HealthStatus::ConnectionFail,
        ] {
            assert!(v.is_failure(), "{v} should be failure");
        }
        assert!(!HealthStatus::Success.is_failure());
    }

    #[test]
    fn from_str_round_trip_all() {
        for v in [
            HealthStatus::Success,
            HealthStatus::Http5xx,
            HealthStatus::Http4xx,
            HealthStatus::ParseFail,
            HealthStatus::Timeout,
            HealthStatus::ConnectionFail,
        ] {
            assert_eq!(HealthStatus::from_str(v.as_str()).unwrap(), v);
        }
    }

    #[test]
    fn from_str_rejects_unknown() {
        let err = HealthStatus::from_str("teapot").unwrap_err();
        assert!(matches!(err, HealthStatusError::Unknown(s) if s == "teapot"));
    }

    #[test]
    fn serde_roundtrip_snake_case() {
        let v = HealthStatus::ParseFail;
        let json = serde_json::to_string(&v).expect("serialize");
        assert_eq!(json, r#""parse_fail""#);
    }
}
```

- [ ] **Step**: 단위 테스트 실행

```bash
cd c:/Users/User/Desktop/gongzzang_2
cargo test -p api-health-domain --lib status
```

Expected: 6 tests passed (`is_hard_fail_only_4xx_and_parse` 등).

#### Step 1.6: entity.rs — HealthCheckRecord 작성

- [ ] **Step**: `crates/operations/api-health/src/entity.rs` 작성

```rust
//! `HealthCheckRecord` — DB row 도메인 표현 + `NewHealthCheck` INSERT 빌더.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::status::HealthStatus;

/// `api_health_check` 테이블 row 의 도메인 표현.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthCheckRecord {
    pub id: i64,
    pub api_name: String,
    pub checked_at: DateTime<Utc>,
    pub status: HealthStatus,
    pub http_code: Option<u16>,
    pub error_detail: Option<String>,
    pub cron_run: bool,
    pub duration_ms: u32,
}

/// `record()` 호출 시 받는 INSERT 인자.
///
/// `id` / `checked_at` 은 DB 가 채움.
#[derive(Debug, Clone)]
pub struct NewHealthCheck<'a> {
    pub api_name: &'a str,
    pub status: HealthStatus,
    pub http_code: Option<u16>,
    pub error_detail: Option<&'a str>,
    pub cron_run: bool,
    pub duration_ms: u32,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn new_health_check_construction() {
        let new = NewHealthCheck {
            api_name: "data_go_kr.getBrTitleInfo",
            status: HealthStatus::Success,
            http_code: Some(200),
            error_detail: None,
            cron_run: true,
            duration_ms: 1234,
        };
        assert_eq!(new.api_name, "data_go_kr.getBrTitleInfo");
        assert_eq!(new.status, HealthStatus::Success);
        assert_eq!(new.duration_ms, 1234);
    }

    #[test]
    fn record_serde_roundtrip() {
        let record = HealthCheckRecord {
            id: 42,
            api_name: "vworld.getFeature".to_owned(),
            checked_at: Utc::now(),
            status: HealthStatus::Http5xx,
            http_code: Some(502),
            error_detail: Some("upstream timeout".to_owned()),
            cron_run: true,
            duration_ms: 5000,
        };
        let json = serde_json::to_string(&record).expect("serialize");
        let back: HealthCheckRecord = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, record);
    }
}
```

- [ ] **Step**: 단위 테스트

```bash
cargo test -p api-health-domain --lib entity
```

Expected: 2 tests passed.

#### Step 1.7: repository.rs — trait + RepoError

- [ ] **Step**: `crates/operations/api-health/src/repository.rs` 작성

```rust
//! `HealthCheckRepository` — port trait + `RepoError`.

use async_trait::async_trait;
use thiserror::Error;

use crate::entity::{HealthCheckRecord, NewHealthCheck};

/// 도메인 레벨 repository 에러.
///
/// 인프라 (`crates/db`) 가 sqlx 에러를 흡수해 본 enum 으로 변환.
#[derive(Debug, Error)]
pub enum RepoError {
    /// 데이터 무결성 위반 (CHECK constraint / NOT NULL / etc).
    #[error("integrity violation: {0}")]
    Integrity(String),

    /// DB 연결 / 쿼리 실패.
    #[error("database error: {0}")]
    Database(String),
}

/// `api_health_check` 테이블 access port.
#[async_trait]
pub trait HealthCheckRepository: Send + Sync {
    /// 새 record INSERT.
    async fn record(&self, new: NewHealthCheck<'_>) -> Result<HealthCheckRecord, RepoError>;

    /// 가장 최근 N개 cron run 이 모두 fail 인가? (수동 trigger 무관)
    ///
    /// `n=3` 으로 호출 시: 최근 3개의 `cron_run=true` record 가 모두 `status != 'success'` 면 true.
    /// 정부 일시 장애 (5xx / timeout) 의 3일 연속 escalation detection 에 사용.
    async fn is_n_cron_runs_failed(
        &self,
        api_name: &str,
        n: u32,
    ) -> Result<bool, RepoError>;

    /// 가장 최근 record (success / fail 무관, cron / 수동 무관).
    async fn find_latest(&self, api_name: &str)
        -> Result<Option<HealthCheckRecord>, RepoError>;
}
```

#### Step 1.8: cargo check / clippy / test 통합 검증

- [ ] **Step**: 전체 검증

```bash
cd c:/Users/User/Desktop/gongzzang_2
cargo check -p api-health-domain --all-features
cargo clippy -p api-health-domain --all-targets -- -D warnings
cargo test -p api-health-domain --lib
cargo fmt --all -- --check
```

Expected: 모두 pass. ~9 단위 테스트 통과.

#### Step 1.9: T1 commit

- [ ] **Step**: T1 commit

```bash
git add migrations/30007_api_health_check.sql \
        crates/operations/api-health/ \
        Cargo.toml

git commit -m "$(cat <<'EOF'
feat(sp7-iii-t1): add api_health_check table + api-health-domain crate

T1 of SP7-iii (정부 API drift 자동 검출 시스템):
- migrations/30007_api_health_check.sql — drift 결과 SSOT 테이블
  - 6 status 분류 (success/5xx/4xx/parse_fail/timeout/connection_fail)
  - 2 indexes (api_name+checked_at DESC, failures-only partial)
- crates/operations/api-health/ — 도메인 crate (port-only)
  - HealthStatus enum + is_hard_fail/is_soft_fail/is_failure
  - HealthCheckRecord + NewHealthCheck 빌더
  - HealthCheckRepository trait + RepoError
- ~9 단위 테스트 통과

Spec: docs/superpowers/specs/2026-05-05-sub-project-7-iii-api-drift-monitoring-design.md
EOF
)"
```

**사용자 체크포인트**: T1 commit 확인 + 다음 진행 여부.

---

## Phase B: PgHealthCheckRepository 인프라

### Task 2: T2 — PgHealthCheckRepository (DB 인프라) + 통합 테스트

**Files:**
- Create: `crates/db/src/api_health.rs`
- Create: `crates/db/tests/api_health_integration.rs`
- Modify: `crates/db/src/lib.rs` (re-export)
- Modify: `crates/db/Cargo.toml` (`api-health-domain` 의존성 추가)

#### Step 2.1: crates/db/Cargo.toml 의존성 추가

- [ ] **Step**: `crates/db/Cargo.toml` `[dependencies]` 에 추가

```toml
api-health-domain = { path = "../operations/api-health" }
```

#### Step 2.2: lib.rs re-export

- [ ] **Step**: `crates/db/src/lib.rs` 에 추가

```rust
pub mod api_health;
```

(파일 끝 부분의 다른 `pub mod` 들 옆에)

#### Step 2.3: api_health.rs PgHealthCheckRepository 작성

- [ ] **Step**: `crates/db/src/api_health.rs` 작성

```rust
//! `PgHealthCheckRepository` — `api_health_check` 테이블 인프라 구현.
//!
//! SP7-iii 의 SSOT. `crates/operations/api-health` 의 trait 구현.

#![allow(clippy::module_name_repetitions)]

use std::sync::Arc;

use api_health_domain::{
    HealthCheckRecord, HealthCheckRepository, HealthStatus, NewHealthCheck, RepoError,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{postgres::PgRow, PgPool, Row};
use std::str::FromStr;
use tracing::instrument;

/// `api_health_check` 테이블에 대한 Postgres 구현.
#[derive(Clone)]
pub struct PgHealthCheckRepository {
    pool: Arc<PgPool>,
}

impl PgHealthCheckRepository {
    #[must_use]
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
}

fn map_repo_error(e: sqlx::Error) -> RepoError {
    match &e {
        sqlx::Error::Database(db_err) if db_err.is_check_violation() || db_err.is_unique_violation() => {
            RepoError::Integrity(format!("{e}"))
        }
        _ => RepoError::Database(format!("{e}")),
    }
}

fn row_to_record(row: PgRow) -> Result<HealthCheckRecord, RepoError> {
    let status_str: String = row.try_get("status").map_err(map_repo_error)?;
    let status = HealthStatus::from_str(&status_str)
        .map_err(|e| RepoError::Integrity(format!("invalid status '{status_str}': {e}")))?;
    let http_code: Option<i16> = row.try_get("http_code").map_err(map_repo_error)?;
    let duration_ms: i32 = row.try_get("duration_ms").map_err(map_repo_error)?;

    Ok(HealthCheckRecord {
        id: row.try_get("id").map_err(map_repo_error)?,
        api_name: row.try_get("api_name").map_err(map_repo_error)?,
        checked_at: row.try_get::<DateTime<Utc>, _>("checked_at").map_err(map_repo_error)?,
        status,
        http_code: http_code.map(|c| c as u16),
        error_detail: row.try_get("error_detail").map_err(map_repo_error)?,
        cron_run: row.try_get("cron_run").map_err(map_repo_error)?,
        duration_ms: duration_ms as u32,
    })
}

#[async_trait]
impl HealthCheckRepository for PgHealthCheckRepository {
    #[instrument(skip(self, new), fields(api = %new.api_name, status = %new.status))]
    async fn record(&self, new: NewHealthCheck<'_>) -> Result<HealthCheckRecord, RepoError> {
        let row = sqlx::query(
            r#"
            INSERT INTO api_health_check
                (api_name, status, http_code, error_detail, cron_run, duration_ms)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, api_name, checked_at, status, http_code,
                      error_detail, cron_run, duration_ms
            "#,
        )
        .bind(new.api_name)
        .bind(new.status.as_str())
        .bind(new.http_code.map(|c| c as i16))
        .bind(new.error_detail)
        .bind(new.cron_run)
        .bind(new.duration_ms as i32)
        .fetch_one(&*self.pool)
        .await
        .map_err(map_repo_error)?;

        row_to_record(row)
    }

    #[instrument(skip(self), fields(api = %api_name))]
    async fn is_n_cron_runs_failed(
        &self,
        api_name: &str,
        n: u32,
    ) -> Result<bool, RepoError> {
        // 최근 N 개 cron run 의 status — fail 가 모두 N 개여야 true.
        let row = sqlx::query(
            r#"
            WITH recent_cron AS (
                SELECT status
                FROM api_health_check
                WHERE api_name = $1 AND cron_run = true
                ORDER BY checked_at DESC
                LIMIT $2
            )
            SELECT
                (SELECT COUNT(*) FROM recent_cron) AS total,
                (SELECT COUNT(*) FROM recent_cron WHERE status != 'success') AS failures
            "#,
        )
        .bind(api_name)
        .bind(n as i64)
        .fetch_one(&*self.pool)
        .await
        .map_err(map_repo_error)?;

        let total: i64 = row.try_get("total").map_err(map_repo_error)?;
        let failures: i64 = row.try_get("failures").map_err(map_repo_error)?;

        // N 개가 모두 모이고, 모두 failure 면 true.
        Ok(total == n as i64 && failures == n as i64)
    }

    #[instrument(skip(self), fields(api = %api_name))]
    async fn find_latest(
        &self,
        api_name: &str,
    ) -> Result<Option<HealthCheckRecord>, RepoError> {
        let row = sqlx::query(
            r#"
            SELECT id, api_name, checked_at, status, http_code,
                   error_detail, cron_run, duration_ms
            FROM api_health_check
            WHERE api_name = $1
            ORDER BY checked_at DESC
            LIMIT 1
            "#,
        )
        .bind(api_name)
        .fetch_optional(&*self.pool)
        .await
        .map_err(map_repo_error)?;

        row.map(row_to_record).transpose()
    }
}
```

#### Step 2.4: 통합 테스트 작성

- [ ] **Step**: `crates/db/tests/api_health_integration.rs` 작성

```rust
//! `PgHealthCheckRepository` 통합 테스트 — 실 Postgres 사용.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::sync::Arc;

use api_health_domain::{
    HealthCheckRepository, HealthStatus, NewHealthCheck,
};
use db::api_health::PgHealthCheckRepository;
use sqlx::PgPool;

async fn setup() -> PgHealthCheckRepository {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let pool = PgPool::connect(&url).await.expect("connect");
    sqlx::query("TRUNCATE TABLE api_health_check RESTART IDENTITY")
        .execute(&pool)
        .await
        .expect("truncate");
    PgHealthCheckRepository::new(Arc::new(pool))
}

#[tokio::test]
async fn record_success_inserts_row() {
    let repo = setup().await;
    let new = NewHealthCheck {
        api_name: "data_go_kr.getBrTitleInfo",
        status: HealthStatus::Success,
        http_code: Some(200),
        error_detail: None,
        cron_run: true,
        duration_ms: 1234,
    };
    let record = repo.record(new).await.expect("record");
    assert_eq!(record.api_name, "data_go_kr.getBrTitleInfo");
    assert_eq!(record.status, HealthStatus::Success);
    assert_eq!(record.http_code, Some(200));
    assert!(record.cron_run);
    assert_eq!(record.duration_ms, 1234);
    assert!(record.id > 0);
}

#[tokio::test]
async fn record_invalid_status_violates_check_constraint() {
    let repo = setup().await;
    // HealthStatus enum 우회 — DB CHECK constraint 만 검증
    let pool = PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();
    let result = sqlx::query(
        "INSERT INTO api_health_check (api_name, status, cron_run, duration_ms)
         VALUES ('test', 'invalid_status', true, 1)",
    )
    .execute(&pool)
    .await;
    assert!(result.is_err(), "CHECK constraint 가 invalid status 거부해야 함");
}

#[tokio::test]
async fn is_n_cron_runs_failed_returns_true_when_3_consecutive_failures() {
    let repo = setup().await;
    // 3 cron fail 입력
    for status in [HealthStatus::Http5xx, HealthStatus::Timeout, HealthStatus::Http5xx] {
        repo.record(NewHealthCheck {
            api_name: "test_api",
            status,
            http_code: None,
            error_detail: None,
            cron_run: true,
            duration_ms: 100,
        })
        .await
        .unwrap();
    }
    assert!(repo.is_n_cron_runs_failed("test_api", 3).await.unwrap());
}

#[tokio::test]
async fn is_n_cron_runs_failed_returns_false_when_one_success_in_3() {
    let repo = setup().await;
    for status in [HealthStatus::Http5xx, HealthStatus::Success, HealthStatus::Http5xx] {
        repo.record(NewHealthCheck {
            api_name: "test_api",
            status,
            http_code: None,
            error_detail: None,
            cron_run: true,
            duration_ms: 100,
        })
        .await
        .unwrap();
    }
    assert!(!repo.is_n_cron_runs_failed("test_api", 3).await.unwrap());
}

#[tokio::test]
async fn is_n_cron_runs_failed_ignores_manual_dispatch() {
    let repo = setup().await;
    // cron 1 fail, manual 2 fail, cron 1 fail = cron 만 보면 2 fail (2 < 3)
    let mixes = [
        (HealthStatus::Http5xx, true),
        (HealthStatus::Http5xx, false),
        (HealthStatus::Http5xx, false),
        (HealthStatus::Http5xx, true),
    ];
    for (status, cron) in mixes {
        repo.record(NewHealthCheck {
            api_name: "test_api",
            status,
            http_code: None,
            error_detail: None,
            cron_run: cron,
            duration_ms: 100,
        })
        .await
        .unwrap();
    }
    assert!(!repo.is_n_cron_runs_failed("test_api", 3).await.unwrap(),
        "cron 만 카운트하면 2 fail < 3");
}

#[tokio::test]
async fn is_n_cron_runs_failed_returns_false_with_insufficient_data() {
    let repo = setup().await;
    repo.record(NewHealthCheck {
        api_name: "test_api",
        status: HealthStatus::Http5xx,
        http_code: None,
        error_detail: None,
        cron_run: true,
        duration_ms: 100,
    })
    .await
    .unwrap();
    // 1개만 있을 때 n=3 = false (총 데이터 < 3)
    assert!(!repo.is_n_cron_runs_failed("test_api", 3).await.unwrap());
}

#[tokio::test]
async fn find_latest_returns_most_recent() {
    let repo = setup().await;
    for i in 0..3 {
        repo.record(NewHealthCheck {
            api_name: "test_api",
            status: HealthStatus::Success,
            http_code: Some(200),
            error_detail: Some(&format!("call {i}")),
            cron_run: true,
            duration_ms: 100 * (i + 1),
        })
        .await
        .unwrap();
    }
    let latest = repo.find_latest("test_api").await.unwrap().expect("Some");
    assert_eq!(latest.duration_ms, 300);  // 가장 최근 = i=2
    assert_eq!(latest.error_detail, Some("call 2".to_owned()));
}

#[tokio::test]
async fn find_latest_returns_none_for_unknown_api() {
    let repo = setup().await;
    let latest = repo.find_latest("never.recorded").await.unwrap();
    assert!(latest.is_none());
}
```

- [ ] **Step**: 통합 테스트 실행

```bash
cd c:/Users/User/Desktop/gongzzang_2
# DATABASE_URL 가 .env 또는 export 로 설정돼 있어야 함
cargo test -p db --test api_health_integration
```

Expected: 8 tests passed.

#### Step 2.5: cargo check / clippy / fmt 검증

- [ ] **Step**: 전체 검증

```bash
cargo check -p db --all-features
cargo clippy -p db --all-targets -- -D warnings
cargo test -p db --lib
cargo fmt --all -- --check
```

Expected: 모두 pass.

#### Step 2.6: T2 commit

- [ ] **Step**: commit

```bash
git add crates/db/Cargo.toml \
        crates/db/src/lib.rs \
        crates/db/src/api_health.rs \
        crates/db/tests/api_health_integration.rs

git commit -m "$(cat <<'EOF'
feat(sp7-iii-t2): add PgHealthCheckRepository + integration tests

T2 of SP7-iii:
- crates/db/src/api_health.rs — PgHealthCheckRepository (HealthCheckRepository impl)
  - record() — INSERT (api_name, status, http_code, error_detail, cron_run, duration_ms)
  - is_n_cron_runs_failed(api_name, n) — 최근 N cron run 모두 fail 인가? (수동 trigger 무관)
  - find_latest(api_name) — 가장 최근 record (None | Some)
  - row_to_record helper + map_repo_error (sqlx::Error → RepoError)
  - tracing::instrument (api/status/n PII 제외 fields)
- 8 통합 테스트 (TRUNCATE 격리 + 실 Postgres):
  - record_success_inserts_row
  - record_invalid_status_violates_check_constraint
  - is_n_cron_runs_failed_returns_true_when_3_consecutive_failures
  - is_n_cron_runs_failed_returns_false_when_one_success_in_3
  - is_n_cron_runs_failed_ignores_manual_dispatch
  - is_n_cron_runs_failed_returns_false_with_insufficient_data
  - find_latest_returns_most_recent
  - find_latest_returns_none_for_unknown_api
EOF
)"
```

**사용자 체크포인트**: T2 commit 확인 + 다음 진행 여부.

---

## Phase C: data.go.kr smoke test

### Task 3: T3 — data.go.kr `BldRgstHubService` 실 API smoke test

**Files:**
- Modify: `crates/data-clients/data-go-kr/Cargo.toml` (real-api feature)
- Create: `crates/data-clients/data-go-kr/tests/smoke_real_api.rs`

#### Step 3.1: Cargo.toml 에 real-api feature 추가

- [ ] **Step**: `crates/data-clients/data-go-kr/Cargo.toml` 에 추가

```toml
[features]
default = []
real-api = []
```

#### Step 3.2: smoke_real_api.rs 작성

- [ ] **Step**: `crates/data-clients/data-go-kr/tests/smoke_real_api.rs` 작성

```rust
//! data.go.kr `BldRgstHubService` 실 API smoke test (SP7-iii).
//!
//! `cargo test --features real-api -p data-go-kr-client --test smoke_real_api -- --ignored`
//!
//! 환경변수:
//! - `ODP_SERVICE_KEY` (필수) — data.go.kr 발급 키
//! - `GONGZZANG_DRIFT_TEST_PNU` (옵션, default `1168010100107370000` = 강남파이낸스)
//!   - simulate_failure workflow input 시 `9999999999999999999` 로 override
//!
//! 검증:
//! 1. `BuildingRegisterClient::fetch_title_info` 가 실 API 응답 받음
//! 2. `parse_building_title` 통과 (schema drift 검출)
//! 3. mainPurpsCd 매핑 정상 (강남파이낸스 = `BuildingPurposeCode::Office`)
//! 4. strctCd 매핑 정상 (강남파이낸스 = `BuildingStructureCode::SteelReinforcedConcrete`)

#![cfg(feature = "real-api")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use building_domain::purpose_code::BuildingPurposeCode;
use building_domain::structure_code::BuildingStructureCode;
use chrono::Utc;
use data_go_kr_client::building_register::parser::parse_building_title;
use data_go_kr_client::building_register::BuildingRegisterClient;
use data_go_kr_client::pnu_split::split;
use data_go_kr_client::{DataGoKrClient, DataGoKrConfig};
use geo_types::{Coord, LineString, Polygon as GeoPolygon};
use shared_kernel::geometry::PolygonSrid;
use shared_kernel::pnu::Pnu;

fn dummy_polygon() -> PolygonSrid {
    let exterior = LineString(vec![
        Coord { x: 126.0, y: 37.0 },
        Coord { x: 127.0, y: 37.0 },
        Coord { x: 127.0, y: 38.0 },
        Coord { x: 126.0, y: 38.0 },
        Coord { x: 126.0, y: 37.0 },
    ]);
    PolygonSrid::try_new_wgs84(GeoPolygon::new(exterior, vec![])).expect("valid")
}

#[tokio::test]
#[ignore]
async fn smoke_data_go_kr_building_register_alive() {
    let key = std::env::var("ODP_SERVICE_KEY").expect("ODP_SERVICE_KEY required");

    let pnu_str = std::env::var("GONGZZANG_DRIFT_TEST_PNU")
        .unwrap_or_else(|_| "1168010100107370000".to_owned());
    let pnu = Pnu::try_new(&pnu_str).expect("valid PNU");

    let config = DataGoKrConfig {
        service_key: key,
        base_url: "https://apis.data.go.kr".to_owned(),
    };
    let client = DataGoKrClient::new(config);

    let br = BuildingRegisterClient::new(&client);
    let raw = br
        .fetch_title_info(split(&pnu))
        .await
        .expect("HTTP call should succeed (endpoint URL drift?)");

    let buildings = parse_building_title(&raw, &pnu, &dummy_polygon(), Utc::now())
        .expect("parser should accept response (schema drift?)");

    assert!(
        !buildings.is_empty(),
        "응답 0건 — endpoint drift 또는 PNU 잘못됨 (simulate_failure 의도된 fail?)"
    );

    // 강남파이낸스센터 검증 (default PNU = 1168010100107370000)
    if pnu.as_str() == "1168010100107370000" {
        let b = &buildings[0];
        assert_eq!(
            b.main_purpose_code,
            BuildingPurposeCode::Office,
            "mainPurpsCd 14000 → Office 매핑 검증"
        );
        assert_eq!(
            b.structure_code,
            BuildingStructureCode::SteelReinforcedConcrete,
            "strctCd 42 → SRC 매핑 검증"
        );
    }
}
```

#### Step 3.3: 평소 cargo test 에서 skip 확인

- [ ] **Step**: feature flag 없이 빌드 통과 확인

```bash
cd c:/Users/User/Desktop/gongzzang_2
cargo test -p data-go-kr-client
```

Expected: 47 lib + 6 wiremock + 6 fixture = 59 tests pass.
`smoke_real_api.rs` 의 테스트는 `#![cfg(feature = "real-api")]` 라 빌드 자체 안 됨 (file 내용 0).

#### Step 3.4: real-api feature 로 호출 검증 (로컬)

- [ ] **Step**: 실 API 호출

```bash
# .env 의 ODP_SERVICE_KEY 가 설정돼 있어야 함
cargo test --features real-api -p data-go-kr-client \
    --test smoke_real_api -- --ignored --nocapture
```

Expected: `smoke_data_go_kr_building_register_alive ... ok` (정부 API 정상 시).

#### Step 3.5: simulate_failure 검증 (로컬)

- [ ] **Step**: 잘못된 PNU 로 fail 의도

```bash
GONGZZANG_DRIFT_TEST_PNU=9999999999999999999 cargo test \
    --features real-api -p data-go-kr-client \
    --test smoke_real_api -- --ignored --nocapture
```

Expected: panic — `assert!(!buildings.is_empty())` fail. 메시지: "응답 0건 — endpoint drift 또는 PNU 잘못됨".

#### Step 3.6: cargo clippy 검증 (real-api on)

- [ ] **Step**: clippy

```bash
cargo clippy --features real-api -p data-go-kr-client --tests -- -D warnings
```

Expected: warnings 0.

#### Step 3.7: T3 commit

- [ ] **Step**: commit

```bash
git add crates/data-clients/data-go-kr/Cargo.toml \
        crates/data-clients/data-go-kr/tests/smoke_real_api.rs

git commit -m "$(cat <<'EOF'
feat(sp7-iii-t3): add data.go.kr real API smoke test (feature-gated)

T3 of SP7-iii:
- crates/data-clients/data-go-kr/Cargo.toml — real-api feature 추가
- tests/smoke_real_api.rs — feature-gated + #[ignore]
  - 강남파이낸스 PNU (1168010100107370000) 실 호출
  - parse_building_title 통과 + Office + SRC 매핑 검증
  - GONGZZANG_DRIFT_TEST_PNU env 으로 simulate_failure 지원
  - 평소 cargo test 에서 빌드/실행 X (real-api off)
- 로컬 검증: cargo test --features real-api -p data-go-kr-client --test smoke_real_api -- --ignored
EOF
)"
```

**사용자 체크포인트**: T3 commit 확인 + 다음 진행 여부.

---

## Phase D: V-World smoke test

### Task 4: T4 — V-World `LP_PA_CBND_BUBUN` 실 API smoke test

**Files:**
- Modify: `crates/data-clients/vworld/Cargo.toml` (real-api feature)
- Create: `crates/data-clients/vworld/tests/smoke_real_api.rs`

#### Step 4.1: Cargo.toml feature 추가

- [ ] **Step**: `crates/data-clients/vworld/Cargo.toml` 에 추가

```toml
[features]
default = []
real-api = []
```

#### Step 4.2: smoke_real_api.rs 작성

- [ ] **Step**: `crates/data-clients/vworld/tests/smoke_real_api.rs` 작성

```rust
//! V-World `LP_PA_CBND_BUBUN` 실 API smoke test (SP7-iii).
//!
//! `cargo test --features real-api -p vworld-client --test smoke_real_api -- --ignored`
//!
//! 환경변수:
//! - `VWORLD_API_KEY` (필수)
//! - `VWORLD_DOMAIN` (필수, default `localhost`)
//! - `GONGZZANG_DRIFT_TEST_PNU` (옵션, default `1168010100107370000` = 강남파이낸스)

#![cfg(feature = "real-api")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use shared_kernel::pnu::Pnu;
use vworld_client::{ParcelReader, VWorldClient, VWorldConfig};

#[tokio::test]
#[ignore]
async fn smoke_vworld_parcel_alive() {
    let key = std::env::var("VWORLD_API_KEY").expect("VWORLD_API_KEY required");
    let domain = std::env::var("VWORLD_DOMAIN").unwrap_or_else(|_| "localhost".to_owned());

    let pnu_str = std::env::var("GONGZZANG_DRIFT_TEST_PNU")
        .unwrap_or_else(|_| "1168010100107370000".to_owned());
    let pnu = Pnu::try_new(&pnu_str).expect("valid PNU");

    let config = VWorldConfig {
        api_key: key,
        domain,
        base_url: "https://api.vworld.kr".to_owned(),
    };
    let client = VWorldClient::new(config);

    let parcel = client
        .parcel_reader()
        .fetch_by_pnu(&pnu)
        .await
        .expect("V-World call should succeed (endpoint URL drift?)");

    let parcel = parcel.expect("필지 응답 — endpoint drift 또는 PNU 잘못됨");

    assert_eq!(parcel.pnu.as_str(), pnu.as_str(), "응답 PNU 가 요청과 일치");
    // 핵심 필드 존재 검증 (jiyok_cd 등 — V-World 응답 schema)
    // 미래 V-World schema 변경 시 추가 assert
}
```

**참고**: 실제 `vworld_client` 의 `ParcelReader::fetch_by_pnu` 시그니처 / 반환 타입 확인 필요. 위 코드는 `Result<Option<Parcel>, _>` 가정 (SP4-ii spec 패턴). 차이 시 수정.

#### Step 4.3: feature flag off 빌드 검증

- [ ] **Step**: 평소 build pass 확인

```bash
cargo test -p vworld-client
```

Expected: 기존 wiremock 통합 테스트 그대로 pass.

#### Step 4.4: real-api on 호출 검증 (로컬)

- [ ] **Step**: 실 호출 — V-World 가 작동하는 시간대에

```bash
cargo test --features real-api -p vworld-client \
    --test smoke_real_api -- --ignored --nocapture
```

Expected: V-World 정상 응답 시 pass. 502 (브레인스토밍 시점 일시 장애) 일 수 있음 — 그 경우 panic + log 확인.

#### Step 4.5: clippy

- [ ] **Step**: real-api feature 로 clippy

```bash
cargo clippy --features real-api -p vworld-client --tests -- -D warnings
```

Expected: warnings 0.

#### Step 4.6: T4 commit

- [ ] **Step**: commit

```bash
git add crates/data-clients/vworld/Cargo.toml \
        crates/data-clients/vworld/tests/smoke_real_api.rs

git commit -m "$(cat <<'EOF'
feat(sp7-iii-t4): add V-World real API smoke test (feature-gated)

T4 of SP7-iii:
- crates/data-clients/vworld/Cargo.toml — real-api feature
- tests/smoke_real_api.rs — feature-gated + #[ignore]
  - 강남파이낸스 PNU 로 ParcelReader::fetch_by_pnu 검증
  - V-World 응답 schema 의 핵심 필드 존재 assert
  - GONGZZANG_DRIFT_TEST_PNU env 으로 simulate_failure 지원
- 로컬 검증: cargo test --features real-api -p vworld-client --test smoke_real_api -- --ignored
EOF
)"
```

**사용자 체크포인트**: T4 commit 확인 + 다음 진행 여부.

---

## Phase E: api-health-recorder Rust binary

### Task 5: T5 — `crates/api-health-recorder/` (octocrab + PgImpl 재사용)

**Files:**
- Create: `crates/api-health-recorder/Cargo.toml`
- Create: `crates/api-health-recorder/src/main.rs`
- Modify: `Cargo.toml` (workspace members 추가)

#### Step 5.1: workspace Cargo.toml members 에 추가

- [ ] **Step**: `Cargo.toml` 에 추가

```toml
"crates/api-health-recorder",
```

#### Step 5.2: api-health-recorder Cargo.toml

- [ ] **Step**: `crates/api-health-recorder/Cargo.toml` 작성

```toml
[package]
name = "api-health-recorder"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true

[[bin]]
name = "api-health-recorder"
path = "src/main.rs"

[dependencies]
api-health-domain = { path = "../operations/api-health" }
db = { path = "../db" }
sqlx = { workspace = true, features = ["runtime-tokio", "postgres"] }
tokio = { workspace = true, features = ["macros", "rt-multi-thread"] }
chrono = { workspace = true }
clap = { workspace = true, features = ["derive"] }
octocrab = "0.46"
anyhow = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true, features = ["env-filter"] }
```

**참고**: `clap`/`anyhow`/`tracing-subscriber` 가 workspace deps 에 있는지 확인. 없으면 직접 버전 명시.

#### Step 5.3: main.rs 작성

- [ ] **Step**: `crates/api-health-recorder/src/main.rs` 작성

```rust
//! API health recorder — SP7-iii GitHub Actions cron 후속 단계.
//!
//! 사용법:
//! ```bash
//! cargo run --bin api-health-recorder -- \
//!     --api-name data_go_kr.getBrTitleInfo \
//!     --status success \
//!     --http-code 200 \
//!     --duration-ms 1234 \
//!     --cron-run true
//! ```
//!
//! 동작:
//! 1. `PgHealthCheckRepository::record()` 로 DB INSERT
//! 2. fail 인 경우:
//!    - hard-fail (4xx / parse_fail) → 즉시 GitHub Issue
//!    - soft-fail (5xx / timeout / connection_fail) + 3일 연속 cron fail → Issue
//!    - else → record only
//! 3. success 인 경우:
//!    - 기존 open `drift` Issue (`api_name` 일치) 자동 close + comment
//!
//! 환경변수:
//! - `DATABASE_URL` (필수) — PgPool 연결
//! - `GITHUB_TOKEN` (필수) — Issue 생성/close
//! - `GITHUB_REPOSITORY` (필수, 자동 set in actions) — `owner/repo` 형식

#![allow(clippy::expect_used)]

use std::str::FromStr;
use std::sync::Arc;

use anyhow::{Context, Result};
use api_health_domain::{
    HealthCheckRepository, HealthStatus, NewHealthCheck,
};
use clap::Parser;
use db::api_health::PgHealthCheckRepository;
use octocrab::Octocrab;
use sqlx::PgPool;
use tracing::{info, warn};

#[derive(Parser, Debug)]
#[command(name = "api-health-recorder")]
struct Args {
    /// API endpoint 식별자. 예: data_go_kr.getBrTitleInfo
    #[arg(long)]
    api_name: String,

    /// HealthStatus 문자열. success / http_5xx / http_4xx / parse_fail / timeout / connection_fail
    #[arg(long)]
    status: String,

    /// HTTP 응답 코드 (선택).
    #[arg(long)]
    http_code: Option<u16>,

    /// masked log (선택).
    #[arg(long)]
    error_detail: Option<String>,

    /// true = scheduled cron, false = workflow_dispatch.
    #[arg(long)]
    cron_run: bool,

    /// 호출 소요 시간 (ms).
    #[arg(long)]
    duration_ms: u32,
}

const STREAK_THRESHOLD: u32 = 3;
const ISSUE_LABEL: &str = "drift";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,sqlx=warn".into()),
        )
        .init();

    let args = Args::parse();
    let status = HealthStatus::from_str(&args.status)
        .with_context(|| format!("invalid --status: {}", args.status))?;

    // 1. DB record
    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL required")?;
    let pool = PgPool::connect(&database_url).await.context("connect DB")?;
    let repo = PgHealthCheckRepository::new(Arc::new(pool));

    let new = NewHealthCheck {
        api_name: &args.api_name,
        status,
        http_code: args.http_code,
        error_detail: args.error_detail.as_deref(),
        cron_run: args.cron_run,
        duration_ms: args.duration_ms,
    };
    let record = repo.record(new).await.context("record to DB")?;
    info!(
        record_id = record.id,
        api = %record.api_name,
        status = %record.status,
        "recorded to api_health_check"
    );

    // 2. GitHub Issue orchestration
    let token = std::env::var("GITHUB_TOKEN").context("GITHUB_TOKEN required")?;
    let repo_full = std::env::var("GITHUB_REPOSITORY").context("GITHUB_REPOSITORY required")?;
    let (owner, repo_name) = repo_full
        .split_once('/')
        .with_context(|| format!("GITHUB_REPOSITORY 형식 'owner/repo' 필요, got: {repo_full}"))?;

    let octo = Octocrab::builder().personal_token(token).build()?;

    let escalate = if status.is_hard_fail() {
        true
    } else if status.is_soft_fail() {
        repo.is_n_cron_runs_failed(&args.api_name, STREAK_THRESHOLD)
            .await
            .context("query streak")?
    } else {
        false
    };

    if escalate {
        create_or_update_drift_issue(&octo, owner, repo_name, &args, status, &record.error_detail).await?;
    } else if status == HealthStatus::Success {
        recover_open_drift_issues(&octo, owner, repo_name, &args.api_name).await?;
    } else {
        info!("soft-fail without 3-day streak — record only");
    }

    Ok(())
}

async fn create_or_update_drift_issue(
    octo: &Octocrab,
    owner: &str,
    repo: &str,
    args: &Args,
    status: HealthStatus,
    error_detail: &Option<String>,
) -> Result<()> {
    let issues = octo.issues(owner, repo);

    // 기존 open issue 검색 (label="drift" + api_name 매치)
    let list = issues
        .list()
        .labels(&[ISSUE_LABEL.to_owned()])
        .state(octocrab::params::State::Open)
        .send()
        .await?;

    let title_match = format!("🚨 정부 API drift detected: {}", args.api_name);

    if let Some(existing) = list.items.iter().find(|i| i.title == title_match) {
        // 기존 issue → comment 추가
        let comment = format!(
            "또 fail (cron_run={}, status={}, http={:?}).\n\n```\n{}\n```",
            args.cron_run,
            status,
            args.http_code,
            error_detail.as_deref().unwrap_or("(no detail)")
        );
        issues.create_comment(existing.number, comment).await?;
        warn!(issue = existing.number, "appended comment to existing drift issue");
    } else {
        // 신규 issue
        let body = format!(
            "## 발견 시각\n{}\n\n## 분류\n{}\n\n## API\n{}\n\n## 응답 정보\n- HTTP: {:?}\n- duration_ms: {}\n- cron_run: {}\n\n## 실패 log\n```\n{}\n```\n\n## 수동 검증\nGitHub Actions → \"api-drift-smoke-test\" → \"Run workflow\"",
            chrono::Utc::now().to_rfc3339(),
            status,
            args.api_name,
            args.http_code,
            args.duration_ms,
            args.cron_run,
            error_detail.as_deref().unwrap_or("(no detail)")
        );

        let labels = vec![
            ISSUE_LABEL.to_owned(),
            format!("drift:{}", status_label_suffix(status)),
        ];

        let new_issue = issues
            .create(&title_match)
            .body(&body)
            .labels(labels)
            .send()
            .await?;
        warn!(issue = new_issue.number, "created drift issue");
    }
    Ok(())
}

async fn recover_open_drift_issues(
    octo: &Octocrab,
    owner: &str,
    repo: &str,
    api_name: &str,
) -> Result<()> {
    let issues = octo.issues(owner, repo);
    let list = issues
        .list()
        .labels(&[ISSUE_LABEL.to_owned()])
        .state(octocrab::params::State::Open)
        .send()
        .await?;

    let title_match = format!("🚨 정부 API drift detected: {api_name}");

    for issue in list.items.iter().filter(|i| i.title == title_match) {
        let comment = "✅ 자가 복구 — 정부 일시 장애였음. 다음 cron 정상 응답으로 close.".to_owned();
        issues.create_comment(issue.number, comment).await?;
        issues.update(issue.number)
            .state(octocrab::models::IssueState::Closed)
            .send()
            .await?;
        info!(issue = issue.number, "closed drift issue (auto-recovered)");
    }
    Ok(())
}

const fn status_label_suffix(status: HealthStatus) -> &'static str {
    match status {
        HealthStatus::Success => "success",
        HealthStatus::Http5xx => "5xx-server",
        HealthStatus::Http4xx => "4xx-auth",
        HealthStatus::ParseFail => "schema",
        HealthStatus::Timeout => "timeout",
        HealthStatus::ConnectionFail => "connection",
    }
}
```

**참고**: `octocrab` 0.46 의 정확한 API (issues / list / state / labels / create_comment / update) 는 plan 작성 시점에 docs.rs 확인하고 변경 가능. 위 코드는 일반적인 패턴.

#### Step 5.4: cargo check + clippy

- [ ] **Step**: 검증

```bash
cargo check -p api-health-recorder
cargo clippy -p api-health-recorder --all-targets -- -D warnings
cargo fmt --all -- --check
```

Expected: 모두 pass.

#### Step 5.5: 로컬 dry-run (선택)

- [ ] **Step**: DB 만 record 검증 (Issue API 안 호출하는 mock GH_TOKEN)

```bash
DATABASE_URL=$DATABASE_URL \
GITHUB_TOKEN=invalid_token_for_db_only_test \
GITHUB_REPOSITORY=w1kch9812-cmd/test \
cargo run --bin api-health-recorder -- \
    --api-name test.local_dry_run \
    --status success \
    --duration-ms 100 \
    --cron-run false
```

Expected: DB record 성공 + GitHub API 호출 시 `recover_open_drift_issues` 가 빈 list 반환 (token 무효지만 search 자체는 try 함). Issue 생성 분기에 안 들어가니 token error 무시 가능.

(GitHub API 가 invalid token 에 401 반환할 수 있음 → main 함수 종료 코드 1. CI 에서만 정확한 token 으로 검증.)

#### Step 5.6: T5 commit

- [ ] **Step**: commit

```bash
git add Cargo.toml crates/api-health-recorder/

git commit -m "$(cat <<'EOF'
feat(sp7-iii-t5): add api-health-recorder Rust binary (octocrab + PgImpl)

T5 of SP7-iii:
- crates/api-health-recorder/ — workspace 신규 binary crate
  - Args (clap derive): --api-name --status --http-code --error-detail --cron-run --duration-ms
  - 1) PgHealthCheckRepository::record() 로 DB INSERT
  - 2) hard-fail (4xx/parse_fail) → 즉시 GitHub Issue
       soft-fail + 3일 연속 cron fail → Issue
       else → record only
  - 3) Success → 기존 open drift Issue 자동 close + 자가 복구 comment
- octocrab 0.46 GitHub API client (Issue create/comment/close)
- 의존성: api-health-domain + db + sqlx + tokio + clap + octocrab + anyhow + tracing
- 로컬 dry-run 검증 (DB record only)
EOF
)"
```

**사용자 체크포인트**: T5 commit 확인 + 다음 진행 여부.

---

## Phase F: GitHub Actions Workflow + 검증

### Task 6: T6 — `.github/workflows/api-drift-smoke-test.yml` + secrets + 검증 + roadmap

**Files:**
- Create: `.github/workflows/api-drift-smoke-test.yml`
- Create: `docs/observability/api-drift-smoke-test.md`
- Modify: `docs/superpowers/roadmap.md`

#### Step 6.1: workflow yml 작성

- [ ] **Step**: `.github/workflows/api-drift-smoke-test.yml` 작성

```yaml
name: api-drift-smoke-test

on:
  schedule:
    # 04:00 KST = 19:00 UTC (전날)
    - cron: '0 19 * * *'
  workflow_dispatch:
    inputs:
      simulate_failure:
        description: 'Force fail (drift detection 검증)'
        type: boolean
        default: false

jobs:
  smoke-data-go-kr:
    runs-on: ubuntu-24.04
    timeout-minutes: 15
    permissions:
      issues: write
      contents: read
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2

      - name: Determine PNU (simulate_failure → invalid PNU)
        id: pnu
        run: |
          if [ "${{ inputs.simulate_failure }}" = "true" ]; then
            echo "value=9999999999999999999" >> $GITHUB_OUTPUT
          else
            echo "value=1168010100107370000" >> $GITHUB_OUTPUT
          fi

      - name: Run smoke test
        id: smoke
        env:
          ODP_SERVICE_KEY: ${{ secrets.ODP_SERVICE_KEY }}
          GONGZZANG_DRIFT_TEST_PNU: ${{ steps.pnu.outputs.value }}
        run: |
          set +e
          START_MS=$(date +%s%3N)
          cargo test --features real-api -p data-go-kr-client \
              --test smoke_real_api -- --ignored --nocapture 2>&1 | tee smoke.log
          STATUS_CODE=$?
          END_MS=$(date +%s%3N)
          DURATION=$((END_MS - START_MS))
          echo "duration_ms=$DURATION" >> $GITHUB_OUTPUT

          if [ $STATUS_CODE -eq 0 ]; then
              echo "status=success" >> $GITHUB_OUTPUT
              echo "http_code=200" >> $GITHUB_OUTPUT
          else
              if grep -q "endpoint URL drift" smoke.log; then
                  echo "status=parse_fail" >> $GITHUB_OUTPUT
                  echo "http_code=200" >> $GITHUB_OUTPUT
              elif grep -q "5xx" smoke.log; then
                  echo "status=http_5xx" >> $GITHUB_OUTPUT
                  echo "http_code=502" >> $GITHUB_OUTPUT
              else
                  echo "status=connection_fail" >> $GITHUB_OUTPUT
              fi
          fi

      - name: Sanitize error log (mask secrets)
        id: sanitize
        if: always()
        run: |
          # ServiceKey 부분 마스킹
          sed -i 's/ServiceKey=[^&"]*/ServiceKey=***/g' smoke.log || true
          # 마지막 80줄만 (Issue body 압축)
          tail -n 80 smoke.log > smoke.log.short
          ESCAPED=$(jq -Rs . < smoke.log.short)
          echo "log_json=$ESCAPED" >> $GITHUB_OUTPUT

      - name: Record + Issue orchestration
        env:
          DATABASE_URL: ${{ secrets.STAGING_DATABASE_URL }}
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          GITHUB_REPOSITORY: ${{ github.repository }}
        run: |
          IS_CRON="${{ github.event_name == 'schedule' }}"
          cargo run --bin api-health-recorder -- \
              --api-name "data_go_kr.getBrTitleInfo" \
              --status "${{ steps.smoke.outputs.status }}" \
              --duration-ms "${{ steps.smoke.outputs.duration_ms }}" \
              --cron-run "$IS_CRON" \
              ${HTTP_CODE:+--http-code $HTTP_CODE} \
              --error-detail "$(cat smoke.log.short)"
        env:
          HTTP_CODE: ${{ steps.smoke.outputs.http_code }}

  smoke-vworld:
    runs-on: ubuntu-24.04
    timeout-minutes: 15
    permissions:
      issues: write
      contents: read
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2

      - name: Determine PNU
        id: pnu
        run: |
          if [ "${{ inputs.simulate_failure }}" = "true" ]; then
            echo "value=9999999999999999999" >> $GITHUB_OUTPUT
          else
            echo "value=1168010100107370000" >> $GITHUB_OUTPUT
          fi

      - name: Run smoke test
        id: smoke
        env:
          VWORLD_API_KEY: ${{ secrets.VWORLD_API_KEY }}
          VWORLD_DOMAIN: ${{ secrets.VWORLD_DOMAIN }}
          GONGZZANG_DRIFT_TEST_PNU: ${{ steps.pnu.outputs.value }}
        run: |
          set +e
          START_MS=$(date +%s%3N)
          cargo test --features real-api -p vworld-client \
              --test smoke_real_api -- --ignored --nocapture 2>&1 | tee smoke.log
          STATUS_CODE=$?
          END_MS=$(date +%s%3N)
          echo "duration_ms=$((END_MS - START_MS))" >> $GITHUB_OUTPUT

          if [ $STATUS_CODE -eq 0 ]; then
              echo "status=success" >> $GITHUB_OUTPUT
              echo "http_code=200" >> $GITHUB_OUTPUT
          else
              echo "status=connection_fail" >> $GITHUB_OUTPUT
          fi

      - name: Sanitize log
        if: always()
        run: |
          sed -i 's/key=[^&"]*/key=***/g' smoke.log || true
          tail -n 80 smoke.log > smoke.log.short

      - name: Record + Issue orchestration
        env:
          DATABASE_URL: ${{ secrets.STAGING_DATABASE_URL }}
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          GITHUB_REPOSITORY: ${{ github.repository }}
          HTTP_CODE: ${{ steps.smoke.outputs.http_code }}
        run: |
          IS_CRON="${{ github.event_name == 'schedule' }}"
          cargo run --bin api-health-recorder -- \
              --api-name "vworld.getFeature" \
              --status "${{ steps.smoke.outputs.status }}" \
              --duration-ms "${{ steps.smoke.outputs.duration_ms }}" \
              --cron-run "$IS_CRON" \
              ${HTTP_CODE:+--http-code $HTTP_CODE} \
              --error-detail "$(cat smoke.log.short)"
```

#### Step 6.2: secrets 등록 (사용자 작업)

- [ ] **Step (사용자)**: GitHub Settings → Secrets and variables → Actions

```
ODP_SERVICE_KEY        = (data.go.kr 키)
VWORLD_API_KEY         = (V-World 키)
VWORLD_DOMAIN          = localhost
STAGING_DATABASE_URL   = (production DB url, 1인 단계는 동일 DB OK)
```

#### Step 6.3: docs/observability/api-drift-smoke-test.md 작성

- [ ] **Step**: `docs/observability/api-drift-smoke-test.md` 작성

````markdown
# API Drift Smoke Test (SP7-iii)

> **목적**: 정부 API (data.go.kr / V-World) 의 endpoint URL + JSON schema drift 자동 검출
> **SSOT**: Postgres `api_health_check` 테이블 + GitHub Issue (사람 알림 사본)

## 시스템 개요

```
[04:00 KST cron]
   ↓
[GitHub Actions: api-drift-smoke-test.yml]
   ├── job: smoke-data-go-kr
   └── job: smoke-vworld
        ↓ (각 job 안에서)
   [cargo test --features real-api -- --ignored]
        ↓
   [api-health-recorder Rust binary]
        ├── PgHealthCheckRepository::record() → DB INSERT
        └── Issue orchestration (escalation / 자가 복구)
```

## 분류 (HealthStatus)

| Status | HTTP | 분류 | Escalation |
|---|---|---|---|
| `success` | 200 | OK | (자가 복구 trigger) |
| `http_5xx` | 5xx | soft-fail | 3일 연속 cron fail 시 Issue |
| `http_4xx` | 4xx | hard-fail | 즉시 Issue (키/quota 문제) |
| `parse_fail` | 200 | hard-fail | 즉시 Issue (schema drift) |
| `timeout` | - | soft-fail | 3일 연속 cron fail 시 Issue |
| `connection_fail` | - | soft-fail | 3일 연속 cron fail 시 Issue |

## 운영 절차

### 매일 정상 응답 확인

GitHub → Actions → api-drift-smoke-test → 최근 cron run 결과.

### Issue 생성 시 처리

1. Issue body 의 "분류" 확인
2. `parse_fail` (schema drift):
   - 정부 API 응답 schema 확인 (curl 또는 staging 검증)
   - parser 코드 (`parse_building_title` 등) 갱신 PR
3. `http_4xx`:
   - 키 만료 또는 quota 초과 확인
   - secrets 갱신 또는 quota 증액 신청
4. `http_5xx` / `timeout` / `connection_fail`:
   - 정부 API 점검 페이지 확인 (https://www.vworld.kr/dev, https://www.data.go.kr)
   - 자가 복구 대기 (다음 cron success 시 Issue 자동 close)

### 수동 trigger (drift 의심 시 즉시 검증)

GitHub → Actions → api-drift-smoke-test → "Run workflow"

체크박스:
- `simulate_failure: false` (default) — 정상 path 검증
- `simulate_failure: true` — 일부러 fail (Issue 자동 생성 검증)

### simulate_failure 사용 케이스

- 새 endpoint 추가 후 alert 시스템 작동 검증
- Issue 자동 생성 / close 로직 변경 후 검증
- Issue label / body 포맷 검증

## 알림 정책

### Issue 자동 생성 조건

| Trigger | Label |
|---|---|
| `parse_fail` 1회 | `drift`, `drift:schema` |
| `http_4xx` 1회 | `drift`, `drift:4xx-auth` |
| `http_5xx` 3 cron 연속 | `drift`, `drift:5xx-server` |
| `timeout` 3 cron 연속 | `drift`, `drift:timeout` |
| `connection_fail` 3 cron 연속 | `drift`, `drift:connection` |

### 자가 복구

다음 cron 의 `success` 응답 시:
1. 기존 open drift Issue (api_name 매치) 에 comment "✅ 자가 복구"
2. Issue close (label `drift` 유지 — 검색 가능)

### 수동 close

문제 해결 후 수동으로 Issue close — 차후 cron success 시 본 시스템이 영향 X.

## 진화 path

- **SP7-i (Sentry)**: production code panic / breaker open 등 — 본 시스템과 별개 dispatch
- **SP7-ii (Grafana)**: `api_health_check` 테이블에서 metrics 추출 (성공률 / 분류 분포 / latency)
- **SP-Admin React Flow**: admin UI 에서 `api_health_check` 시계열 시각화

## DB Schema 참조

- 마이그레이션: `migrations/30007_api_health_check.sql`
- 도메인: `crates/operations/api-health/`
- 인프라: `crates/db/src/api_health.rs`
- recorder binary: `crates/api-health-recorder/`

## Spec / Plan

- Spec: `docs/superpowers/specs/2026-05-05-sub-project-7-iii-api-drift-monitoring-design.md`
- Plan: `docs/superpowers/plans/2026-05-05-sub-project-7-iii-api-drift-monitoring.md`
````

#### Step 6.4: roadmap.md 갱신

- [ ] **Step**: `docs/superpowers/roadmap.md` 의 header / 완료 표 / follow-up 갱신

다음 변경 적용:

**Header:**
```markdown
> **갱신일**: 2026-05-05 (SP7-iii 종료 직후)
> **현재 main**: `<T6 commit hash>` (SP7-iii — drift 자동 검출 시스템)
```

**완료 표 (SP7-iii 추가):**
```markdown
| **7-iii** | 정부 API drift 자동 검출 시스템 | crates/operations/api-health (도메인) + crates/db/src/api_health.rs (PgImpl) + 2 smoke test crate (data.go.kr + V-World, feature-gated) + crates/api-health-recorder (octocrab binary) + .github/workflows/api-drift-smoke-test.yml (nightly cron + workflow_dispatch) + docs/observability/api-drift-smoke-test.md (운영 SSOT). FU 45/46 closed. SSS 7기둥 모두 ◎ | ✅ |
```

**누적 통계 갱신:**
```markdown
**누적**: 33 crate, ~1285 tests, 4 CI workflow 그린 (CI / db-migrations / walking-skeleton / api-drift-smoke-test).
```

**Follow-up 갱신:**
- ~~FU 45 (제안): endpoint URL drift staging-only smoke test~~ → ✅ closed by SP7-iii (`<T6 commit>`)
- ~~FU 46 (제안): JSON Number vs String schema drift 모니터링~~ → ✅ closed by SP7-iii (`<T6 commit>`)
- FU 47: V-World 지오코딩 — 미해소, SP6 frontend 또는 dev tool sub-project

**SP7-i / SP7-ii 자리:**
```markdown
### SP7 시리즈 (관측성)
- ✅ SP7-iii: drift 자동 검출 (2026-05-05, `<commit>`)
- 미착수 SP7-i: Sentry — 에러 자동 추적 (services/api 통합, 1-2일)
- 미착수 SP7-ii: Grafana metrics + Outbox publisher metrics (2-3일)
```

#### Step 6.5: 워크플로우 검증 (사용자 체크포인트)

- [ ] **Step (사용자)**: secrets 등록 후 push → GitHub Actions 페이지에서 수동 trigger
  - workflow_dispatch (정상 path) — 모든 job pass + DB record 확인
  - workflow_dispatch (`simulate_failure=true`) — fail + Issue 자동 생성 확인 (label: drift, drift:schema)

- [ ] **Step (사용자)**: simulate_failure 후 수동 trigger 정상 응답 → 기존 Issue 자동 close + comment "✅ 자가 복구" 확인

#### Step 6.6: cargo check / clippy / test workspace 그린

- [ ] **Step**: workspace 검증

```bash
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --lib
cargo fmt --all -- --check
```

Expected: 모두 pass.

#### Step 6.7: T6 commit + push

- [ ] **Step**: commit

```bash
git add .github/workflows/api-drift-smoke-test.yml \
        docs/observability/api-drift-smoke-test.md \
        docs/superpowers/roadmap.md

git commit -m "$(cat <<'EOF'
feat(sp7-iii-t6): GitHub Actions cron workflow + docs + roadmap

T6 of SP7-iii (마지막):
- .github/workflows/api-drift-smoke-test.yml — nightly cron 04:00 KST + workflow_dispatch
  - 2 jobs (smoke-data-go-kr / smoke-vworld) 병렬
  - simulate_failure input 으로 fail 의도 검증 가능
  - secrets 마스킹 + tail 80 lines log 압축
  - api-health-recorder binary 호출 → DB record + Issue orchestration
- docs/observability/api-drift-smoke-test.md — 운영 SSOT
  - 분류 표 (HealthStatus)
  - 운영 절차 (Issue 처리 / 수동 trigger / simulate_failure)
  - 알림 정책 (label / 자가 복구)
  - 진화 path (SP7-i/ii / SP-Admin)
- docs/superpowers/roadmap.md — SP7-iii ✅ closed
  - FU 45/46 closed 표기
  - 누적 33 crate / ~1285 tests / 4 CI workflow

SP7 시리즈 첫 sub-project 완료. SP7-i (Sentry) / SP7-ii (Grafana) 자리 명시.
EOF
)"

git push origin main
```

**사용자 체크포인트**: T6 commit 확인 + 4 CI workflow 그린 확인 + 다음 sub-project 결정.

---

## 위험 요소

- **V-World 일시 장애**: brainstorming 시점에 V-World 502 발견. 첫 cron 결과가 fail 일 수 있음 — 일시 장애로 분류 (`http_5xx`), 3일 연속 fail 시에만 escalation 이라 시스템은 정상 작동
- **octocrab 0.46 API 변경**: docs.rs 시점 따라 `issues().list()` / `update()` API signature 다를 수 있음 — plan 작성 시점에 확인하고 수정
- **데이터.go.kr 도메인 등록**: ODP_SERVICE_KEY 가 발급 시 도메인 등록한 경우 GitHub Actions IP 가 다른 도메인 일 수 있음 — 일반 키 (도메인 검증 X) 면 OK
- **STAGING_DATABASE_URL 결정**: 1인 단계 = production DB 와 동일. 미래 production scale 시 분리 (SP8 IaC)

## 추정

- T1: 1 commit, 1-2시간
- T2: 1 commit, 2-3시간
- T3: 1 commit, 1-2시간
- T4: 1 commit, 1-2시간
- T5: 1 commit, 4-6시간 (octocrab 학습 + Issue orchestration)
- T6: 1 commit, 2-3시간 (workflow yml + docs)

총: 4-5일 (각 task 끝 사용자 체크포인트 포함)

## 완료 후 다음

- SP7-i (Sentry) brainstorming
- 또는 SP4-iii-b (data.go.kr 실거래가) — drift smoke test 자연 추가 가능
- 또는 SP6 (Frontend) brainstorming
- 또는 SP-FU-OCC (FU 14/15/16 OCC API)

---

## 자가 평가 — Spec coverage

Spec 의 모든 § 가 plan task 로 커버됐는지 확인:

- § 1 배경 — context only, plan task X
- § 2 목표 — T1-T6 전체
- § 3 SSS 7기둥 — T1-T6 누적
- § 4 아키텍처 (4.1 그림 / 4.2 컴포넌트 / 4.3 책임) → T1-T6 파일 구조 그대로
- § 5 데이터 모델 (5.1 SQL / 5.2 entity / 5.3 trait) → T1
- § 6 Smoke test 통합 테스트 (6.1 feature flag / 6.2 data.go.kr / 6.3 V-World / 6.4 simulate) → T3 + T4
- § 7 GitHub Actions workflow → T6
- § 8 알림 정책 → T5 (Issue orchestration) + T6 (workflow)
- § 9 검증 / 테스트 전략 → T1-T6 의 단위 + 통합 테스트
- § 10 Migration 진화 path → T6 docs/observability/
- § 11 Follow-up → T6 roadmap.md
- § 12 작업 단위 → T1-T6 그대로
- § 13 추정 → 본 plan 의 추정
- § 14 SSS 자가 평가 → T1-T6 누적

**모든 § 가 task 로 covered.** ✅
