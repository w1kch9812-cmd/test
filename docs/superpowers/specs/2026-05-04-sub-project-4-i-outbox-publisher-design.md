# Sub-project 4-i: Outbox Publisher Worker (Spec)

| | |
|---|---|
| 작성일 | 2026-05-04 |
| 상태 | Approved |
| 선행 | SP5-iii (Outbox 패턴 write side), SP5-iv (9 BC 정합) |
| 후속 | SP4-ii (V-World 외부 API), SP5-ii (Insights BC RDS) |
| 관련 ADR | — |

---

## 1. 개요 / 동기

SP5-iii/iv 가 9 BC 모두에 transactional `outbox_event` INSERT 를 깔았어요. 그런데 **publisher worker 는 0** — `outbox_event` row 가 무한 누적 + 외부 발행 0. 약속의 절반만 채워진 상태로 SSS 1번(일관성)·5번(가시성)·4번(안전성) 부분 위반.

본 sub-project 는 **최소 기능 publisher** 를 도입해 약속의 read side 를 닫는 게 목표예요:

- 폴링 루프 (Tokio interval) — `fetch_unpublished` → `Sink.publish` → `mark_published`
- `Sink` 는 trait — v1 의 default 구현은 **structured tracing event 발행** (외부 시스템 통합 X). 실제 Kafka/Webhook/SQS 같은 sink 는 후속 sub-project (SP4-iii+) 에서 같은 trait 구현체로 추가
- **단일 인스턴스 가정** — 분산 환경 advisory lock / `SELECT FOR UPDATE SKIP LOCKED` 는 SP4-i 미포함 (FU)
- 실패 시 publish 안 한 row 는 그대로 남음 → 다음 tick 에 재시도 (멱등). max-retry / DLQ 는 FU

본 SP 는 *작은 기둥 닫기* — 새 도메인 0, 새 외부 API 0, 새 스키마 0. 패턴 + 테스트 + 워크스페이스 신규 멤버 2 개만.

---

## 2. 범위 (Scope)

### 포함

- **신규 라이브러리 crate** `crates/outbox-publisher/`:
  - `Sink` trait — `async fn publish(&self, event: &OutboxEvent) -> Result<(), SinkError>`
  - `LoggingSink` 구현체 — `tracing::info!` 로 구조화 이벤트 발행 (event_type / aggregate_kind / aggregate_id / correlation_id 모두 fields)
  - `CountingSink` 테스트용 in-memory 카운터 sink
  - `tick(repo, sink, limit) -> Result<TickReport>` — 한 번의 폴링 사이클 단위 (테스트 friendly)
  - `PublisherError` enum
- **신규 binary crate** `services/outbox-publisher/`:
  - `main.rs` — 환경변수 로드 (`DATABASE_URL`, `OUTBOX_POLL_INTERVAL_MS`, `OUTBOX_BATCH_SIZE`), `PgOutboxRepository` 빌드, `LoggingSink` 빌드, Tokio interval loop, SIGTERM/Ctrl+C graceful shutdown
- **워크스페이스 Cargo.toml 갱신** — `members` 에 두 신규 crate 추가
- **통합 테스트** `crates/db/tests/outbox_publisher_integration.rs`:
  - `tick_publishes_unpublished_rows` — 3 row INSERT → `tick` 1 회 → 모두 `published_at NOT NULL` + `CountingSink` 카운터 = 3
  - `tick_skips_already_published` — 이미 published 된 row 는 polling 안 잡힘
  - `tick_returns_zero_when_no_rows` — 빈 테이블에서 `tick` 호출 시 `published = 0`
  - `tick_failure_leaves_row_unpublished` — `FailingSink` (테스트 inline) → row 그대로 (`published_at IS NULL`)
- **단위 테스트** in `crates/outbox-publisher/`:
  - `LoggingSink::publish` smoke (panic 없음)
  - `CountingSink` 카운터 정확
  - `PublisherError` 메시지 형식
- **README** 짧게 — 기동/환경변수/ FU 명시

### 미포함 (후속)

