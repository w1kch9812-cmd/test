# Sub-project 5-iv: Core BC RDS Repository — `MutationContext` 일원화 (Spec)

| | |
|---|---|
| 작성일 | 2026-05-04 |
| 상태 | Approved |
| 선행 | SP5-i (Core BC RDS Repository), SP5-iii (Audit + Pipeline + Operations + 트랜잭션 Outbox 패턴) |
| 후속 | SP5-ii (Insights BC RDS), SP4 (외부 API + R2 Reader + Outbox publisher) |
| 관련 ADR | — |

---

## 1. 개요

SP5-iii 가 6 개 BC (Audit / Pipeline / Operations 5) 의 `Pg*Repository.save()` 에 트랜잭션 audit/outbox 패턴을 도입했지만, **SP5-i 의 Core BC 3 개** (`PgUserRepository`, `PgListingRepository`, `PgListingPhotoRepository`) 는 옛날 `save(aggregate)` 시그니처 그대로 남아 있어요.

이 상태는 SSS 7 기둥 중 3 개 기둥을 **부분 위반** 해요:

- **1 일관성** — 같은 일(=Aggregate 저장)이 6 BC 는 transactional, 3 BC 는 그렇지 않음. 신규 기여자가 어느 패턴 따라야 할지 추측해야 함.
- **3 추적성** — SP5-iii 가 약속한 *모든 mutation audit_log 자동 기록* 이 9 BC 중 6 BC 만 적용됨. User 가입 / Listing 등록 / ListingPhoto 업로드는 audit 흔적 0.
- **6 SSOT (단일 출처)** — `Repository` trait 시그니처가 이원화 (`save(agg)` vs `save(agg, ctx)`).

본 sub-project 는 위 빚을 닫는 게 목표예요. 새 기능 0, 패턴 일원화만.

---

## 2. 범위 (Scope)

### 포함

- **3 도메인 trait 시그니처 변경** — `UserRepository::save` / `ListingRepository::save` / `ListingPhotoRepository::save` 가 `ctx: MutationContext` 인자 추가
- **3 PgImpl 구현 변경** — SP5-iii 의 transactional 패턴 (Aggregate UPSERT + `audit_log` INSERT + `outbox_event` INSERT, 모두 같은 tx) 적용
  - `PgUserRepository.save` — `resource_kind = 'user'`
  - `PgListingRepository.save` — `resource_kind = 'listing'`
  - `PgListingPhotoRepository.save` — `resource_kind = 'listing_photo'`
- **`PgListingPhotoRepository.delete`** — hard delete 도 audit 대상. `delete(&id, ctx)` 로 시그니처 변경 (관리/테스트 흐름 동일하게 추적성 보장)
- **`services/api` auth 미들웨어** — first-sign-in 자동 생성 시 `MutationContext::new_system_action(...)` 구성 후 `repo.save(user, ctx)` 호출
- **모든 통합 테스트 갱신** — `crates/db/tests/*_integration.rs` 의 seed 헬퍼 / 직접 `save` 호출이 `MutationContext` 받도록 갱신 (총 7 파일: `user_integration` / `listing_integration` / `listing_photo_integration` / `error_map_integration` / `bvq_integration` / `lrq_integration` / `listing_report_integration` / `operations_meta_integration`)
- **신규 통합 테스트 ~10** — 3 Pg repo 각각:
  1. `save_inserts_aggregate_audit_outbox_in_one_tx` (성공 → 3 row 모두 존재)
  2. `save_with_events_inserts_outbox_per_event` (events 2개 → outbox 2 row)
  3. `save_system_action_records_null_actor` (시스템 mutation, audit_log.actor_id IS NULL)
  4. (User 추가) `save_with_metadata_writes_to_after_state` 1 개

### 미포함 (후속)

- **AuthMiddleware HTTP 컨텍스트 자동 주입** — `correlation_id`(X-Request-ID) / `client_ip` / `user_agent` 자동 추출은 SP7 관측성과 묶음. SP5-iv 는 first-sign-in 한정 minimal `new_system_action` 만.
- **`Repository.save` `expected_version` 명시 인자** — FU 15 후보. 별도 ADR.
- **AuditLog full diff (`before_state` JSON)** — current SP5-iv 는 `before_state = NULL` (SP5-iii 와 동일 정책)
- **Outbox publisher worker** — SP4 또는 별도 sub-project
- **HTTP 응답 매핑 (`RepoError → IntoResponse`)** — 별도

