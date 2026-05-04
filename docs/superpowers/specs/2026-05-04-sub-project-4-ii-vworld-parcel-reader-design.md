# Sub-project 4-ii: V-World 외부 API + ParcelReader (Spec)

| | |
|---|---|
| 작성일 | 2026-05-04 |
| 상태 | Approved |
| 선행 | SP5-ii (13 BC RDS Repo 정합), SP4-i (Outbox publisher pattern), SP2b-ii (`ParcelReader` port) |
| 후속 | SP4-iii (data.go.kr + 법제처 + R2 Reader 6 + 분산 락) |
| 관련 ADR | ADR-0006 (외부 API 표준), `docs/data-sources/v-world.md`, `docs/backend/circuit-breaker.md` |

---

## 1. 개요 / 동기

지금까지 13 BC 모두 transactional save/insert 패턴으로 정합됐고, outbox publisher 가 read side 를 닫았어요. 그런데 **외부 데이터 0** — V-World/data.go.kr/법제처 API 통합 부재. *공짱* 의 차별점이 산업 부동산 데이터인데, 데이터 소스가 0 이면 빈 매물 게시판.

본 SP 가 도입하는 것:

1. **`crates/circuit-breaker`** — 외부 API 호출 표준 미들웨어 (현재 README 만 있는 stub). 모든 후속 외부 API 클라이언트가 활용
2. **`crates/data-clients/vworld`** — V-World 단일 API 통합. `Parcel` 도메인 Reader 구현체.
3. **Anti-Corruption Layer 패턴 검증** — V-World 외부 스키마(GeoJSON 형태) → 도메인 `Parcel` Aggregate 변환 격리

본 SP 가 작은 *정합* 검증 — V-World 만 + `fetch_by_pnu` 만. data.go.kr / 법제처 / `fetch_markers_in_bbox` PMTiles 는 SP4-iii.

---

## 2. 범위

### 포함

- **`crates/circuit-breaker` 신규 라이브러리**:
  - `Policy` struct — `timeout_ms`, `max_retries`, `retry_base_ms`, `open_threshold`, `open_window_ms`, `open_cooldown_ms`
  - `Breaker` — sliding window failure 카운터 + state machine (`Closed` / `Open` / `HalfOpen`)
  - `execute<F, T, E>(breaker, policy, op_name, fut) -> Result<T, BreakerError<E>>` — timeout + retry + state 추적
  - `BreakerError<E>` enum — `Inner(E)` / `Timeout` / `Open` / `MaxRetriesExceeded`
  - 단위 테스트 ~12 (state transitions, retries, timeouts)
- **`crates/data-clients/vworld` 신규 라이브러리**:
  - `VWorldConfig` — `api_key`, `domain`, `base_url` (default `https://api.vworld.kr`)
  - `VWorldClient` — `reqwest::Client` + `Breaker` + `Policy`
  - `VWorldParcelReader` — `ParcelReader` 구현체:
    - `fetch_by_pnu(pnu)`: WFS GetFeature 호출 + parse → `Parcel`
    - `fetch_markers_in_bbox(bbox)`: SP4-ii 미구현 — `Err(Fetch("bbox markers deferred to SP4-iii"))` 반환 (honest failure)
  - `RawCapture` trait — `async fn capture(pnu, source: &str, raw: serde_json::Value, fetched_at)` (raw_response 보존 hook)
  - V-World JSON → `Parcel` 변환 (Anti-Corruption Layer):
    - PNU 19자리 추출 (V-World feature 의 `pnu` property)
    - `geometry` (GeoJSON Polygon) → `PolygonSrid` (WGS84)
    - `area` `m²` 추출
    - `land_use_type` ↔ V-World 지목코드 (대/전/답/임야 등)
    - `zoning` ↔ V-World 용도지역 코드
    - `admin_division` ↔ PNU 앞 10자리 분해
    - `road_address` / `jibun_address` 일부 필드만 (V-World 응답에 항상 포함되지 않음)
- **`Cargo.toml` workspace deps 추가**: `wiremock = "0.6"` (테스트 mock HTTP server)
- **단위 테스트**:
  - JSON parser fixture 기반 (V-World 샘플 응답 → Parcel) — 5-7 tests
  - HTTP integration tests (wiremock — fake V-World server) — 5-7 tests
  - Circuit breaker state machine — 10-12 tests

### 미포함

