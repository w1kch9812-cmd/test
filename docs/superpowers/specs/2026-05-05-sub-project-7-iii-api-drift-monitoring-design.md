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

## 6. Smoke Test 통합 테스트

### 6.1 Feature flag 패턴

```toml
# crates/data-clients/data-go-kr/Cargo.toml
[features]
real-api = []
```

### 6.2 data.go.kr smoke test

```rust
// crates/data-clients/data-go-kr/tests/smoke_real_api.rs
#![cfg(feature = "real-api")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[tokio::test]
#[ignore]  // 평소 cargo test 에서 skip — feature + ignored 둘 다 필수
async fn smoke_data_go_kr_building_register_alive() {
    // 환경변수 ODP_SERVICE_KEY 필수 (CI secret)
    let client = DataGoKrClient::from_env().expect("ODP_SERVICE_KEY required");

    // 강남파이낸스센터 PNU (검증된 fixture, 항상 존재)
    let pnu_str = std::env::var("GONGZZANG_DRIFT_TEST_PNU")
        .unwrap_or_else(|_| "1168010100107370000".to_owned());
    let pnu = Pnu::try_new(&pnu_str).expect("valid PNU");

    let buildings = client.building_register()
        .fetch_title_info(split(&pnu)).await
        .expect("정부 API 호출 + parse 통과")
        .as_object().cloned().expect("response object");

    let parsed = parse_building_title(&Value::Object(buildings), &pnu, &dummy_polygon(), Utc::now())
        .expect("parser 통과");

    assert!(!parsed.is_empty(), "응답 0건 — endpoint drift 의심");
    let b = &parsed[0];
    assert_eq!(b.main_purpose_code, BuildingPurposeCode::Office,
        "강남파이낸스센터 mainPurpsCd → Office 매핑 검증");
    assert_eq!(b.structure_code, BuildingStructureCode::SteelReinforcedConcrete);
}
```

### 6.3 V-World smoke test

```rust
// crates/data-clients/vworld/tests/smoke_real_api.rs
#![cfg(feature = "real-api")]

#[tokio::test]
#[ignore]
async fn smoke_vworld_parcel_alive() {
    let client = VWorldClient::from_env().expect("VWORLD_API_KEY + VWORLD_DOMAIN required");

    // 강남파이낸스센터 PNU
    let pnu = Pnu::try_new("1168010100107370000").unwrap();

    let parcel = client.parcel_reader().fetch_by_pnu(&pnu).await
        .expect("V-World 호출 + parse 통과")
        .expect("필지 존재");

    assert_eq!(parcel.pnu.as_str(), "1168010100107370000");
    // jiyok_cd 같은 핵심 필드 존재 검증
}
```

### 6.4 simulate_failure 환경변수

`GONGZZANG_DRIFT_TEST_PNU=9999999999999999999` 로 호출 시 응답 0건 → assert fail → 의도된 fail.

---

## 7. GitHub Actions Workflow

### 7.1 `.github/workflows/api-drift-smoke-test.yml`

```yaml
name: api-drift-smoke-test

on:
  schedule:
    - cron: '0 19 * * *'  # 04:00 KST (UTC+9)
  workflow_dispatch:
    inputs:
      simulate_failure:
        description: 'Force fail (drift detection 검증)'
        type: boolean
        default: false

jobs:
  smoke-data-go-kr:
    runs-on: ubuntu-24.04
    timeout-minutes: 10
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: smoke test
        id: smoke
        env:
          ODP_SERVICE_KEY: ${{ secrets.ODP_SERVICE_KEY }}
          GONGZZANG_DRIFT_TEST_PNU: >-
            ${{ inputs.simulate_failure
              && '9999999999999999999'
              || '1168010100107370000' }}
        run: |
          cargo test --features real-api -p data-go-kr-client \
            --test smoke_real_api -- --ignored --nocapture
        continue-on-error: true
      - name: record to DB + alert
        env:
          DATABASE_URL: ${{ secrets.STAGING_DATABASE_URL }}
          API_NAME: data_go_kr.getBrTitleInfo
          STATUS: ${{ steps.smoke.outcome }}
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          CRON_RUN: ${{ github.event_name == 'schedule' }}
        run: ./scripts/api-drift-record-and-alert.sh

  smoke-vworld:
    runs-on: ubuntu-24.04
    # smoke-data-go-kr 와 동일 구조
    ...
```

### 7.2 `scripts/api-drift-record-and-alert.sh`

이 스크립트가:
1. `psql` 또는 별도 Rust binary 로 `api_health_check` 에 INSERT
2. fail 인 경우:
   - `is_n_cron_runs_failed(api_name, 3)` 쿼리 → true 면 escalation
   - `status="parse_fail"` 또는 `http_4xx` 면 즉시 escalation
   - 기존 open Issue 가 있으면 comment 추가, 없으면 신규 Issue
3. success + 기존 open Issue 있으면 close (자가 복구)

**구현: B 채택 (Rust binary `crates/api-health-recorder/`)**