---

## 3. 아키텍처 (변화 요약)

### Before (SP5-i)

```
handler → repo.save(&user)
  → Pg: INSERT/UPSERT user
  (audit_log 누락, outbox 누락)
```

### After (SP5-iv)

```
handler / middleware
  → ctx = MutationContext::new_user_action(actor, request_id, "update_profile")
         .with_events(vec![Arc::new(UserUpdatedEvent {...})])
  → repo.save(&user, ctx)
  → Pg (tx):
      1. UPSERT user (OCC)
      2. INSERT audit_log (resource_kind='user', resource_id=user.id, ...)
      3. INSERT outbox_event for each ctx.events
      4. tx.commit() — 실패 시 자동 rollback
```

### auth middleware first-sign-in (`services/api`)

```
[1] resolve_or_create_user — find_by_zitadel_sub miss
[2] User::try_new(...) 도메인 생성
[3] ctx = MutationContext::new_system_action(claims.sub.clone(), "first_sign_in")
        .with_metadata(json!({"zitadel_sub": claims.sub}))
[4] repo.save(&user, ctx) → audit_log row 1개 ('user' resource, action='first_sign_in', actor=NULL)
[5] race 시 동일 동작 (find 재시도)
```

> first-sign-in 의 `correlation_id` 는 zitadel sub. 후속 (SP7) HTTP middleware 로 X-Request-ID 자동 주입 시 우선순위 변경.

---

## 4. 컴포넌트 정의

### 4.1 `crates/domain/core/user/src/repository.rs` 변경

```rust
use shared_kernel::mutation::MutationContext;
// ...
#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_id(&self, id: &Id<UserMarker>) -> Result<Option<User>, RepoError>;
    async fn find_by_zitadel_sub(&self, sub: &str) -> Result<Option<User>, RepoError>;
    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, RepoError>;

    /// 저장 (insert or update). `ctx` 의 actor/action/events 가 같은 tx 안에서
    /// `audit_log` / `outbox_event` 로 자동 기록돼요.
    async fn save(&self, user: &User, ctx: MutationContext) -> Result<(), RepoError>;
}
```

### 4.2 `crates/domain/core/listing/src/repository.rs` 변경

`save(&self, listing: &Listing, ctx: MutationContext) -> Result<(), RepoError>`. find 메서드 변경 없음.

### 4.3 `crates/domain/core/listing-photo/src/repository.rs` 변경

```rust
async fn save(&self, photo: &ListingPhoto, ctx: MutationContext) -> Result<(), RepoError>;
async fn delete(&self, id: &Id<ListingPhotoMarker>, ctx: MutationContext) -> Result<(), RepoError>;
```

> `delete` 도 audit 대상으로 포함. spec 문서 위로 운영자가 사진을 hard delete 하면 그 흔적이 `audit_log` 에 남아야 함 (`action='delete'`, `resource_kind='listing_photo'`).

### 4.4 PgImpl 패턴 (3 repo 동일)

`PgAdminActionRepository.insert` 와 동일 구조 (SP5-iii T5):