- **분산 락 / 멀티 인스턴스** — Postgres advisory lock 또는 `SELECT FOR UPDATE SKIP LOCKED` — SP4-i 안 함, 단일 인스턴스 가정. `services/worker/README.md` 의 ShedLock 패턴 후속 도입
- **외부 sink 구현체** (Kafka, Webhook HTTP POST, SQS, NATS 등) — `Sink` trait 만 정의, 구현체는 SP4-iii+
- **재시도 정책 / Exponential backoff / DLQ** — v1 은 무한 재시도 (idempotent fetch). max_retry 컬럼 추가는 FU
- **Circuit breaker 통합** (`crates/circuit-breaker/`) — 외부 sink 통합 시 같이
- **Metrics emit** (Prometheus counter `outbox_published_total` 등) — SP7 관측성 sub-project
- **`OutboxRepository::fetch_unpublished` LIMIT 정책 개선** — 현재 `occurred_at ASC LIMIT $1` 그대로

---

## 3. 아키텍처

```
┌────────────────────────────────────────────┐
│  services/outbox-publisher (binary)        │
│  ┌──────────────────────────────────────┐  │
│  │ main.rs:                             │  │
│  │  - PgOutboxRepository::new(pool)     │  │
│  │  - LoggingSink::new()                │  │
│  │  - tokio interval (every N ms)       │  │
│  │     for each tick:                   │  │
│  │       outbox_publisher::tick(...)    │  │
│  │  - graceful shutdown (SIGTERM)       │  │
│  └──────────────────────────────────────┘  │
└────────┬─────────────────────────┬─────────┘
         │ uses                    │ uses
         ▼                         ▼
┌──────────────────┐     ┌────────────────────┐
│ crates/db        │     │ crates/outbox-     │
│ PgOutboxReposi-  │     │ publisher (lib)    │
│ tory             │     │ - Sink trait       │
└──────────────────┘     │ - LoggingSink      │
                         │ - CountingSink     │
                         │ - tick(repo, sink) │
                         │ - PublisherError   │
                         └────────────────────┘
                                  │ via trait
                                  ▼
                         ┌────────────────────┐
                         │ outbox-event-domain│
                         │ - OutboxRepository │
                         │ - OutboxEvent      │
                         └────────────────────┘
```

`tick` 시퀀스:
```
[1] fetch_unpublished(limit) → Vec<OutboxEvent>
[2] for each event in batch (in order, sequential):
     a. sink.publish(&event).await
     b. on Ok  → repo.mark_published(&event.id, Utc::now()).await
     c. on Err → log error, continue (row 남음, 다음 tick 재시도)
[3] return TickReport { fetched, published, failed }
```

핵심 설계 결정:
- **순차 처리 (직렬)**: v1 은 같은 Aggregate 의 이벤트 순서 보존 (event 순서가 비즈니스 의미 있음). 동시성은 후속.
- **at-least-once 발행**: `mark_published` 성공 전 워커 죽으면 다음 tick 에 재발행 — sink 는 멱등성 보장 의무 (LoggingSink 는 자동 멱등)
- **장애 격리**: `mark_published` 실패 시 `failed += 1` 만 카운트 — 다음 tick 에 재시도 (이벤트 자체는 발행됐으나 mark 만 실패한 경우)

---

## 4. 컴포넌트 정의

### 4.1 `crates/outbox-publisher/src/sink.rs`