- **`fetch_markers_in_bbox`** — PMTiles streaming + bbox WFS — SP4-iii
- **raw_response DB 저장** — `parcel_external_data` 테이블 + 마이그 — SP4-iii (또는 별도)
- **Redis 캐시 레이어 (TTL 24h)** — `crates/cache` 정합 후 SP7
- **data.go.kr / 법제처** — SP4-iii
- **Naver Maps / NICE 본인인증 / Gemini Embedding** — 별도 sub-project
- **Sentry alert on circuit open** — 관측성 sub-project (SP7)
- **Distributed circuit breaker** (Redis 공유 state) — 멀티 인스턴스 시 SP4-iii+
- **rate limit / governor** — 단일 인스턴스 단순 카운터로 충분 (V-World 일일 한도 정책 별도)
- **API 키 vault 통합** — env var (`VWORLD_API_KEY` / `VWORLD_DOMAIN`) 직접

---

## 3. 아키텍처

```
┌────────────────────────────────────────────────────┐
│  Application (services/api handler 등 - 후속 SP)  │
│  → reader: Arc<dyn ParcelReader>                   │
│  → reader.fetch_by_pnu(&pnu).await                 │
└──────────┬─────────────────────────────────────────┘
           │ trait 호출
           ▼
┌────────────────────────────────────────────────────┐
│  crates/data-clients/vworld                        │
│  ┌─────────────────────────────────────────────┐  │
│  │ VWorldParcelReader::fetch_by_pnu            │  │
│  │  1. URL build (WFS GetFeature?data=...&     │  │
│  │     geomFilter=POINT(lng lat)&pnu=...)      │  │
│  │  2. circuit_breaker::execute(               │  │
│  │     breaker, policy, "vworld.parcel",       │  │
│  │     async { client.get(url).send().await })│  │
│  │  3. raw_capture(pnu, "vworld", json, now)   │  │
│  │  4. ACL: V-World feature → Parcel           │  │
│  │  5. Ok(Some(Parcel))                        │  │
│  └─────────────────────────────────────────────┘  │
└────────────┬───────────────────────────────────────┘
             │ via Breaker
             ▼
┌────────────────────────────────────────────────────┐
│  crates/circuit-breaker                            │
│  ┌─────────────────────────────────────────────┐  │
│  │ Breaker { state, failures, last_open_at }   │  │
│  │  - Closed: 정상 통과                         │  │
│  │  - Open: 즉시 BreakerError::Open 반환       │  │
│  │  - HalfOpen: 1 회 trial 허용                │  │
│  └─────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────┘
             │ HTTP (with timeout + retry)
             ▼
┌────────────────────────────────────────────────────┐
│  reqwest → V-World API                             │
└────────────────────────────────────────────────────┘
```

---

## 4. 컴포넌트 정의

### 4.1 `crates/circuit-breaker/src/policy.rs`

```rust
#[derive(Debug, Clone, Copy)]
pub struct Policy {
    /// 단일 호출 timeout.
    pub timeout_ms: u64,
    /// 재시도 횟수 (총 시도 = max_retries + 1).
    pub max_retries: u32,
    /// 첫 retry 까지 base delay (지수 백오프 × 2^attempt + jitter).
    pub retry_base_ms: u64,
    /// open 트리거 — open_window_ms 안에 N 회 실패하면 open.
    pub open_threshold: u32,
    /// failure window 길이.
    pub open_window_ms: u64,
    /// open → half-open 까지 cooldown.
    pub open_cooldown_ms: u64,
}

impl Policy {
    /// V-World 표준 정책 (docs/data-sources/v-world.md § Circuit Breaker 정책).
    pub const fn vworld_default() -> Self {
        Self {
            timeout_ms: 10_000,
            max_retries: 1,
            retry_base_ms: 1_000,
            open_threshold: 5,
            open_window_ms: 5_000,
            open_cooldown_ms: 30_000,
        }
    }
}
```

### 4.2 `crates/circuit-breaker/src/breaker.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState { Closed, Open, HalfOpen }

pub struct Breaker {
    inner: Mutex<Inner>,
}

struct Inner {
    state: CircuitState,
    /// 최근 open_window_ms 안의 실패 시각들 (가장 오래된 것 pop).
    recent_failures: VecDeque<Instant>,
    /// 마지막 open 시각 (HalfOpen 전이 판단).
    opened_at: Option<Instant>,
}