```rust
async fn save(&self, user: &User, ctx: MutationContext) -> Result<(), RepoError> {
    let mut tx = self.pool.begin().await.map_err(map_sqlx_err)?;

    // 1. UPSERT user (기존 SP5-i 쿼리 그대로 — &mut *tx 로만 변경)
    let result = sqlx::query(r#"insert into "user" (...) values (...) on conflict ..."#)
        // ... 18 binds ...
        .execute(&mut *tx).await.map_err(map_sqlx_err)?;
    if result.rows_affected() == 0 {
        return Err(RepoError::Conflict);
    }

    // 2. audit_log INSERT (같은 tx)
    let audit_id = Id::<AuditLogMarker>::new();
    let occurred_at = ctx.occurred_at.unwrap_or_else(Utc::now);
    sqlx::query(r#"
        insert into audit_log (id, actor_id, action, resource_kind, resource_id,
            before_state, after_state, ip_address, user_agent,
            correlation_id, created_at)
        values ($1, $2, $3, 'user', $4, NULL, $5, $6::inet, $7, $8, $9)
    "#)
    .bind(audit_id.as_str())
    .bind(ctx.actor_id.as_ref().map(Id::as_str))
    .bind(&ctx.action)
    .bind(user.id.as_str())
    .bind(&ctx.metadata)
    .bind(ctx.client_ip.as_deref())
    .bind(ctx.user_agent.as_deref())
    .bind(&ctx.correlation_id)
    .bind(occurred_at)
    .execute(&mut *tx).await.map_err(map_sqlx_err)?;

    // 3. outbox_event INSERT for each ctx.events (같은 tx)
    for event in &ctx.events {
        let outbox_id = Id::<OutboxEventMarker>::new();
        sqlx::query(r#"
            insert into outbox_event (id, aggregate_kind, aggregate_id, event_type,
                payload, correlation_id, created_at, published_at)
            values ($1, 'user', $2, $3, $4, $5, $6, NULL)
        "#)
        .bind(outbox_id.as_str())
        .bind(user.id.as_str())
        .bind(event.event_type())
        .bind(event.payload())
        .bind(&ctx.correlation_id)
        .bind(event.occurred_at())
        .execute(&mut *tx).await.map_err(map_sqlx_err)?;
    }

    tx.commit().await.map_err(map_sqlx_err)?;
    Ok(())
}
```

`PgListingRepository.save` / `PgListingPhotoRepository.save` 도 동일 구조 — Aggregate INSERT/UPSERT 부분만 21 필드 / 12 필드 SQL.

### 4.5 `PgListingPhotoRepository.delete` 트랜잭션화

```rust
async fn delete(&self, id: &Id<ListingPhotoMarker>, ctx: MutationContext) -> Result<(), RepoError> {
    let mut tx = self.pool.begin().await.map_err(map_sqlx_err)?;

    // 1. DELETE listing_photo
    let result = sqlx::query("delete from listing_photo where id = $1")
        .bind(id.as_str())
        .execute(&mut *tx).await.map_err(map_sqlx_err)?;
    if result.rows_affected() == 0 {
        return Err(RepoError::NotFound);
    }

    // 2. audit_log INSERT (같은 tx, action='delete' 권장)
    // ... (위 패턴과 동일)

    // 3. ctx.events outbox INSERT (있으면)
    // ...

    tx.commit().await.map_err(map_sqlx_err)?;
    Ok(())
}
```

### 4.6 `services/api` auth middleware 변경

`crates/auth/src/middleware.rs` 의 `resolve_or_create_user` 함수에서 `repo.save(&user)` 호출 부 변경:

```rust
let ctx = MutationContext::new_system_action(claims.sub.clone(), "first_sign_in")
    .with_metadata(serde_json::json!({"zitadel_sub": &claims.sub}));
if let Err(save_err) = state.user_repo.save(&user, ctx).await {
    // ... 기존 race 처리 동일 ...
}
```

> `correlation_id` = `claims.sub` (request 단위가 아닌 user-creation 단위 추적). HTTP request_id 자동 주입은 SP7 후속.

---

## 5. 데이터 흐름 (시퀀스)

### 5.1 first-sign-in (시스템 액션)

```
[1] HTTP 요청 + Bearer JWT — auth middleware 진입
[2] verify(jwt) 통과
[3] find_by_zitadel_sub miss
[4] User::try_new(...) 생성
[5] ctx = MutationContext::new_system_action(sub, "first_sign_in")
[6] PgUserRepository.save(&user, ctx) — tx 안:
    a. UPSERT user
    b. INSERT audit_log (actor=NULL, action='first_sign_in', resource_kind='user',
       resource_id=user.id, after_state={"zitadel_sub": ...})
    c. (events 없음 → outbox 0)
    d. commit
[7] handler 진행
```

### 5.2 사용자 자기 정보 수정 (사용자 액션, 후속 sub-project 에서 등장)