```rust
//! `Sink` trait — outbox event 의 발행 대상 추상화.
//!
//! v1 은 `LoggingSink` (tracing event 발행) 만 제공. 외부 시스템 통합
//! (Kafka/Webhook/SQS/NATS) 은 후속 sub-project 에서 같은 trait 구현체로 추가.

use std::sync::atomic::{AtomicU64, Ordering};
use async_trait::async_trait;
use outbox_event_domain::entity::OutboxEvent;
use thiserror::Error;
use tracing::info;

#[derive(Debug, Error)]
pub enum SinkError {
    #[error("sink publish failed: {0}")]
    Publish(String),
}

#[async_trait]
pub trait Sink: Send + Sync {
    async fn publish(&self, event: &OutboxEvent) -> Result<(), SinkError>;
}

/// 기본 sink — `tracing::info!` 로 구조화 event 발행.
/// 외부 LMS (Loki/Grafana) 가 이를 수집해 가시성 제공.
pub struct LoggingSink;

impl LoggingSink {
    #[must_use]
    pub const fn new() -> Self { Self }
}

impl Default for LoggingSink {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl Sink for LoggingSink {
    async fn publish(&self, event: &OutboxEvent) -> Result<(), SinkError> {
        info!(
            target: "outbox.publish",
            event_id = %event.id.as_str(),
            event_type = %event.event_type,
            aggregate_kind = %event.aggregate_kind,
            aggregate_id = %event.aggregate_id,
            correlation_id = %event.correlation_id,
            occurred_at = %event.occurred_at,
            "outbox event published"
        );
        Ok(())
    }
}

/// 테스트용 — publish 호출 횟수 카운트.
#[derive(Debug, Default)]
pub struct CountingSink {
    count: AtomicU64,
}

impl CountingSink {
    #[must_use]
    pub const fn new() -> Self { Self { count: AtomicU64::new(0) } }
    pub fn count(&self) -> u64 { self.count.load(Ordering::Relaxed) }
}

#[async_trait]
impl Sink for CountingSink {
    async fn publish(&self, _event: &OutboxEvent) -> Result<(), SinkError> {
        self.count.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}
```

### 4.2 `crates/outbox-publisher/src/publisher.rs`

```rust
//! `tick` — outbox publisher 의 한 사이클 단위.

use chrono::Utc;
use outbox_event_domain::repository::{OutboxRepository, RepoError};
use thiserror::Error;
use tracing::{instrument, warn};

use crate::sink::{Sink, SinkError};

#[derive(Debug, Error)]
pub enum PublisherError {
    #[error("repository error: {0}")]
    Repo(#[from] RepoError),
}

/// 한 tick 의 결과 — 메트릭 / 테스트 검증용.
#[derive(Debug, Default, Clone, Copy)]
pub struct TickReport {
    pub fetched: u32,
    pub published: u32,
    pub failed: u32,
}

#[instrument(skip(repo, sink), fields(limit))]
pub async fn tick(
    repo: &dyn OutboxRepository,
    sink: &dyn Sink,
    limit: u32,
) -> Result<TickReport, PublisherError> {
    let events = repo.fetch_unpublished(limit).await?;
    let fetched = u32::try_from(events.len()).unwrap_or(u32::MAX);
    let mut report = TickReport { fetched, published: 0, failed: 0 };

    for event in events {
        match sink.publish(&event).await {
            Ok(()) => match repo.mark_published(&event.id, Utc::now()).await {
                Ok(()) => report.published += 1,
                Err(e) => {
                    warn!(event_id = %event.id.as_str(), error = %e,
                          "mark_published failed — will retry next tick");
                    report.failed += 1;
                }
            },
            Err(e) => {
                warn!(event_id = %event.id.as_str(), error = %e,
                      "sink publish failed — will retry next tick");
                report.failed += 1;
            }
        }
    }
    Ok(report)
}
```

### 4.3 `services/outbox-publisher/src/main.rs`

