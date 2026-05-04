# Sub-project 4-i: Outbox Publisher Worker — 구현 계획

| | |
|---|---|
| 작성일 | 2026-05-04 |
| 상태 | Approved |
| 선행 spec | [`2026-05-04-sub-project-4-i-outbox-publisher-design.md`](../specs/2026-05-04-sub-project-4-i-outbox-publisher-design.md) |
| 추정 | 7 task (T1..T7), 1일 |

---

## 작업 흐름 원칙

1. **순서**: 라이브러리 (T1) → 바이너리 (T2) → 워크스페이스 등록 (T3) → 통합 테스트 (T4) → 종합 검증 + 푸시 (T5-T6) → SSOT (T7)
2. **각 task commit 단위** 로 분리 — `feat|chore(sp4-i-tN): <한 줄>`
3. **SP5-iv 교훈 적용**: 각 task commit *전* `cargo check -p <crate>` (또는 `cargo check --workspace`) 실행 — 로컬 cargo 1.88.0 설치됨, MSVC 링커는 없지만 *typecheck-only* 명령은 일부 가능. 링커 필요 단계는 push 후 CI 가 진실
   - `cargo check`: rustc 의 build script 가 proc-macro 컴파일 시 link 필요 → 링커 미설치 환경에선 사실상 작동 안 함
   - **현실적 대안**: 각 task 후 `cargo fmt --all -- --check` (link 불필요) 만 로컬 수행. typecheck/clippy/test 는 push 후 CI 가 검증
4. **구현 순서 안에서 깨지는 컴파일 상태**:
   - T1 commit: `crates/outbox-publisher` 자체는 그린, 다른 crate 영향 0
   - T2 commit: `services/outbox-publisher` 자체는 그린 — `db` + `outbox-publisher` lib 의존
   - T3 commit: workspace.members 갱신 — workspace check 그린
   - T4 commit: 통합 테스트 추가 — 빌드 정합 (db 의 dev-dep 에 `outbox-publisher` 추가 필요할 수 있음, 아래 § T4 참조)
   - 즉, **T1~T4 각 단계가 자체 그린** — broken state 없음

---

## T1 — `crates/outbox-publisher` 라이브러리 crate

**대상**: 신규 crate

```
crates/outbox-publisher/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── sink.rs       (Sink trait + LoggingSink + CountingSink + SinkError)
    └── publisher.rs  (tick + TickReport + PublisherError)
```

**Cargo.toml**:
```toml
[package]
name = "outbox-publisher"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license = "Apache-2.0"
description = "공짱 outbox 이벤트 발행기 — Sink trait + tick 함수"

[dependencies]
shared-kernel = { path = "../domain/core/shared-kernel", version = "0.1.0" }
outbox-event-domain = { path = "../domain/audit/outbox-event", version = "0.1.0" }
async-trait = { workspace = true }
chrono = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }

[dev-dependencies]
tokio = { workspace = true, features = ["macros", "rt-multi-thread"] }
serde_json = { workspace = true }

[lints]
workspace = true
```

**src/lib.rs**:
```rust
//! 공짱 outbox publisher — `OutboxRepository` 를 폴링해 `Sink` 로 발행.
//!
//! 사용:
//! - 라이브러리 사용자: [`tick`] 호출, [`Sink`] 구현체 제공
//! - daemon: `services/outbox-publisher` 가 [`tick`] 을 interval loop 안에서 호출

pub mod publisher;
pub mod sink;

pub use publisher::{tick, PublisherError, TickReport};
pub use sink::{CountingSink, LoggingSink, Sink, SinkError};
```

**src/sink.rs** + **src/publisher.rs**: spec § 4.1, § 4.2 그대로 (조금 정리해서).

**단위 테스트** (in `sink.rs` 와 `publisher.rs` 의 `#[cfg(test)] mod tests`):
- `LoggingSink::publish` 호출 시 `Ok(())` 반환 (panic 없음 — `#[tokio::test]`)
- `CountingSink::count` 가 publish 횟수와 일치 (3 회 호출 → 3)
- `SinkError::Publish` Display 형식 검증
- `PublisherError::Repo` Display 형식 검증
- `TickReport::default()` 가 모두 0
- `tick` 의 happy path 는 통합 테스트에서 (단위에서는 mock OutboxRepository 만들기엔 비용 큼 — skip 하고 통합 테스트로 대체)

**검증**:
- `cargo fmt -p outbox-publisher -- --check` (로컬 가능)
- 로컬 typecheck/clippy 는 링커 없이 안 됨 → CI 가 진실

**commit**: `feat(sp4-i-t1): outbox-publisher library — Sink trait + tick + LoggingSink + CountingSink`