impl Breaker {
    pub fn new() -> Self;
    /// 호출 가능 여부 — Open 이고 cooldown 안 지났으면 Err(Open).
    pub fn check(&self, policy: &Policy) -> Result<CircuitState, ()>;
    /// 성공 기록 — HalfOpen 이면 Closed 로 전이, recent_failures 비움.
    pub fn record_success(&self);
    /// 실패 기록 — recent_failures 추가, threshold 초과 시 Open 으로 전이.
    pub fn record_failure(&self, policy: &Policy);
}
```

### 4.3 `crates/circuit-breaker/src/execute.rs`

```rust
pub async fn execute<F, Fut, T, E>(
    breaker: &Breaker,
    policy: &Policy,
    op_name: &'static str,
    op: F,
) -> Result<T, BreakerError<E>>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    // 1. breaker.check() → Open 이면 즉시 반환
    // 2. 0..=max_retries:
    //    - tokio::time::timeout(timeout_ms, op()).await
    //    - 성공: record_success → return Ok
    //    - timeout: record_failure
    //    - inner err: record_failure
    //    - retry 시 backoff: retry_base_ms * 2^attempt
    // 3. retries 다 쓰면 MaxRetriesExceeded
}

#[derive(Debug, Error)]
pub enum BreakerError<E: Display> {
    #[error("circuit open — too many recent failures")]
    Open,
    #[error("operation timed out after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },
    #[error("max retries exceeded ({max_retries}): last error: {last}")]
    MaxRetriesExceeded { max_retries: u32, last: String },
    #[error(transparent)]
    Inner(E),
}
```

### 4.4 `crates/data-clients/vworld/src/lib.rs`

```rust
pub mod client;
pub mod parser;
pub mod raw_capture;
pub mod reader;

pub use client::{VWorldClient, VWorldConfig};
pub use raw_capture::{NoOpRawCapture, RawCapture};
pub use reader::VWorldParcelReader;
```

### 4.5 `crates/data-clients/vworld/src/raw_capture.rs`

```rust
#[async_trait]
pub trait RawCapture: Send + Sync {
    async fn capture(
        &self,
        pnu: &str,
        source: &str,
        raw: &serde_json::Value,
        fetched_at: DateTime<Utc>,
    ) -> Result<(), RawCaptureError>;
}

/// 기본 구현 — `tracing::info!` 로 raw_response 흔적 로깅. SP4-iii 의 DB
/// `parcel_external_data` 저장 도입 전 임시.
pub struct NoOpRawCapture;
```

### 4.6 `crates/data-clients/vworld/src/parser.rs`

V-World WFS GetFeature 응답 → `Parcel` 변환 (Anti-Corruption Layer).

```rust
pub fn parse_parcel(raw: &serde_json::Value, fetched_at: DateTime<Utc>) -> Result<Parcel, ParseError>;
```

V-World 응답 구조 (docs § 요청 예시):
```json
{
  "response": {
    "result": {
      "featureCollection": {
        "features": [
          {
            "geometry": { "type": "Polygon", "coordinates": [[[lng,lat], ...]] },
            "properties": {
              "pnu": "1111010100100010000",
              "jibun": "1-1",
              "addr": "서울특별시 종로구 청운동",
              "lndcgr_code": "01",
              "lndcgr_nm": "대",
              "lndpcl_ar": 250.0,
              "uq_cd": "11",
              "uq_nm": "주거지역"
            }
          }
        ]
      }
    }
  }
}
```

ACL 매핑:
- `properties.pnu` → `Pnu::try_new`
- `properties.lndpcl_ar` → `AreaM2::try_new`
- `properties.lndcgr_nm` → `LandUseType` (도메인 enum)
- `properties.uq_nm` → `Zoning`
- `geometry.coordinates` → `PolygonSrid::try_new_wgs84`
- PNU 앞 10자리 → `AdminDivision`
- `properties.addr` → `JibunAddress`

### 4.7 `crates/data-clients/vworld/src/client.rs`

```rust
pub struct VWorldConfig {
    pub api_key: String,
    pub domain: String,
    pub base_url: String,
}

impl VWorldConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        // env: VWORLD_API_KEY / VWORLD_DOMAIN / VWORLD_BASE_URL (default https://api.vworld.kr)
    }
}

pub struct VWorldClient {
    http: reqwest::Client,
    config: VWorldConfig,
    breaker: Breaker,
    policy: Policy,
}

impl VWorldClient {
    pub fn new(config: VWorldConfig) -> Self;
    pub fn with_policy(config: VWorldConfig, policy: Policy) -> Self;