```rust
//! 공짱 outbox publisher daemon — `outbox_event` row 를 폴링해 `Sink` 로 발행.
//!
//! 환경변수:
//! - `DATABASE_URL` (필수) — Postgres 접속 문자열
//! - `OUTBOX_POLL_INTERVAL_MS` (기본 1000) — tick 주기
//! - `OUTBOX_BATCH_SIZE` (기본 100) — tick 당 fetch limit
//!
//! 종료 신호 (SIGTERM / Ctrl+C) 받으면 진행 중 tick 완료 후 graceful shutdown.

#![forbid(unsafe_code)]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::env;
use std::sync::Arc;
use std::time::Duration;

use db::outbox::PgOutboxRepository;
use outbox_publisher::publisher::tick;
use outbox_publisher::sink::LoggingSink;
use outbox_event_domain::repository::OutboxRepository;
use sqlx::postgres::PgPoolOptions;
use tokio::signal;
use tokio::time;
use tracing::{error, info};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .json()
        .init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let interval_ms: u64 = env::var("OUTBOX_POLL_INTERVAL_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1000);
    let batch_size: u32 = env::var("OUTBOX_BATCH_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);

    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .expect("connect to Postgres");

    let repo: Arc<dyn OutboxRepository> = Arc::new(PgOutboxRepository::new(pool));
    let sink = LoggingSink::new();

    info!(interval_ms, batch_size, "outbox publisher starting");

    let mut interval = time::interval(Duration::from_millis(interval_ms));
    interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                match tick(repo.as_ref(), &sink, batch_size).await {
                    Ok(report) if report.fetched > 0 =>
                        info!(?report, "tick"),
                    Ok(_) => {}, // empty tick — silent
                    Err(e) => error!(error = %e, "tick failed"),
                }
            }
            _ = shutdown_signal() => {
                info!("shutdown signal received — stopping");
                break;
            }
        }
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("install ctrl-c handler");
    };
    #[cfg(unix)]
    let term = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();

    tokio::select! { _ = ctrl_c => {}, _ = term => {} }
}
```

### 4.4 통합 테스트 `crates/db/tests/outbox_publisher_integration.rs`

4 시나리오 — § 2 Scope 참조. 핵심 setup:
- `truncate_all` → outbox_event 비움
- 3 row INSERT 는 `PgOutboxRepository::save` 직접 호출 (또는 어떤 PgImpl `save` 호출 — admin_action 등으로 events 함께 INSERT)
- `tick(&repo, &sink, 100)` 호출
- `report.published == 3`, `published_at NOT NULL` count = 3, `sink.count() == 3`

`FailingSink` 는 테스트 파일 inline:
```rust
struct FailingSink;
#[async_trait::async_trait]
impl Sink for FailingSink {
    async fn publish(&self, _: &OutboxEvent) -> Result<(), SinkError> {
        Err(SinkError::Publish("test failure".into()))
    }
}
```

---

## 5. 데이터 흐름

### 5.1 정상 흐름

```
[1] handler/middleware → repo.save(&aggregate, ctx) (SP5-iv)
[2] tx 안: aggregate UPSERT + audit_log INSERT + outbox_event INSERT (published_at = NULL)
[3] tx commit
... (시간 경과)
[4] outbox-publisher 데몬의 다음 tick (interval_ms ms 후)
[5] PgOutboxRepository.fetch_unpublished(100) → 1 row
[6] LoggingSink.publish(&event) → tracing event "outbox.publish"
[7] PgOutboxRepository.mark_published(&id, now) → published_at = now
[8] 다음 tick 에서 fetch 안 잡힘 (정상)
```

### 5.2 sink 실패 흐름

```
[5] fetch_unpublished → 1 row
[6] sink.publish() → Err(...)
[7] mark_published 호출 X — row 그대로 (published_at NULL)
[8] tick report: failed = 1
[9] 다음 tick 에서 같은 row 다시 fetch → 재시도
```

### 5.3 mark_published 실패 흐름 (rare)

```
[5-7] 정상 → sink.publish OK
[7'] mark_published 실패 (DB 통신 일시 단절)
[8] 다음 tick 에서 같은 row 다시 fetch → sink.publish 다시 호출
   → ⚠️ at-least-once: sink 가 멱등성 보장 의무
```

---

## 6. 에러 정책

- `PublisherError::Repo(RepoError)` — fetch 자체 실패. tick 종료, 다음 tick 에 재시도
- `SinkError::Publish(msg)` — 개별 event 발행 실패. tick 계속 (다음 event 처리)
- `mark_published` 실패 — log + count, tick 계속

`tick` 함수 자체는 **fetch 실패**일 때만 `Err` 반환. 개별 event 실패는 `report.failed` 로만 누적 — 한 row 의 실패가 batch 전체를 막지 않음.

---

## 7. 가시성

모든 메서드 `#[tracing::instrument]`. PII 미노출 (event payload 는 `LoggingSink` 가 발행하지만 `aggregate_id` / `event_type` 은 식별자 수준).