---

## T2 — `services/outbox-publisher` binary crate

**대상**: 신규 binary crate

```
services/outbox-publisher/
├── Cargo.toml
├── README.md
└── src/
    └── main.rs
```

**Cargo.toml**:
```toml
[package]
name = "outbox-publisher-service"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license = "Apache-2.0"
description = "공짱 outbox publisher daemon (services)"

[[bin]]
name = "outbox-publisher"
path = "src/main.rs"

[dependencies]
db = { path = "../../crates/db", version = "0.1.0" }
outbox-event-domain = { path = "../../crates/domain/audit/outbox-event", version = "0.1.0" }
outbox-publisher = { path = "../../crates/outbox-publisher", version = "0.1.0" }
sqlx = { workspace = true }
tokio = { workspace = true, features = ["macros", "rt-multi-thread", "signal", "time"] }
tracing = { workspace = true }
tracing-subscriber = { workspace = true, features = ["env-filter", "json"] }

[lints]
workspace = true
```

**main.rs**: spec § 4.3 그대로.

**README.md** (짧게):
```markdown
# services/outbox-publisher

공짱 outbox 이벤트 발행 daemon.

## 환경변수
- `DATABASE_URL` (필수)
- `OUTBOX_POLL_INTERVAL_MS` (기본 1000)
- `OUTBOX_BATCH_SIZE` (기본 100)
- `RUST_LOG` (기본 `info`)

## 기동
```bash
cargo run -p outbox-publisher-service
```

## 종료
SIGTERM / Ctrl+C 로 graceful shutdown.

## 후속
SP4-i 미포함 — 분산 락, 외부 sink, 재시도 정책, metrics 는 후속 sub-project.
```
```

**검증**: 로컬 `cargo fmt`, push 후 CI build.

**commit**: `feat(sp4-i-t2): outbox-publisher service — binary daemon with interval loop`

---

## T3 — 워크스페이스 Cargo.toml 갱신

**대상**: `Cargo.toml` (root)

```toml
members = [
    # ... 기존
    "crates/outbox-publisher",
    "services/outbox-publisher",
]
```

추가 변경 없음. `tracing-subscriber` 는 이미 workspace deps 에 있음.

**검증**: 로컬 `cargo metadata` 또는 push 후 CI workspace 빌드.

**commit**: `chore(sp4-i-t3): add outbox-publisher crates to workspace`

---

## T4 — 통합 테스트 `crates/db/tests/outbox_publisher_integration.rs`

**대상**: 신규 통합 테스트 파일

**Cargo.toml 변경 (`crates/db`)**:
```toml
[dev-dependencies]
tokio = { ... 기존 ... }
outbox-publisher = { path = "../outbox-publisher", version = "0.1.0" }   # 신규
```

**테스트 파일 구조**:
```rust
//! `outbox-publisher::tick` 통합 테스트 (SP4-i).
//!
//! 4 시나리오: spec § 2 참조.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
#![cfg(feature = "integration")]

mod common;

use async_trait::async_trait;
use chrono::Utc;
use db::admin_action::PgAdminActionRepository;
use db::outbox::PgOutboxRepository;
use db::user::PgUserRepository;
// 또는 PgAdminActionRepository 로 outbox row 생성 — admin_action.insert 가
// ctx.events 에 따라 outbox row INSERT 수행
use outbox_event_domain::entity::OutboxEvent;
use outbox_event_domain::repository::OutboxRepository;
use outbox_publisher::{tick, CountingSink, Sink, SinkError};
use shared_kernel::id::{Id, OutboxEventMarker};
use shared_kernel::mutation::MutationContext;

use common::{setup_test_pool, truncate_all};

