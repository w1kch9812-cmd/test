# SP7-iii API Drift Monitoring - Part 01B: Entity, Repository, and Commit

Parent index: [SP7-iii API Drift Monitoring - Part 01](./2026-05-05-sub-project-7-iii-api-drift-monitoring.part-01.md).
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