`LoggingSink` 가 발행하는 tracing event:
- target: `"outbox.publish"`
- level: `INFO`
- fields: `event_id`, `event_type`, `aggregate_kind`, `aggregate_id`, `correlation_id`, `occurred_at`
- 메시지: `"outbox event published"`

운영 시 Loki/Grafana 가 `target=outbox.publish` 필터로 발행 흐름 모니터링.

---

## 8. CI 통합

- 신규 통합 테스트는 `walking-skeleton.yml` 의 `cargo test --features integration` 단계가 자동 실행
- `services/outbox-publisher` 바이너리 빌드는 `cargo build --workspace --all-features` 가 자동 빌드 (CI workflow `cargo-check` job 이 빌드 검증)
- 별도 CI 변경 없음

---

## 9. 검증 기준 (DoD)

1. `crates/outbox-publisher` 신규 — `Sink` trait + `LoggingSink` + `CountingSink` + `tick()` + `TickReport` + `PublisherError` (5-8 단위 테스트)
2. `services/outbox-publisher` 신규 — `main.rs` (binary, 환경변수 + interval loop + shutdown)
3. 워크스페이스 `Cargo.toml.members` 에 두 신규 crate 추가
4. 통합 테스트 4 신규 (`crates/db/tests/outbox_publisher_integration.rs`)
5. 누적 테스트 ≥ 1138 (SP5-iv 종료 ~1130 + 4 통합 + 5-8 단위)
6. 3 CI workflow 그린 (CI / db-migrations / walking-skeleton)
7. clippy `-D warnings` 통과 (`--all-features`)
8. tarpaulin ≥ 90% 유지
9. 모든 파일 ≤ 500 권장
10. README 업데이트 (services/outbox-publisher/README.md, 새 파일)

---

## 10. SSS 7 기둥 매핑

| 기둥 | SP4-i 적용 |
|---|---|
| 1 일관성 | Outbox 약속의 read side 가 비로소 채워짐 (write side = SP5-iii/iv). Sink trait 가 표준 인터페이스 — 후속 외부 sink 도 동일 채택 |
| 2 자동 강제 | tick 의 mark_published 자동 — 수동 호출 0. 통합 테스트가 published_at NOT NULL 검증, 누락 시 빨강 |
| 3 추적성 | 모든 event 가 tracing event 로 발행 (event_type, correlation_id, aggregate_id) — 한 request 의 audit_log → outbox_event → publisher tracing 까지 chain 추적 가능 |
| 4 안전성 | at-least-once 발행 + 멱등성 의무 명시. `unsafe` 0, `panic` 0. fetch 실패는 tick 단위 격리 — 개별 event 실패가 batch 막지 않음 |
| 5 가시성 | tick report 가 fetched/published/failed 메트릭 (운영 시 Prometheus exporter 추가 가능). `LoggingSink` 가 즉시 가시성 제공 — 운영 dashboard 즉시 활용 |
| 6 SSOT | Outbox event = DB row (published_at = SSOT for "발행됨"). `Sink` trait = "어디로 발행되는가" SSOT |
| 7 명확성 | `tick` 단일 함수 — 사이클 의미 명확. `LoggingSink` / `CountingSink` 이름이 의도 명시 |

---

## 11. Follow-up items

1. **분산 락** — 멀티 인스턴스 시 `SELECT FOR UPDATE SKIP LOCKED` 또는 advisory lock. 도커 클러스터 / k8s scale 후 도입
2. **외부 sink 구현체** — Kafka/Webhook/SQS/NATS — 도메인 sub-project 별 발행 정책 결정 후
3. **재시도 정책** — `outbox_event` 에 `attempt_count`, `last_attempted_at` 컬럼 추가 → max-retry 후 DLQ 테이블 이동
4. **Circuit breaker 통합** — 외부 sink 도입 시 `crates/circuit-breaker` 사용
5. **Prometheus metrics** — `outbox_published_total{aggregate_kind, sink}` counter — SP7 관측성과 묶음
6. **Backoff** — 빈 tick 연속 시 polling 간격 점진적 증가 (현재는 고정)
7. **`fetch_unpublished` LIMIT 정책** — 현재 `occurred_at ASC` — `correlation_id` 기준 grouping 정책 검토
