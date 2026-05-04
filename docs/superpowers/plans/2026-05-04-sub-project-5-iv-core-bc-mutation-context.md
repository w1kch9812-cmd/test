# Sub-project 5-iv: Core BC `MutationContext` 일원화 — 구현 계획

| | |
|---|---|
| 작성일 | 2026-05-04 |
| 상태 | Approved |
| 선행 spec | [`2026-05-04-sub-project-5-iv-core-bc-mutation-context-design.md`](../specs/2026-05-04-sub-project-5-iv-core-bc-mutation-context-design.md) |
| 추정 | 10 tasks (T1..T10), 각 task = 1 commit |

---

## 작업 흐름 원칙

1. **순서**: trait 변경 (T1~T3) → Pg 구현 변경 (T4~T6) → caller 변경 (T7) → 통합 테스트 (T8) → 검증 (T9) → SSOT 갱신/종료 (T10).
2. **Trait → impl → caller 순서 강제**: trait 변경 commit 단독은 임시로 컴파일 깨짐 — 다음 task 에서 impl 변경 후 다시 그린. integration test 가 빨강이 되는 구간은 T7 까지.
3. **각 task commit 메시지** = `feat|chore(sp5-iv-tN): <변경 한 줄>` + body 에 변경 파일 리스트.
4. **검증**: T9 가 마지막 종합 검증. T1~T8 commit 푸시 시 별도 cargo 호출 안 함 (시간 단축; T9 한 번에 모두 검증).

---

## T1 — `UserRepository::save` 시그니처 변경

**대상**: `crates/domain/core/user/src/repository.rs`

**변경**:
- `use shared_kernel::mutation::MutationContext;` 추가
- `save` 시그니처: `async fn save(&self, user: &User, ctx: MutationContext) -> Result<(), RepoError>`
- doc comment 갱신: `ctx` 의 actor/action/events 가 `audit_log` / `outbox_event` 로 자동 기록됨 명시

**기대 컴파일 상태**: `cargo check -p user-domain` 그린. workspace 컴파일은 빨강 (PgUserRepository 가 trait 미일치).

**commit**: `feat(sp5-iv-t1): UserRepository.save signature accepts MutationContext`

---

## T2 — `ListingRepository::save` 시그니처 변경

**대상**: `crates/domain/core/listing/src/repository.rs`

**변경**:
- `use shared_kernel::mutation::MutationContext;` 추가
- `save` 시그니처: `async fn save(&self, listing: &Listing, ctx: MutationContext) -> Result<(), RepoError>`

**commit**: `feat(sp5-iv-t2): ListingRepository.save signature accepts MutationContext`

---

## T3 — `ListingPhotoRepository::save` + `delete` 시그니처 변경

**대상**: `crates/domain/core/listing-photo/src/repository.rs`

**변경**:
- `use shared_kernel::mutation::MutationContext;` 추가
- `save(photo, ctx)` + `delete(id, ctx)` 둘 다 ctx 인자 추가
- doc comment 갱신: `delete` 도 audit 대상임을 명시

**commit**: `feat(sp5-iv-t3): ListingPhotoRepository.save+delete signature accepts MutationContext`

---

## T4 — `PgUserRepository.save` transactional 패턴

**대상**: `crates/db/src/user.rs`

**변경**:
- `use chrono::{DateTime, Utc};` 의 `Utc` 가 이미 import 되어 있음 — 변경 없음
- `use shared_kernel::id::{Id, UserMarker, AuditLogMarker, OutboxEventMarker};`
- `use shared_kernel::mutation::MutationContext;`
- `save` 메서드:
  1. `let mut tx = self.pool.begin().await.map_err(map_sqlx_err)?;`
  2. 기존 UPSERT SQL 그대로 — `.execute(&self.pool)` → `.execute(&mut *tx)` 로 교체
  3. `rows_affected() == 0` → `RepoError::Conflict` (early return; tx Drop 시 rollback)
  4. `audit_log` INSERT (resource_kind='user', actor_id, action, before_state=NULL, after_state=ctx.metadata, ip_address=ctx.client_ip::inet, user_agent, correlation_id, created_at=occurred_at)
  5. `for event in &ctx.events`: `outbox_event` INSERT (aggregate_kind='user', aggregate_id=user.id)
  6. `tx.commit()`
- `#[instrument]` 갱신: `ctx_action`, `correlation_id`, `events_count` 추가

**참조**: `crates/db/src/admin_action.rs:111-197` (T5 패턴 그대로 답습)

**기대 컴파일 상태**: workspace 빨강 — 콜러 (auth middleware + integration tests) 미반영. T7 에서 그린.

**commit**: `feat(sp5-iv-t4): PgUserRepository.save tx + audit_log + outbox`

---

## T5 — `PgListingRepository.save` transactional 패턴

**대상**: `crates/db/src/listing.rs`

**변경**:
- T4 와 동일 패턴
- Aggregate INSERT/UPSERT 부분만 21 필드 SQL 그대로
- `aggregate_kind = 'listing'`, `resource_kind = 'listing'`
- ctx import 추가, `#[instrument]` 갱신