**이유:**
- 기존 `PgHealthCheckRepository` 재사용 (SSS 일관성)
- type-safe (bash 보다 robust)
- `octocrab` (Rust GitHub API client) 로 Issue 자동 생성 / close
- workflow yml 은 단순 orchestration 만 (`cargo run --bin api-health-recorder -- --api-name X --status Y`)

**대안 (bash + psql + gh CLI) 거부 이유:** bash logic 이 복잡해질 때 type 검증 X. 1500줄 안티패턴 회피 (스크립트가 길어지면 차라리 Rust).

### 7.3 secrets 목록

| Secret | 용도 |
|---|---|
| `ODP_SERVICE_KEY` | data.go.kr 호출 |
| `VWORLD_API_KEY` | V-World 호출 |
| `VWORLD_DOMAIN` | V-World 도메인 등록 (`localhost`) |
| `STAGING_DATABASE_URL` | DB write (별도 staging DB 권장 — production DB 분리) |

**STAGING_DATABASE_URL 결정:**
- 옵션 A: production DB 와 동일 (간단)
- 옵션 B: 별도 staging DB (격리, 비용 ↑)
- → **옵션 A 채택** (1인 단계, drift 결과는 production 에 영향 0). 미래 production scale 시 분리.

---

## 8. 알림 정책 (GitHub Issue)

### 8.1 Issue 자동 생성 조건

| 분류 | trigger 조건 | label |
|---|---|---|
| **drift:5xx-server** | 3 cron runs 연속 fail | `drift`, `drift:5xx-server`, `drift:3-day-streak` |
| **drift:4xx-auth** | 1회 fail (즉시) | `drift`, `drift:4xx-auth` |
| **drift:schema** | 1회 fail (즉시) | `drift`, `drift:schema` |
| **drift:timeout** | 3 cron runs 연속 fail | `drift`, `drift:timeout`, `drift:3-day-streak` |

### 8.2 Issue 본문 포맷

```markdown
## 발견 시각
2026-05-XX 04:00 KST (cron run)

## 분류
schema-mismatch (parse_building_title fail)

## API
data.go.kr.getBrTitleInfo

## 실패 log (secrets masked)
<test stderr>

## 최근 cron run 결과 (DB 쿼리 결과)
| checked_at         | status      | http_code |
| ------------------ | ----------- | --------- |
| 2026-05-05 04:00   | parse_fail  | 200       |
| 2026-05-04 04:00   | parse_fail  | 200       |
| 2026-05-03 04:00   | parse_fail  | 200       |

## 수동 검증
GitHub Actions → "api-drift-smoke-test" → "Run workflow" 버튼

## 자가 복구
다음 cron 정상 응답 시 이 Issue 자동 close (label `drift:auto-recovered` 추가).
```

### 8.3 Assignees / Notification

- Assignees: `w1kch9812` (repo owner)
- Notification: GitHub default (이메일/푸시)
- Slack/Sentry/Discord 통합: SP7-i 이후

### 8.4 자가 복구

workflow success + `find_latest_open_issue(api_name, label="drift")` 가 있으면:
1. comment 추가: "✅ 자가 복구 — 정부 일시 장애였음"
2. label 추가: `drift:auto-recovered`
3. issue close

---

## 9. 검증 / 테스트 전략

### 9.1 `crates/operations/api-health` 단위 테스트

- `HealthStatus::is_hard_fail()` enum behavior
- `HealthCheckRecord` serde / Display
- `NewHealthCheck` 빌더

### 9.2 `crates/db/api_health.rs` 통합 테스트

- `PgHealthCheckRepository::record()` happy path
- `is_n_cron_runs_failed(api_name, 3)` 다양한 시나리오:
  - 3 cron 모두 fail → true
  - 3 cron 중 1 success → false
  - 수동 trigger fail 만 있음 → false (cron 만 카운트)
  - 데이터 없음 → false

### 9.3 smoke test 자체

- 외부 API 직접 호출 = mock 무의미 → 단위 테스트 X
- 검증 방법: workflow_dispatch 수동 trigger (정상 path + simulate_failure path)

### 9.4 workflow yml + script

- 첫 push 후 workflow_dispatch 1회 (정상 path)
- workflow_dispatch + `simulate_failure=true` 1회 (Issue 자동 생성 검증)
- 다음 cron 정상 응답 시 자가 복구 검증

### 9.5 docs/observability/api-drift-smoke-test.md

운영 절차 SSOT — 위 검증 절차 명시 + 실패 분류 표 + escalation 정책.

---

## 10. Migration 진화 path

### 10.1 SP7-i (Sentry) 통합 시점

`api_health_check` 테이블은 그대로. Sentry 는 production code 의 panic / breaker open 등 **별도 이벤트 종류**:

```
SP7-iii: drift 정기 검출 → api_health_check 테이블
SP7-i:   에러 자동 추적   → Sentry SaaS
SP7-ii:  metrics          → Grafana Cloud / Prometheus
```

각 SSOT 가 분류별로 명확. 통합 시 변경 0.

### 10.2 SP-Admin (React Flow 시각화) 통합

미래 admin UI 가 `api_health_check` 쿼리 → React Flow 노드 그래프로 시각화:

```
[강남파이낸스 PNU 호출]
   ↓ ✅ (최근 cron success)
[BuildingRegisterClient]
   ↓ ✅ HTTP 200
[parse_building_title]
   ↓ ✅ Office 매핑
[검증 완료]
```

또는 fail 시 빨간 노드로 표시 + DB 의 error_detail 클릭 시 detail.

### 10.3 production scale 진화

1인 단계: STAGING_DATABASE_URL = production DB
production scale: 별도 staging DB + replication

이건 SP8 (IaC) 영역.

---

## 11. Follow-up 변경

### 11.1 본 sub-project 가 closing 하는 FU

- **FU 45**: 정부 API endpoint URL drift staging-only smoke test → ✅ closed
- **FU 46**: 정부 API JSON Number vs String schema drift 모니터링 → ✅ closed

### 11.2 본 sub-project 가 흡수 안 한 FU

- **FU 47**: V-World 지오코딩 (주소 → PNU) — SP6 frontend 또는 dev tool sub-project 로 분리

### 11.3 본 sub-project 에서 발견될 가능성

- 정부 API 의 추가 endpoint (실거래가 / 법제처) 도 같은 패턴 적용 — SP4-iii-b/c 도입 시 smoke test 자연 추가
- `api_health_check` 테이블에 추가 분류 (예: response_size / schema_version) 발견 시 별도 FU

---

## 12. 작업 단위 (T1-T6)

### T1: DB 마이그레이션 + 도메인 crate
- `30007_api_health_check.sql` 작성
- `crates/operations/api-health/` 신규 crate (entity / status / repository trait)
- 단위 테스트 (HealthStatus enum + record builder)
- 누적 테스트 ≥1259 → ≥1267

### T2: PgHealthCheckRepository
- `crates/db/api_health.rs` 신규
- `record()` / `is_n_cron_runs_failed()` / `find_latest()` 구현
- 통합 테스트 (Postgres 실 DB) — 4-5 시나리오
- workspace 통합 테스트 ≥110 → ≥114

### T3: data.go.kr smoke test
- `crates/data-clients/data-go-kr/Cargo.toml` 에 `real-api` feature
- `tests/smoke_real_api.rs` 신규 (feature-gated)
- 로컬 검증: `cargo test --features real-api -- --ignored` 1회 통과

### T4: V-World smoke test
- 동일 패턴으로 `crates/data-clients/vworld/`
- 로컬 검증

### T5: api-health-recorder Rust binary
- `crates/api-health-recorder/` 신규 crate (binary)
- `octocrab` (GitHub API client) 의존성 추가
- CLI 인자: `--api-name <X> --status <Y> --http-code <Z> --error-detail <log>`
- 동작: PgHealthCheckRepository 로 record + (fail 시) Issue 자동 생성/comment + (success 시) 기존 open Issue 자가 복구
- 단위 테스트: CLI parsing + 분기 로직

### T6: GitHub Actions workflow + secrets + 검증
- `.github/workflows/api-drift-smoke-test.yml`
- secrets 등록 (사용자 작업 — 4개: ODP_SERVICE_KEY / VWORLD_API_KEY / VWORLD_DOMAIN / STAGING_DATABASE_URL)
- workflow_dispatch 정상 path 검증
- workflow_dispatch + simulate_failure path 검증 (Issue 자동 생성 확인)
- 자가 복구 검증 (다음 정상 run 시 Issue 자동 close)

### T6: docs + Issue 자동 생성 검증
- `docs/observability/api-drift-smoke-test.md` 작성
- simulate_failure 1회 → Issue 자동 생성 확인
- 자가 복구 1회 검증
- `roadmap.md` 갱신 (SP7-iii ✅ closed, SP7-i/ii 자리 명시)

---

## 13. 추정

- **작업량**: 4-5일 (분해 됨, T1-T6)
- **신규 crate**: 2 (`crates/operations/api-health` 도메인 + `crates/api-health-recorder` binary)
- **신규 마이그레이션**: 1 (`30007_api_health_check.sql`)
- **신규 workflow**: 1 (`api-drift-smoke-test.yml`)
- **신규 docs**: 1 (`docs/observability/api-drift-smoke-test.md`)
- **누적 통계 변화**: 31 crate → 33 / 1259 tests → ~1285

---

## 14. SSS 자가 평가

| 기둥 | 보장 |
|---|---|
| 1 일관성 | ◎ (Repository pattern + 통합 테스트 패턴 그대로) |
| 2 자동 강제 | ◎ (cron + DB record + Issue 자동) |
| 3 추적성 | ◎ (api_health_check 영구 보존 + workflow run history) |
| 4 안전성 | ◎ (feature flag + production code path 검증) |
| 5 가시성 | ◎ (Issue alert + DB 쿼리 + 미래 admin UI) |
| 6 SSOT | ◎ (drift 결과 = 우리 DB; Issue 는 사람 알림 사본) |
| 7 명확성 | ◎ (docs/observability + simulate_failure 영구 인프라) |

= **근본 SSS 80%+ 달성**.