// 4 tests — see spec § 2
```

**시드 전략**: outbox_event row 만들기 위해 `PgOutboxRepository::save` 직접 호출 (간단). 또는 `PgAdminActionRepository::insert` 를 events 함께 호출 (실제 흐름과 동일). **선택: 직접 INSERT** — 테스트 격리 + 단순성.

**4 tests**:
1. `tick_publishes_unpublished_rows`:
   - 3 row INSERT (PgOutboxRepository.save 직접)
   - `tick(&repo, &sink, 100)` → `report.published == 3, fetched == 3, failed == 0`
   - `published_at IS NOT NULL` row count = 3
   - `sink.count() == 3`
2. `tick_skips_already_published`:
   - 1 row INSERT + 즉시 `mark_published`
   - 1 row INSERT (unpublished)
   - `tick` → published == 1
3. `tick_returns_zero_when_no_rows`:
   - 빈 테이블에서 `tick` → published == 0, fetched == 0
4. `tick_failure_leaves_row_unpublished`:
   - 1 row INSERT
   - `FailingSink` (file inline) 사용
   - `tick` → failed == 1, published == 0
   - `published_at IS NULL` 유지

**검증**: push 후 CI walking-skeleton 의 integration test 단계.

**commit**: `feat(sp4-i-t4): integration tests for outbox publisher tick`

---

## T5 — 종합 검증

**로컬 가능**:
- `cargo fmt --all -- --check`
- (시도) `cargo metadata --format-version 1 > $null` — workspace 일관성 체크 (link 불필요한 메타 명령)

**push 후 CI 가 진실** (T6 에서 처리).

**T9 거짓 완료 재발 방지** (SP5-iv 교훈 #1): 본 task 는 *로컬에서 가능한 것만* 실행. 가능 명령:
- `cargo fmt --all -- --check` ✓
- `cargo metadata` ✓
- `cargo check` ✗ (proc-macro link 필요)
- `cargo clippy` ✗
- `cargo test` ✗

**commit 없음** — 검증 단계.

---

## T6 — push + CI 모니터링

**명령**:
```bash
git push origin main
# 4-6분 후 GitHub API 폴링
```

**검증**: 3 workflow 그린 (CI / db-migrations / walking-skeleton).

**실패 시**:
- compile 빨강 → diff 분석 → fix commit + 재푸시
- clippy 빨강 → fix 후 재푸시
- integration test 빨강 → 로그 분석 (특히 outbox_publisher_integration.rs 의 4 시나리오)

**commit (필요 시)**: `fix(sp4-i): <issue>`

---

## T7 — SSOT 갱신

**대상**:
- `docs/superpowers/roadmap.md`:
  - 완료 표에 SP4-i 행 추가
  - "다음 sub-project (사용자 결정)" 에서 SP4-i 제거, SP5-ii / SP4-ii 만 남김
  - "다음 단계" 갱신
- `memory/project_progress.md`:
  - 새 섹션 `### Sub-project 4-i: Outbox Publisher (완료, T1-T7)`
  - 누적 카운트 갱신 (~1138 tests, 26-27 crate)
- `MEMORY.md` 인덱스 한 줄 갱신

**commit**: `docs(sp4-i-t7): SP4-i 종료 — Outbox publisher worker`

---

## 변경 파일 요약 (예상)

| 분류 | 파일 | 변경 |
|---|---|---|
| lib (신규) | `crates/outbox-publisher/Cargo.toml` | 신규 |
| lib (신규) | `crates/outbox-publisher/src/lib.rs` | 신규 |
| lib (신규) | `crates/outbox-publisher/src/sink.rs` | 신규 |
| lib (신규) | `crates/outbox-publisher/src/publisher.rs` | 신규 |
| bin (신규) | `services/outbox-publisher/Cargo.toml` | 신규 |
| bin (신규) | `services/outbox-publisher/src/main.rs` | 신규 |
| bin (신규) | `services/outbox-publisher/README.md` | 신규 |
| workspace | `Cargo.toml` | members 추가 |
| db dep | `crates/db/Cargo.toml` | dev-dep `outbox-publisher` |
| test (신규) | `crates/db/tests/outbox_publisher_integration.rs` | 신규 |
| docs | `docs/superpowers/roadmap.md` | SP4-i 종료 표기 |
| memory | `memory/project_progress.md` | 새 섹션 |
| memory | `MEMORY.md` | 인덱스 한 줄 |

총 ~13 파일.

---

## 위험 요소

- **integration test 의 outbox row 시드**: `PgOutboxRepository::save` 가 caller tx 밖에서 단순 INSERT 한다 (이미 SP5-iii). 테스트 시드 시 그대로 사용 가능.
- **`mark_published` 의 `where published_at is null` 조건**: 이미 SP5-iii 구현 — 이중 mark 방지. SP4-i 가 활용.
- **`fetch_unpublished` LIMIT**: 100 batch — 통합 테스트는 3 row 만 다루므로 무관.
- **`tokio::signal::unix`**: Windows 빌드 시 `#[cfg(unix)]` 분기 필요 (이미 spec § 4.3 에 반영).
- **CI runner 가 Linux** — `signal::unix` 정상. `#[cfg(unix)]` 가드 덕분에 Windows 로컬 빌드도 OK (대안 분기는 `pending::<()>()`).
- **workspace `tracing-subscriber` features**: `env-filter`, `json` 둘 다 필요. workspace deps 에 추가 필요.
- **`outbox-publisher` 와 `outbox-publisher-service` 명명**: 라이브러리 / binary crate 분리 — 라이브러리 이름이 짧아야 import 깨끗. binary 패키지명에 `-service` 접미.
