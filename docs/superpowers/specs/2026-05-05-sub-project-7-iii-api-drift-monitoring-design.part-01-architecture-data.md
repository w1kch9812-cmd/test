# Sub-project 7-iii: 정부 API endpoint URL + JSON schema drift 자동 검출 시스템

> **작성일**: 2026-05-05
> **이전 sub-project**: SP-FU-i (T1-T5 closed, `bae883c`/`d762437`)
> **SP7 통합 architecture 의 첫 sub-project**
> **상태**: 디자인 — implementation plan 작성 대기

---

## 1. 배경 및 문제

### 1.1 직전 sub-project 에서 발견한 silent drift 2건

SP-FU-i T4 검증 중 사용자 의심 ("실제로 API 호출해봤어?") + 실 호출로 발견:

**Bug 1: endpoint URL deprecated**
- 코드의 `BR_TITLE_PATH = "/1613000/BldRgstService_v2/getBrTitleInfo"` 가 deprecated
- HTTP 200 + body `"Unexpected errors"` 반환 — JSON parse 실패만 발생
- production 호출 0건 보장 — wiremock 만 통과시킨 silent endpoint drift
- 수정: `BldRgstHubService` 로 fix (`bae883c`)

**Bug 2: JSON Number vs String schema mismatch**
- 실 응답: `totArea` / `heit` / `grndFlrCnt` 가 JSON Number
- wiremock fixture: 모든 숫자 필드가 string
- 우리 parser: `as_str` 만 처리 → 모든 응답 `Malformed`
- 수정: `read_f64_field` helper 도입 (`bae883c`)

### 1.2 근본 원인 — wiremock 단방향 검증

기존 통합 테스트:
```
[wiremock fixture] → [우리 client] → [우리 parser] → [도메인 enum]
```

이 chain 은 **fixture 의 정확성** 을 가정. 실 API 와 fixture 가 다르면 silent drift 가 wiremock 을 통과.

### 1.3 production 진입 전 catastrophic 위험

lazy fetch architecture (현재 우리 패턴):
- 사용자 요청 → 우리 DB 캐시 확인 → 없으면 정부 API 호출 → 응답 저장
- silent drift 발생 시: 사용자에게 빈 응답 또는 잘못된 응답
- production 0 사용자 → catastrophic (서비스 신뢰 망가짐)

---

## 2. 목표

### 2.1 핵심 목표

**정부 API (data.go.kr / V-World) 의 endpoint URL 변경 + JSON schema 변경을 자동 검출.**

검출 분류:
1. **endpoint URL drift** — HTTP 4xx/5xx/timeout/non-JSON body
2. **schema drift** — HTTP 200 + JSON 응답이지만 우리 parser fail
3. **value drift** — HTTP 200 + parser 통과지만 핵심 필드 missing/empty

### 2.2 비목표 (다른 sub-project)

- 정부 API 의 실제 데이터 정확성 검증 (e.g. 강남파이낸스 의 면적이 정확한가) — domain validation, 별도
- 우리 production code 의 logic bug 검증 — SP7-i (Sentry) 영역
- 외부 호출 latency / throughput 시각화 — SP7-ii (Grafana) 영역
- V-World 지오코딩 (FU 47) — SP6 frontend 또는 별도 dev tool

### 2.3 SP7 통합 architecture 안의 위치

```
SP7 통합 architecture (3 sub-project)
   ├── SP7-iii (본 sub-project, 첫 번째)
   │   └── drift 검출 — 우리 Postgres SSOT
   ├── SP7-i (미래)
   │   └── 에러 자동 추적 — Sentry SaaS (1인 단계) / self-host (production scale)
   └── SP7-ii (미래)
       └── metrics — Prometheus + Grafana Cloud (1인 단계) / self-host (scale)
```

**왜 SP7-iii 가 첫 번째:**
1. 직접 발견한 catastrophic risk (silent drift)
2. 우리 DB 통합 = SSS SSOT 패턴 도입 → SP7-i/ii 가 그 패턴 활용
3. 외부 SaaS 의존 0 (Sentry/Grafana 도입 의사결정 미루기 가능)

---

## 3. SSS 7기둥 매칭

| 기둥 | 보장 방법 |
|---|---|
| **1 일관성** | smoke test 가 production code path 그대로 사용 (BuildingRegisterClient → parse_building_title). wiremock 통합 테스트와 동일 패턴 |
| **2 자동 강제** | nightly cron 자동 trigger + 3일 연속 fail 시 GitHub Issue 자동 생성. 사람 의존 0 |
| **3 추적성** | `api_health_check` 테이블 영구 보존 — 모든 cron run / 수동 trigger 결과 record. 미래 SP-Admin 시각화 통합 |
| **4 안전성** | feature flag `real-api` = 평소 PR/CI 에서 외부 호출 0. 통합 테스트 자체가 production code path 검증 |
| **5 가시성** | Issue 자동 alert + DB 쿼리로 history 추적 + (미래) admin UI React Flow 시각화 |
| **6 SSOT** | drift 결과 = 우리 Postgres `api_health_check` (외부 GitHub Issue 는 사람 알림용 사본) |
| **7 명확성** | `docs/observability/api-drift-smoke-test.md` 영구 문서 + workflow_dispatch input `simulate_failure` 로 검증 절차 영구 인프라 |

---

## 4. 아키텍처

### 4.1 큰 그림