**commit**: `feat(sp5-iv-t5): PgListingRepository.save tx + audit_log + outbox`

---

## T6 — `PgListingPhotoRepository.save` + `delete` transactional 패턴

**대상**: `crates/db/src/listing_photo.rs`

**변경**:
- `save`: T4/T5 와 동일 패턴 (12 필드 UPSERT + audit_log + outbox)
  - `aggregate_kind = 'listing_photo'`, `resource_kind = 'listing_photo'`
- `delete`: tx 안에서
  1. `DELETE FROM listing_photo WHERE id = $1`
  2. `rows_affected() == 0` → `RepoError::NotFound` (early return; rollback)
  3. `audit_log` INSERT (`action` 은 `ctx.action` 그대로 — caller 가 "delete" 등 명시; before_state/after_state 는 NULL — full diff capture 는 후속)
  4. `ctx.events` 가 있으면 outbox INSERT (대개 `PhotoDeletedEvent`)
  5. commit

**commit**: `feat(sp5-iv-t6): PgListingPhotoRepository.save+delete tx + audit_log + outbox`

---

## T7 — `services/api` auth middleware + 모든 integration test 콜러 갱신

**대상 (caller)**:
- `crates/auth/src/middleware.rs` — `resolve_or_create_user` 내 `repo.save(&user)` →
  ```rust
  let ctx = MutationContext::new_system_action(claims.sub.clone(), "first_sign_in")
      .with_metadata(serde_json::json!({"zitadel_sub": &claims.sub}));
  if let Err(save_err) = state.user_repo.save(&user, ctx).await { ... }
  ```
  - race 재시도 시점에는 또 한 번 ctx 가 필요한가? — 재시도는 find 만 하므로 추가 ctx 없음. `save` 두 번 부르지 않음. 변경 없음.
  - `crates/auth/Cargo.toml` 의 `shared-kernel` 의존성 확인 (이미 있음 — `Email`, `Id` 사용)
- `crates/auth` 의 doctest / 단위 테스트가 mock `UserRepository` 를 쓰는지 확인 → 시그니처 변경 반영 필요. 있으면 갱신.

**대상 (integration tests)** — 모두 `crates/db/tests/`:
- `user_integration.rs` (5 호출 지점, 모두 system action ctx 로 변경)
- `listing_integration.rs` (10 호출 지점)
- `listing_photo_integration.rs` (5 호출 지점, `delete` 1 호출도 ctx 추가)
- `error_map_integration.rs` (4 호출 지점)
- `bvq_integration.rs` (1 — owner seed user.save)
- `lrq_integration.rs` (3 — owner+listing+admin seed)
- `listing_report_integration.rs` (3 — 동일)
- `operations_meta_integration.rs` (1 — admin seed)
- `admin_action_integration.rs` (1 — admin seed via PgUserRepository.save)

**갱신 형태** (seed 헬퍼는 `MutationContext::new_system_action("test-seed", "create")` 으로 통일):
```rust
fn test_ctx() -> MutationContext {
    MutationContext::new_system_action("test-seed", "create")
}

repo.save(&user, test_ctx()).await.unwrap();
```

> seed 패턴은 `common.rs` 에 `pub fn test_ctx() -> MutationContext` 헬퍼 추가 검토. 현재는 각 파일 안 inline 으로도 OK — 단일 함수 호출이라 짧음.

**기대 컴파일 상태**: workspace 그린 (T1~T6 + T7 합치면 모두 정합).

**commit**: `feat(sp5-iv-t7): callers + integration test seeds use MutationContext`

---

## T8 — 신규 transactional 검증 통합 테스트

**대상**:
- `crates/db/tests/user_integration.rs` — 4 신규
  - `save_inserts_user_audit_log_in_one_tx`
  - `save_with_events_inserts_outbox_per_event`
  - `save_system_action_records_null_actor`
  - `save_with_metadata_writes_to_after_state`
- `crates/db/tests/listing_integration.rs` — 3 신규
- `crates/db/tests/listing_photo_integration.rs` — 3 신규 (`delete_audit_logs_with_action_delete` 포함)

**참조 패턴**: `crates/db/tests/admin_action_integration.rs` 4 테스트 그대로 답습 — `TestEvent` 헬퍼만 각 파일 안에 inline 정의 또는 `common.rs` 로 빼기 (선택). 짧으니 inline 유지 권장.

**검증 쿼리 형태**:
```rust
let audit_count: (i64,) = sqlx::query_as(
    "select count(*) from audit_log where resource_kind = 'user' and resource_id = $1"
).bind(user.id.as_str()).fetch_one(&pool).await.unwrap();
assert_eq!(audit_count.0, 1);
```

**commit**: `feat(sp5-iv-t8): integration tests verify audit_log + outbox rows for Core BC`

---

## T9 — 종합 검증

**명령** (순차):
```bash
cargo fmt --all -- --check
cargo check --workspace --all-features
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo test --workspace --all-features
# 통합 테스트는 Postgres 환경 필요 — 로컬 PG 띄운 상태에서:
DATABASE_URL=postgres://... cargo test -p db --features integration -- --test-threads=1
```

