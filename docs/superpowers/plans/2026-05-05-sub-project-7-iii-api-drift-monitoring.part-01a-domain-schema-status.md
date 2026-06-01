# SP7-iii API Drift Monitoring - Part 01A: Domain Schema and Status

Parent index: [SP7-iii API Drift Monitoring - Part 01](./2026-05-05-sub-project-7-iii-api-drift-monitoring.part-01.md).
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