    /// V-World WFS GetFeature 호출 — raw JSON 반환.
    pub async fn fetch_feature_by_pnu(&self, layer: &str, pnu: &str)
        -> Result<serde_json::Value, BreakerError<reqwest::Error>>;
}
```

### 4.8 `crates/data-clients/vworld/src/reader.rs`

```rust
pub struct VWorldParcelReader {
    client: Arc<VWorldClient>,
    raw_capture: Arc<dyn RawCapture>,
}

#[async_trait]
impl ParcelReader for VWorldParcelReader {
    async fn fetch_by_pnu(&self, pnu: &Pnu) -> Result<Option<Parcel>, ReaderError>;
    async fn fetch_markers_in_bbox(&self, bbox: &BoundingBox) -> Result<Vec<ParcelMarker>, ReaderError>;
    // bbox markers 는 v1 미구현 — Err(Fetch("bbox markers deferred to SP4-iii"))
}
```

`fetch_by_pnu` 동작:
1. `client.fetch_feature_by_pnu("LT_C_UQ111", pnu.as_str()).await` — 용도지역 레이어 조회
2. raw 응답 → `raw_capture.capture(pnu, "vworld", raw, now)` (best-effort, 실패 시 warn)
3. `parser::parse_parcel(raw, now)` → `Parcel`
4. raw 응답이 빈 featureCollection → `Ok(None)`
5. BreakerError → `ReaderError::Fetch(msg)` 매핑
6. ParseError → `ReaderError::Parse(msg)` 매핑

---

## 5. 데이터 흐름

### 5.1 정상 fetch
```
[1] reader.fetch_by_pnu(pnu)
[2] VWorldClient.fetch_feature_by_pnu("LT_C_UQ111", pnu)
[3] breaker.execute(..., op = || http.get(url).send().await)
[4] HTTP 200 + JSON
[5] raw_capture.capture(pnu, "vworld", json, now)  // tracing event
[6] parser.parse_parcel(json, now) → Parcel
[7] Ok(Some(parcel))
```

### 5.2 외부 5xx 흐름
```
[3] http.get(url) → 5xx
[4] retry 1회 → 또 5xx
[5] breaker.record_failure ×2 (window 안 5번 도달 안하면 Closed 유지)
[6] BreakerError::MaxRetriesExceeded → ReaderError::Fetch(msg)
```

### 5.3 circuit open 흐름
```
[1] 5초 안 5번 실패 누적 → breaker open
[2] 다음 fetch_by_pnu 호출 → breaker.check() = Open
[3] BreakerError::Open → ReaderError::Fetch("circuit open")
[4] 30초 cooldown 후 HalfOpen → trial 1회 허용
[5] trial 성공 → Closed
```

---

## 6. 에러 매핑

| BreakerError | ReaderError | 의미 |
|---|---|---|
| `Open` | `Fetch("circuit open")` | 외부 API 일시 차단됨 |
| `Timeout` | `Fetch("timeout")` | 10초 내 응답 없음 |
| `MaxRetriesExceeded` | `Fetch(msg)` | 모든 재시도 실패 |
| `Inner(reqwest::Error)` | `Fetch(msg)` | HTTP 자체 에러 |
| ParseError | `Parse(msg)` | 응답 형식 깨짐 |

---

## 7. 가시성

- 모든 `breaker::execute` 호출이 `tracing::instrument(skip(op), fields(op_name))` — 호출 시도/실패/재시도/state 전이 모두 구조화 이벤트
- `VWorldClient` 메서드 instrument: `pnu`, `layer`, `attempt`
- `parse_parcel` instrument: `pnu`, `feature_count`
- `RawCapture::capture` (`NoOpRawCapture`): target = `"vworld.raw"`, fields = `pnu`, `source`, `bytes` (raw 크기), `fetched_at`. payload 자체는 `skip` (PII 가능)

---

## 8. 테스트

### 단위 테스트 (`crates/circuit-breaker/`)
- `policy_vworld_default` 값 확인
- `breaker_starts_closed`
- `breaker_transitions_to_open_after_threshold_failures`
- `breaker_stays_closed_below_threshold`
- `breaker_open_blocks_calls`
- `breaker_transitions_to_half_open_after_cooldown`
- `breaker_half_open_success_transitions_to_closed`
- `breaker_half_open_failure_transitions_back_to_open`
- `execute_returns_inner_ok_immediately`
- `execute_retries_on_inner_err`
- `execute_returns_max_retries_exceeded_after_all_fails`
- `execute_timeout_records_failure`
- `execute_returns_open_when_breaker_open`

### 단위 테스트 (`crates/data-clients/vworld/`)
- `parser_parse_valid_parcel_json`
- `parser_parse_empty_feature_collection_returns_none`
- `parser_parse_missing_pnu_returns_error`
- `parser_parse_malformed_geometry_returns_error`
- `parser_parse_korean_addr_round_trip`
- `client_from_env_validates_required_vars`
- `noop_raw_capture_logs_via_tracing`

### HTTP 통합 테스트 (`crates/data-clients/vworld/tests/`)
- `wiremock` 으로 fake V-World server:
  - `fetch_by_pnu_happy_path` — 200 + 유효 JSON → Some(Parcel)
  - `fetch_by_pnu_empty_feature_returns_none` — 200 + empty featureCollection
  - `fetch_by_pnu_5xx_retries_then_fails` — 500 ×2 → ReaderError::Fetch
  - `fetch_by_pnu_circuit_opens_after_threshold` — 5xx ×5 → 다음 호출 즉시 Open 에러
  - `fetch_by_pnu_malformed_response_returns_parse_error`
  - `fetch_markers_in_bbox_returns_deferred_error` (honest failure)

`wiremock` 은 dev-dependency 만 추가.

---

## 9. CI 통합

- CI workflow 변경 0
- `cargo test --workspace --all-features` 가 자동 실행
- wiremock 은 self-contained (외부 네트워크 0)

---

## 10. 검증 기준 (DoD)

1. `crates/circuit-breaker` 신규 — Policy + Breaker + execute + 12+ 단위 테스트
2. `crates/data-clients/vworld` 신규 — Config + Client + Reader + Parser + RawCapture + 7+ 단위 테스트 + 6 통합 테스트
3. 워크스페이스 `Cargo.toml.members` 에 두 신규 crate
4. `wiremock` 워크스페이스 dev-dep 추가
5. 3 CI workflow 그린 (CI / db-migrations / walking-skeleton)
6. clippy `-D warnings` 통과
7. tarpaulin ≥ 90% 유지
8. 누적 테스트 ≥ 1190 (~1166 + 25 신규)
9. 모든 파일 ≤ 500 권장
10. SSOT 갱신 (roadmap + memory + MEMORY.md)

---

## 11. SSS 7 기둥 매핑

| 기둥 | 적용 |
|---|---|
| 1 일관성 | 모든 외부 API 호출이 `circuit_breaker::execute` 통과 — 후속 data.go.kr / 법제처도 같은 패턴 강제 |
| 2 자동 강제 | `Breaker` 가 5번 실패 시 자동 차단 — 사람 개입 0. lint (`crates/data-clients/*` 가 `crates/circuit-breaker` 통하지 않고 reqwest 직접 호출 차단) 는 별도 (FU 26) |
| 3 추적성 | 모든 외부 호출이 `tracing::instrument` + raw_response capture (현재는 tracing event, SP4-iii 에서 DB) |
| 4 안전성 | timeout + retry + circuit open — 외부 장애가 우리 시스템 panic 으로 전파 안 됨. parameterized URL only (SQL injection 불가능 영역). `unsafe` 0 |
| 5 가시성 | tick report 는 없지만 모든 호출이 tracing event. circuit state 전이 시 `warn!`/`info!` |
| 6 SSOT | V-World 정책 = `Policy::vworld_default()` 하드코딩 (도메인 의미). `crates/data-clients/<api>/policy.rs` 패턴 확립 |
| 7 명확성 | `BreakerError` enum variants 가 4 가지 실패 모드 명시. 후속 후행 SP 가 같은 어휘 사용 |

---

## 12. Follow-up

- **FU 26**: `clippy::disallowed_types` 로 `crates/data-clients/*` 가 `reqwest::Client::*` 직접 호출 차단 — 후속 SP4-iii
- **FU 27**: `parcel_external_data` 테이블 + 마이그 + DB 저장 `RawCapture` 구현체 — SP4-iii
- **FU 28**: Redis 캐시 레이어 (TTL 24h) — `crates/cache` 정합 후 SP7
- **FU 29**: Sentry alert on `Breaker` open — SP7 관측성과 묶음
- **FU 30**: `fetch_markers_in_bbox` PMTiles 또는 V-World BBOX WFS 구현 — SP4-iii
- **FU 31**: Distributed circuit breaker (Redis 공유 state) — 멀티 인스턴스 SP4-iv+
- **FU 32**: `governor` rate limit — V-World 일일 쿼터 보호