```
[1] PATCH /users/me
[2] auth middleware 통과 → AuthenticatedUser
[3] handler:
    user.update_profile(...)
    ctx = MutationContext::new_user_action(auth.user.id, request_id, "update_profile")
        .with_events(vec![Arc::new(UserProfileUpdated{...})])
        .with_client_info(req_ip, req_ua)
    repo.save(&user, ctx)
[4] tx 안: UPSERT + audit_log + outbox
```

### 5.3 listing 등록 (사용자 액션)

```
[1] POST /listings
[2] handler:
    listing = Listing::try_new_draft(...)
    ctx = MutationContext::new_user_action(auth.user.id, request_id, "create_listing")
        .with_events(vec![Arc::new(ListingDraftCreated{...})])
    listing_repo.save(&listing, ctx)
[3] tx 안: INSERT listing + audit_log + outbox
```

---

## 6. 에러 매핑 정책

### 6.1 RepoError variants — 변경 없음

`UserRepository::RepoError` / `ListingRepository::RepoError` / `ListingPhotoRepository::RepoError` 모두 기존 `NotFound` / `Conflict` / `Database(String)` 유지. 새 variant 도입 안 함.

### 6.2 트랜잭션 실패

- `tx.commit()` 실패 → `Database(msg)`
- 중간 INSERT 실패 → `?` 자동 rollback (sqlx tx Drop)
- audit_log INSERT 실패 = 전체 mutation 실패. 사용자 변경도 안 들어감 — 이게 SSS 안전성 핵심
- OCC 충돌 시 `rows_affected() == 0` 체크 후 `RepoError::Conflict` 반환 (commit 도달 전 explicit return → tx Drop → rollback)

### 6.3 SQL injection 방어

모든 사용자 입력 `bind()` 통해 parameterized query. `ctx.action` / `metadata` / `correlation_id` 등 모두 bind.

---

## 7. 가시성 — `tracing::instrument`

3 repo `save` 메서드 모두 `#[instrument]` 갱신:

```rust
#[instrument(skip(self, user, ctx), fields(
    user_id = %user.id.as_str(),
    version = user.version,
    ctx_action = %ctx.action,
    correlation_id = %ctx.correlation_id,
    events_count = ctx.events.len(),
))]
```

PII 미노출: `metadata` / `client_ip` / `user_agent` / `events` 의 payload 는 모두 `skip` (운영 시 audit_log 직접 쿼리).

---

## 8. 통합 테스트 전략

### 8.1 신규 통합 테스트 (3 repo × 3 = 9 + User 메타 1 = 10)

**`user_integration.rs` (4 개 추가)**:
1. `save_inserts_user_audit_log_in_one_tx` — `audit_log` row 1개 검증 (resource_kind='user')
2. `save_with_events_inserts_outbox_per_event` — events 2개 → outbox 2 row
3. `save_system_action_records_null_actor` — actor_id NULL
4. `save_with_metadata_writes_to_after_state` — metadata → audit_log.after_state

**`listing_integration.rs` (3 개 추가)**:
1. `save_inserts_listing_audit_log_in_one_tx`
2. `save_with_events_inserts_outbox_per_event`
3. `save_system_action_records_null_actor`

**`listing_photo_integration.rs` (3 개 추가)**:
1. `save_inserts_photo_audit_log_in_one_tx`
2. `save_with_events_inserts_outbox_per_event`
3. `delete_audit_logs_with_action_delete` — hard delete 후 audit_log row 검증

### 8.2 기존 통합 테스트 갱신

기존 모든 `repo.save(&user)` / `.save(&listing)` / `.save(&photo)` 콜이 `repo.save(&user, MutationContext::new_system_action("test", "create"))` 형태로 변경. 그 외 검증 로직은 동일.

### 8.3 단위 테스트

`MutationContext` 자체는 SP5-iii 에서 6 unit test 완비 — 추가 없음. 도메인 trait 변경은 시그니처만이므로 단위 테스트 변경 없음 (mock 구현체가 본 sub-project 범위 내에 없음).

---

## 9. CI 통합

`walking-skeleton.yml` 변경 없음 — `cargo test --features integration` 단계가 새 통합 테스트 자동 실행.
`db-migrations.yml` 변경 없음 — 스키마 변화 없음.

---

## 10. 검증 기준 (DoD)