```
[04:00 KST cron — workflow_dispatch 도 가능 (수동 trigger)]
   ↓
[GitHub Actions: api-drift-smoke-test.yml]
   ├── secrets 로딩: ODP_SERVICE_KEY / VWORLD_API_KEY / VWORLD_DOMAIN
   │                  + STAGING_DATABASE_URL (DB write 용)
   ↓
[2 jobs 병렬 실행]
   ├── job 1: cargo test --features real-api -p data-go-kr-client -- --ignored
   │           └── DataGoKrClient::from_env() → fetch_title_info(강남파이낸스 PNU)
   │                → parse_building_title → assert(Office + SteelReinforcedConcrete)
   └── job 2: cargo test --features real-api -p vworld-client -- --ignored
               └── VWorldClient::from_env() → ParcelReader::fetch_by_pnu(강남 PNU)
                    → assert(필지 존재 + jiyok_cd 매핑)
   ↓
[결과 분류 (Rust 코드 안)]
   ├── success → status="success"
   ├── HTTP 5xx → status="http_5xx" (soft-fail)
   ├── HTTP 4xx → status="http_4xx" (hard-fail, 키 문제)
   ├── HTTP 200 + parse fail → status="parse_fail" (hard-fail, schema drift)
   └── timeout / connection → status="timeout" (soft-fail)
   ↓
[DB write — 모든 결과 (success 포함) record]
   INSERT INTO api_health_check (api_name, status, http_code, error_detail, cron_run, duration_ms)
   ↓
[fail 시 추가 처리]
   ├── 3일 연속 fail (DB 쿼리: find_consecutive_failures(api_name, n=3))
   │   ├── true → 신규 Issue 생성 (또는 기존 Issue 에 streak label 추가)
   │   └── false → 단순 fail record 만 (alert X)
   └── status="parse_fail" → 즉시 Issue (1일도 escalation)
   ↓
[복구 시 자동 close]
   기존 open Issue + 오늘 success → comment 추가 ("자가 복구") + close
```

### 4.2 컴포넌트

```
crates/db/migration/
└── 30007_api_health_check.sql            (NEW — DB schema)

crates/operations/api-health/
├── Cargo.toml                            (NEW crate)
└── src/
    ├── lib.rs
    ├── entity.rs                         (HealthCheckRecord)
    ├── repository.rs                     (HealthCheckRepository trait)
    └── status.rs                         (HealthStatus enum)

crates/db/src/
└── api_health.rs                         (NEW — PgHealthCheckRepository)

crates/data-clients/data-go-kr/
├── Cargo.toml                            (real-api feature 추가)
└── tests/
    └── smoke_real_api.rs                 (NEW — feature-gated 통합 테스트)

crates/data-clients/vworld/
├── Cargo.toml                            (real-api feature 추가)
└── tests/
    └── smoke_real_api.rs                 (NEW)

crates/api-health-recorder/
├── Cargo.toml                            (NEW binary crate)
└── src/
    └── main.rs                           (DB record + Issue 자동 생성/close)

.github/workflows/
└── api-drift-smoke-test.yml              (NEW)

docs/observability/
└── api-drift-smoke-test.md               (NEW — 운영 절차 SSOT)
```

### 4.3 책임 분리

- **`crates/operations/api-health` (도메인)** — `HealthCheckRecord` + `HealthCheckRepository` trait + `HealthStatus` enum. 외부 의존 0
- **`crates/db/api_health.rs` (인프라)** — `PgHealthCheckRepository` 구현. SQL 만
- **smoke test 통합 테스트** — production client 그대로 호출 + 결과 분류
- **workflow yml** — orchestration (cron + DB write + Issue 생성). business logic 0
- **docs** — 운영 절차 SSOT (수동 trigger 방법, 분류 표, escalation 정책)

---

## 5. 데이터 모델

### 5.1 DB Schema (`30007_api_health_check.sql`)

```sql
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
    -- true = scheduled cron, false = workflow_dispatch (수동)
    duration_ms INT NOT NULL CHECK (duration_ms >= 0)
);

CREATE INDEX idx_api_health_check_api_name_checked_at
    ON api_health_check (api_name, checked_at DESC);

CREATE INDEX idx_api_health_check_failures
    ON api_health_check (api_name, checked_at DESC)
    WHERE status != 'success';

COMMENT ON TABLE api_health_check IS
    '정부 API drift 검출 — SP7-iii. 모든 cron run / 수동 trigger 결과 record. SSS SSOT.';
```

### 5.2 도메인 entity

```rust
// crates/operations/api-health/src/entity.rs
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

// crates/operations/api-health/src/status.rs
pub enum HealthStatus {
    Success,
    Http5xx,         // soft-fail (정부 일시 장애 가능)
    Http4xx,         // hard-fail (키 / quota / endpoint 죽음)
    ParseFail,       // hard-fail (schema drift)
    Timeout,         // soft-fail
    ConnectionFail,  // soft-fail
}

impl HealthStatus {
    pub const fn is_hard_fail(self) -> bool {
        matches!(self, Self::Http4xx | Self::ParseFail)
    }
}
```

### 5.3 Repository trait

```rust
// crates/operations/api-health/src/repository.rs
#[async_trait]
pub trait HealthCheckRepository: Send + Sync {
    async fn record(&self, record: NewHealthCheck<'_>) -> Result<HealthCheckRecord, RepoError>;

    /// 최근 N개 cron run 모두 fail 인가? (수동 trigger 무관)
    async fn is_n_cron_runs_failed(
        &self,
        api_name: &str,
        n: u32,
    ) -> Result<bool, RepoError>;

    /// 가장 최근 record (success / fail 무관)
    async fn find_latest(&self, api_name: &str) -> Result<Option<HealthCheckRecord>, RepoError>;
}

pub struct NewHealthCheck<'a> {
    pub api_name: &'a str,
    pub status: HealthStatus,
    pub http_code: Option<u16>,
    pub error_detail: Option<&'a str>,
    pub cron_run: bool,
    pub duration_ms: u32,
}
```

---