**합격 기준**:
- fmt clean
- check / clippy 모두 그린 (`-D warnings`)
- 단위 테스트 ≥1058 (SP5-iii 기준 1058 단위) — Core trait 변경은 단위 영향 0
- 통합 테스트 ≥72 (SP5-iii 기준 62 + 10 신규)
- 누적 ≥1130

**실패 시 원인 추적**:
- compile 빨강 → caller 미반영 (T7 빠진 곳 검색: `grep -rn "\.save(&[a-z_]*)\.await" crates/`)
- clippy `clippy::needless_pass_by_value` → `ctx: MutationContext` 가 by-value 라 `#[allow]` 필요할 수 있음. T4/T5/T6 코드에 추가 (`admin_action.rs` 와 동일 처리)
- integration test 빨강 → audit_log row 검증 SQL 의 resource_kind 오타 / aggregate_kind 오타 확인

**commit 없음** — 검증만. 실패 항목 발견 시 fix commit 추가 후 재실행.

---

## T10 — SSOT 갱신 + 종료

**대상**:
- `docs/superpowers/roadmap.md`:
  - "완료" 표에 SP5-iv 행 추가 (✅, "Core BC RDS Repository — `MutationContext` 일원화", "User/Listing/ListingPhoto save+delete 모두 tx + audit_log + outbox")
  - "다음 sub-project (사용자 결정)" 섹션에서 SP5-iv 항목 제거, SP5-ii / SP4 / SP6 만 남김
  - "Spec FU 누적" 섹션에 본 sub-project 가 닫은 빚 (= SP5-i / SP5-iii 가 완전 정합) 표기
- `memory/project_progress.md`:
  - 새 섹션 `### Sub-project 5-iv: Core BC MutationContext 일원화 (완료, T1-T10)` 추가
  - "다음 단계" 섹션 갱신: SP5-ii 또는 SP4 추천
  - 누적 카운트 갱신 (~1130 tests, 25 crate — crate 수 변동 없음)

**commit**: `docs(sp5-iv-t10): SP5-iv 종료 — Core BC MutationContext 일원화 완료`

---

## 변경 파일 요약 (예상)

| 분류 | 파일 | 변경 종류 |
|---|---|---|
| domain trait | `crates/domain/core/user/src/repository.rs` | `save` 시그니처 |
| domain trait | `crates/domain/core/listing/src/repository.rs` | `save` 시그니처 |
| domain trait | `crates/domain/core/listing-photo/src/repository.rs` | `save`+`delete` 시그니처 |
| Pg impl | `crates/db/src/user.rs` | `save` tx 패턴 |
| Pg impl | `crates/db/src/listing.rs` | `save` tx 패턴 |
| Pg impl | `crates/db/src/listing_photo.rs` | `save`+`delete` tx 패턴 |
| caller | `crates/auth/src/middleware.rs` | `MutationContext` 호출 |
| test (갱신) | `crates/db/tests/user_integration.rs` | seed + new tests |
| test (갱신) | `crates/db/tests/listing_integration.rs` | seed + new tests |
| test (갱신) | `crates/db/tests/listing_photo_integration.rs` | seed + new tests |
| test (갱신) | `crates/db/tests/error_map_integration.rs` | seed |
| test (갱신) | `crates/db/tests/bvq_integration.rs` | seed |
| test (갱신) | `crates/db/tests/lrq_integration.rs` | seed |
| test (갱신) | `crates/db/tests/listing_report_integration.rs` | seed |
| test (갱신) | `crates/db/tests/operations_meta_integration.rs` | seed |
| test (갱신) | `crates/db/tests/admin_action_integration.rs` | seed |
| docs | `docs/superpowers/roadmap.md` | SP5-iv 종료 표기 |
| memory | `memory/project_progress.md` | SP5-iv 섹션 |

총 ~17 파일.

---

## 위험 요소 (lessons-applicable)

- **clippy `needless_pass_by_value`**: `ctx: MutationContext` 가 by-value. `admin_action.rs` 와 동일하게 `#[allow(clippy::needless_pass_by_value)]` 필요 가능
- **caller 누락**: integration test 외 다른 곳에서 `repo.save(&user)` 패턴 호출이 더 있을 수 있음 → T7 시작 전 `grep -rn "\.save(&[a-z]" crates/ services/` 풀스캔 후 파악
- **`crates/auth` 의 mock UserRepository**: 단위 테스트용 mock 존재 시 시그니처 갱신 필요
- **DB-less 단위 테스트의 시그니처 자동 매칭**: `async-trait` 매크로라 mismatch 가 컴파일 에러로 명확히 잡힘
- **integration test 격리**: `truncate_all` 이 `audit_log` 도 truncate 하므로 새 테스트의 `count(*)` 검증 격리 보장됨
- **MutationContext clone 비용**: 도메인 이벤트 `Arc<dyn DomainEvent>` 는 cheap clone. 시그니처가 `ctx: MutationContext` (by value) 이므로 caller 가 한 번만 만들고 넘김 — 비용 무시