본 sub-project 종료 조건:

1. `crates/domain/core/{user,listing,listing-photo}/src/repository.rs` 의 `save` (+ photo `delete`) 시그니처에 `ctx: MutationContext` 추가
2. `crates/domain/core/{user,listing,listing-photo}/Cargo.toml` 에 `shared-kernel` dep 확인 (이미 있어야 함 — 변경 없을 수 있음)
3. `crates/db/src/{user,listing,listing_photo}.rs` 의 `save` (+ photo `delete`) 가 transactional 패턴 (Aggregate + audit_log + outbox 모두 같은 tx)
4. `services/api` `crates/auth/src/middleware.rs` first-sign-in 이 `MutationContext::new_system_action(claims.sub, "first_sign_in")` 사용
5. 기존 통합 테스트 7 파일 모두 새 시그니처로 컴파일 통과
6. 신규 통합 테스트 ≥10 추가 → audit_log + outbox row 검증
7. 3 CI 워크플로우 모두 그린
8. 누적 테스트 ≥1130 (SP5-iii 종료 ~1120 + 통합 10 + 기존 시그니처 변경분)
9. clippy `-D warnings` 통과 (`--all-features` 포함)
10. tarpaulin ≥90% 유지
11. 모든 파일 ≤500 권장 / ≤1500 강제
12. `docs/superpowers/roadmap.md` 갱신 (SP5-iv 종료 + SP5 시리즈 닫기 표기)
13. `memory/project_progress.md` 갱신

---

## 11. SSS 7 기둥 매핑 (정직 평가)

| 기둥 | SP5-iv 적용 |
|---|---|
| 1 일관성 | **9 BC 모두** 동일 transactional save 패턴. `Repository::save(agg, ctx)` 단일 시그니처. SP5-iii 가 도입한 패턴이 Core BC 까지 도달 — 시리즈 닫음 |
| 2 자동 강제 | `cargo check` 가 시그니처 변경 미반영 caller 모두 컴파일 실패로 차단. CI integration test 가 audit_log row 미존재 시 빨강 |
| 3 추적성 | **모든 mutation** (User/Listing/ListingPhoto 의 save + delete) 이 audit_log 자동 INSERT. correlation_id + actor_id 추적. SP5-iii 의 `find_by_correlation_id` 가 1 request 내 *모든* 9 BC mutation 추적 가능 |
| 4 안전성 | tx atomic — audit 실패 = 전체 실패. parameterized SQL only. RepoError 기존 enum 유지 (호환성). `unsafe` 0, `panic!` 0 |
| 5 가시성 | 모든 메서드 `tracing::instrument`. PII 미노출 (metadata/IP/UA/events.payload skip) |
| 6 SSOT | DB schema = SSOT. Repository trait 시그니처 일원화 (`save(agg, ctx)` 단일 형태) — 신규 BC 도 같은 패턴 채택 강제 |
| 7 명확성 | `ctx.action` 도메인 의미 강제 ("first_sign_in" / "update_profile" / "create_listing" 등). 예시는 plan 에서 강제 |

---

## 12. Follow-up items

본 sub-project 가 닫는 빚 외에도:

- **FU 19** — `MutationContext` 가 application layer 에서 자주 쓰이므로 helper 함수 (`MutationContext::http_user_action(req, action)` 등) 가 `services/api` 에 필요. SP6 frontend 작업 시작과 함께 추가
- **FU 20** — 기존 SP5-iii FU 14 (BVQ/LRQ updated_at 합성) 가 Core BC 에는 없음 — `User`/`Listing` 은 명시 `updated_at` 컬럼 보유. 본 sub-project 와 무관, SP5-iii FU 14 는 그대로 유지
- **AuditLog full diff capture** — 9 BC 공통 후속. SP5-iii FU 와 같음

---

## 13. 후속 sub-project 시드

- **SP5-ii**: Insights BC RDS Repository (Bookmark/SearchHistory/AnalysisReport/Notification) — 이제 Core/Audit/Pipeline/Operations 전체가 일관된 패턴이라 답습만
- **SP4**: 외부 API ingestion + R2 Reader 6 + Outbox publisher
- **SP6**: Frontend (Next.js)
