# Sub-project 5-i: Core BC RDS Repository SQLx 구현 (Spec)

| | |
|---|---|
| 작성일 | 2026-05-03 |
| 상태 | Approved |
| 선행 | SP1 (헌법 + 모노레포), SP2 (도메인 모델), SP3 (Auth) |
| 후속 | SP5-ii (Insights BC), SP5-iii (Audit + Pipeline + Operations BC) |
| 관련 ADR | [ADR-0008](../../adr/0008-spatial-postgis.md) (PostGIS SRID 4326) |

---

## 1. 개요

Core BC RDS Aggregate (`Listing`, `ListingPhoto`) 의 `Postgres` 저장소 구현체를 작성해요.
기존 `PgUserRepository` 의 18 필드 중 8 필드만 처리하던 기술 부채도 함께 보강해요.

본 sub-project 는 *진짜 SSS 급* 을 목표로 — 기존 `PgUserRepository` 패턴을 그대로
답습하지 않고 누락된 *가시성* / *자동 강제* 를 명시적으로 추가해요.

---

## 2. 범위 (Scope)

### 포함
- `PgListingRepository` — `crates/db/src/listing.rs` 신규
- `PgListingPhotoRepository` — `crates/db/src/listing_photo.rs` 신규
- `crates/db/src/error_map.rs` 공통 helper 신규 (`map_sqlx_err`)
- 모든 repo 메서드에 `#[tracing::instrument]` 적용 — `PgUserRepository` 도 같이 보강
- `PgUserRepository` 18 필드 완전 처리 (기존 8 필드 → 18 필드 — Walking Skeleton 한계 해소)
- 통합 테스트 (`crates/db/tests/*_integration.rs`) — `cargo test --features integration` 게이트
- CI workflow 갱신: integration test 게이트 명시 (`walking-skeleton.yml` 또는 신규 `db-integration.yml`)
- Cargo.toml 에 `[features] integration = []` 추가 + `cfg(feature = "integration")` gate

### 미포함 (후속 SP)
- Outbox + Aggregate 같은 트랜잭션 패턴 → SP5-iii (Audit BC 와 묶음)
- audit_log 자동 INSERT → SP5-iii
- 도메인 이벤트 발행 → SP5-iii
- R2 Reader 구현체 (`Parcel`/`Building`/`IndustrialComplex`/`Manufacturer`) → SP4 (외부 데이터 ingestion)
- `RealTransactionReader` / `CourtAuctionReader` → SP4
- `sqlx::query!()` macro 채택 (compile-time SSOT) — 별도 ADR 가치, 본 sub-project 는 runtime `sqlx::query` 유지
- `RepoError → IntoResponse` HTTP 매핑 → 후속 (axum 핸들러 표준 에러 매핑 sub-project)

---

## 3. 아키텍처

```
┌──────────────────────────────────────────────┐
│  ListingService / handler                    │
│  (application layer — sub-project 미정)      │
└──────────┬───────────────────────────────────┘
           │ ListingRepository trait
           ▼
┌──────────────────────────────────────────────┐
│  PgListingRepository (crates/db/src/listing) │
│  + #[tracing::instrument] on every method    │
│  + map_sqlx_err helper                       │
│  + ST_SetSRID + ST_MakePoint for geom_point  │
│  + OCC: WHERE version = $N                   │
│  + soft-delete: WHERE deleted_at IS NULL     │
└──────────┬───────────────────────────────────┘
           │ sqlx::PgPool
           ▼
       Postgres + PostGIS
       (migrations/ V001-V003_05)
```

`ListingPhotoRepository` 도 동일 패턴. `PgUserRepository` 도 같이 보강.

---

## 4. 컴포넌트 정의

### 4.1 `crates/db/src/error_map.rs` (신규)

```rust
//! `sqlx::Error` → `RepoError` 공통 매핑.
//!
//! 모든 `Pg*Repository` 가 사용하는 단일 매핑 함수.

use sqlx::Error as SqlxError;

/// `sqlx::Error` 를 도메인 `RepoError` 로 매핑해요.
///
/// - `RowNotFound` → 호출 측에서 `Option::None` 으로 처리하므로 본 함수는 *발생할 일이 없음*.
///   (`fetch_optional` 사용 시 `RowNotFound` 가 `None` 으로 변환됨.)
/// - Unique violation → `RepoError::Conflict`
/// - 그 외 → `RepoError::Database(<문자열>)` — 정보 누설 방지로 메시지만 보존.
pub fn map_sqlx_err<E: From<SqlxError>>(e: SqlxError) -> E
where
    E: ToRepoError,
{
    /* 실제 구현은 trait `ToRepoError` 통해 각 BC 의 `RepoError` enum 으로 분기 */
}
```

> **Note**: 각 도메인 crate (user-domain, listing-domain 등) 가 자체 `RepoError` 를 정의해요. `error_map.rs` 는 모든 enum 이 공통적으로 가진 3 variants (`NotFound`, `Conflict`, `Database(String)`) 를 가정하는 trait `ToRepoError` 또는 generic helper 로 구현. 또는 BC 별로 thin wrapper. 구현 단계에서 trade-off 결정.

### 4.2 `crates/db/src/listing.rs`

```rust
pub struct PgListingRepository {
    pool: PgPool,
}

#[async_trait]
impl ListingRepository for PgListingRepository {
    #[tracing::instrument(skip(self), fields(listing_id = %id.as_str()))]
    async fn find_by_id(&self, id: &Id<ListingMarker>)
        -> Result<Option<Listing>, RepoError>;

    #[tracing::instrument(skip(self), fields(owner_id = %owner.as_str()))]
    async fn find_by_owner(&self, owner: &Id<UserMarker>, limit: u32)
        -> Result<Vec<Listing>, RepoError>;

    #[tracing::instrument(skip(self, listing), fields(listing_id = %listing.id.as_str(), version = listing.version))]
    async fn save(&self, listing: &Listing) -> Result<(), RepoError>;

    // ... 그 외 trait 메서드
}
```

#### 도메인 → DB 컬럼 매핑 (spec § 5.1 verbatim)

`Listing` Aggregate 20 필드 → `listing` 테이블 20 컬럼 (1:1).

특수 처리:
- `geom_point: Option<Point<f64>>` (geo_types) ↔ `geometry(POINT, 4326)`:
  - INSERT: `ST_SetSRID(ST_MakePoint($lng, $lat), 4326)` (SRID 4326 — ADR-0008)
  - SELECT: `ST_X(geom_point) AS lng, ST_Y(geom_point) AS lat`
- `transaction_type: TransactionType` enum ↔ `varchar(20)`: `as_db_str()` / `from_db_str()`
- `status: ListingStatus` enum + `transaction_type` 의 cross-field invariant (V003_01) — DB CHECK 가 강제하지만 도메인 `try_new` 도 강제 (defense in depth)
- `version: i64` OCC

#### OCC + UPSERT 패턴

`PgUserRepository::save` 에서 검증된 패턴:

```rust
async fn save(&self, listing: &Listing) -> Result<(), RepoError> {
    let result = sqlx::query(r#"
        insert into listing (id, owner_id, parcel_pnu, listing_type, transaction_type,
                              ..., geom_point, version)
        values ($1, $2, $3, $4, $5, ..., ST_SetSRID(ST_MakePoint($N, $M), 4326), $V)
        on conflict (id) do update set
            <갱신 가능 필드들> = excluded.<...>,
            updated_at = excluded.updated_at,
            version = listing.version + 1
        where listing.version = $V
    "#)
    .bind(listing.id.as_str())
    // ... 나머지 binds
    .execute(&self.pool).await
    .map_err(map_sqlx_err)?;

    if result.rows_affected() == 0 {
        // INSERT 도 UPDATE 도 적용 안 됨 = 버전 mismatch
        return Err(RepoError::Conflict);
    }
    Ok(())
}
```

#### Soft-delete

- `find_by_id` 활성 행만: `WHERE id = $1 AND deleted_at IS NULL`
- `find_by_id_including_deleted` (필요 시): `WHERE id = $1`
- `save` 는 `deleted_at` 도 그대로 반영 (도메인 메서드로 set 한 후 save)

### 4.3 `crates/db/src/listing_photo.rs`

```rust
pub struct PgListingPhotoRepository { pool: PgPool }

#[async_trait]
impl ListingPhotoRepository for PgListingPhotoRepository {
    #[tracing::instrument(skip(self), fields(photo_id = %id.as_str()))]
    async fn find_by_id(&self, id: &Id<ListingPhotoMarker>)
        -> Result<Option<ListingPhoto>, RepoError>;

    #[tracing::instrument(skip(self), fields(listing_id = %listing_id.as_str()))]
    async fn find_by_listing(&self, listing_id: &Id<ListingMarker>)
        -> Result<Vec<ListingPhoto>, RepoError>;

    #[tracing::instrument(skip(self, photo), fields(photo_id = %photo.id.as_str(), order_index = photo.order_index))]
    async fn save(&self, photo: &ListingPhoto) -> Result<(), RepoError>;

    // ... 그 외
}
```

### 4.4 `crates/db/src/user.rs` 보강 (기존 코드 수정)

**현재 상태** (193줄, Walking Skeleton 한계):
- 18 필드 중 8 필드만 SELECT
- `roles`, `phone_kr_hash`, `business_*`, `broker_*`, `nice_*`, `marketing_*`, `last_login_at`, `deleted_at` 모두 누락

**SP5-i 변경**:
- `row_to_user` 가 18 필드 모두 처리
- `save` 가 18 필드 모두 INSERT/UPDATE
- `roles` (`text[]`) — `Vec<UserRole>` ↔ `Vec<String>` 양방향 변환 (`UserRole::as_str()` 활용)
- `business_number` / `broker_license_number` 값 객체 ↔ `varchar` 양방향
- 모든 메서드에 `#[tracing::instrument]` 추가

이 변경으로 SP3 first-sign-in 으로 자동 생성된 User 의 admin 부여 role 이 round-trip 되도록 해요.

### 4.5 통합 테스트 (`crates/db/tests/*_integration.rs`)

- `tests/listing_integration.rs` — 8-10 tests
- `tests/listing_photo_integration.rs` — 5-7 tests
- `tests/user_integration.rs` — 6-8 tests (기존 부재)
- `tests/error_map.rs` — 2-3 tests (unique violation → Conflict 등)

각 테스트 시나리오:
- 빈 DB → INSERT → SELECT 으로 round-trip 검증
- OCC: 동시 update 한 쪽 Conflict
- Unique violation (예: `listing.id` 중복) → Conflict
- Soft-delete → `find_by_id` `None`
- PostGIS round-trip: 입력 좌표 ↔ SELECT 좌표 정확 일치 (lat/lng)

`#[cfg(feature = "integration")]` 으로 게이트. 실행:
```
cargo test --workspace --features integration
```

`DATABASE_URL` 환경 변수 필수 — 미설정 시 테스트가 panic 하는 게 아니라 skip (또는 명시적 `expect("DATABASE_URL must be set")` — 단위 테스트 분리 명확).

---

## 5. CI 통합 (자동 강제)

### 5.1 신규 워크플로우 또는 기존 확장 — 결정

**옵션 A** (제 권장): 기존 `walking-skeleton.yml` 에 단계 추가
- 이미 PG 컨테이너 + 마이그 적용된 환경 활용
- API 빌드 직전에 `cargo test --workspace --features integration` 추가
- 단점: walking-skeleton 워크플로우 시간 증가 (3m45s → ~5-6m)

**옵션 B**: 신규 `db-integration.yml` 분리
- 더 빠른 PR 피드백 (병렬 실행)
- 단점: PG 컨테이너 + 마이그 setup 중복

본 spec 은 **A 채택** — 분리는 SP5-ii / SP5-iii 추가 시 시간 늘면 그때 재검토.

### 5.2 추가 단계 (`walking-skeleton.yml`)

```yaml
- name: Run integration tests (DB Repository)
  env:
    DATABASE_URL: postgres://gongzzang:ci_only_changeme@localhost:5432/gongzzang
  run: cargo test --workspace --features integration --no-fail-fast
```

API e2e 단계 *전* 또는 *후* — `migrations` 적용 직후가 자연스러움.

### 5.3 강제

위 step 빨강 → 워크플로우 빨강 → push 안 통과. SSS 자동 강제 게이트 확정.

---

## 6. 가시성 (`tracing::instrument`)

모든 repo 메서드에 적용. 패턴:

```rust
#[tracing::instrument(skip(self), fields(<doman_id> = %<id>.as_str()))]
async fn find_by_id(...) -> Result<...>;
```

`skip(self)` — `PgPool` 안 찍기 (logs 폭증 방지).
`fields(...)` — 도메인 식별자만 노출 (PII 노출 방지 — `email` / `zitadel_sub` 등은 적지 않음).

운영 시 효과:
- 어떤 repo 메서드가 느린지 즉시 식별 (Tempo 트레이스)
- Conflict 빈발하는 ID 패턴 추적
- `error.kind = "Conflict"` 같은 에러 분류 (구조적 로그)

`PgUserRepository` 도 같이 적용 (SSS 일관성).

---

## 7. 에러 매핑 정책

### 7.1 `map_sqlx_err` 단일 helper

```rust
pub fn map_sqlx_err<E: From<SqlxError>>(e: SqlxError) -> E
```

generic 으로 BC 별 `RepoError` 으로 매핑.

또는 더 간단한 직접 함수 (BC 별):
```rust
pub fn map_to_listing_repo_err(e: SqlxError) -> ListingDomainRepoError;
```

trade-off: generic = DRY, BC-specific = 명시적. 구현 단계에서 결정.

### 7.2 변환 규칙

| `sqlx::Error` | → `RepoError` |
|---|---|
| `RowNotFound` | (호출 측 `fetch_optional` 사용 → `None` 처리, 매핑 안 도달) |
| `Database(e)` `is_unique_violation()` | `Conflict` |
| `Database(e)` 그 외 | `Database(e.to_string())` |
| `Io(e)` / `Tls(e)` / 기타 | `Database(e.to_string())` |

원본 메시지 그대로 노출 → PII / 스키마 정보 누설 가능성. 본 spec 은 *허용* (운영자가 로그 보고 디버깅 필요). 외부 노출 (HTTP 응답) 시에는 추상화 단계가 따로 처리.

### 7.3 SQL injection 방어

**모든 사용자 입력은 `bind()` 통해 parameterized query 로 전달**. 문자열 보간 (`format!`) 으로 SQL 짜는 거 *금지*. 본 sub-project 의 모든 query 가 이 패턴 따름. clippy 가 잡지는 않으니 코드 리뷰 시 강제.

---

## 8. 테스트 전략

### 8.1 단위 테스트 (기존 trait object-safety + RepoError display 등)
- 변경 없음 — `crates/domain/core/listing/src/repository.rs` 등의 기존 unit tests 유지
- mock UserRepository 등은 SP3 가 사용 — 기존 그대로

### 8.2 통합 테스트 (신규)

`crates/db/tests/*_integration.rs`:

- ListingPhotoRepository: 5-7 tests
- ListingRepository: 8-10 tests
- UserRepository (확장): 6-8 tests
- error_map: 2-3 tests
- Total: ~22-28 신규 테스트

`#[cfg(feature = "integration")]` 게이트.

### 8.3 좋은 통합 테스트 시나리오 (예시)

```rust
// crates/db/tests/listing_integration.rs

#[cfg(feature = "integration")]
#[tokio::test]
async fn round_trip_listing_with_postgis_geom() {
    let pool = setup_test_pool().await;
    let repo = PgListingRepository::new(pool);

    let listing = Listing::try_new(
        Id::new(),
        owner_id,
        Pnu::try_new("1111010100100070000").unwrap(),
        // ...
        Some(Point::new(127.0276, 37.4979)), // 강남
        // ...
    ).unwrap();

    repo.save(&listing).await.unwrap();
    let fetched = repo.find_by_id(&listing.id).await.unwrap().unwrap();

    assert_eq!(fetched.geom_point, listing.geom_point); // 정확 일치 (4326 round-trip)
    assert_eq!(fetched.version, 1);
}
```

### 8.4 테스트 데이터 정리

- 각 통합 테스트 끝에 `truncate` 또는 transaction rollback
- 테스트 격리 보장 — 병렬 실행 시 다른 테스트 영향 X
- 세부 패턴은 plan 단계에서 결정 (transaction-per-test 가 sqlx 패턴 표준)

---

## 9. 검증 기준 (DoD)

본 sub-project 는 다음 모두 만족 시 종료:

1. `crates/db/src/listing.rs` + `crates/db/src/listing_photo.rs` 신규
2. `crates/db/src/error_map.rs` 신규
3. `crates/db/src/user.rs` 18 필드 완전 처리 + `#[tracing::instrument]` 적용
4. `Cargo.toml` `[features] integration = []` 추가
5. `crates/db/tests/*_integration.rs` 신규 ~22-28 tests
6. `walking-skeleton.yml` 에 `cargo test --features integration` 단계 추가
7. 모든 repo 메서드 `#[tracing::instrument(skip(self), fields(...))]`
8. 3 CI 워크플로우 모두 그린 (CI / db-migrations / walking-skeleton)
9. 누적 단위 + 통합 테스트 ≥1075 (1050 + ~25)
10. tarpaulin ≥90% 유지 (db crate 는 통합 테스트로 커버됨)
11. clippy `-D warnings` 통과
12. 모든 파일 ≤500 권장 / ≤1500 강제

---

## 10. SSS 7 기둥 매핑 (정직 평가)

| 기둥 | SP5-i 적용 |
|---|---|
| 1 일관성 | 모든 repo 같은 패턴 (`PgUserRepository` 보강 포함). 예외 0 |
| 2 자동 강제 | `cargo test --features integration` CI 단계 *명시*. 안 돌리면 워크플로우 빨강 |
| 3 추적성 | `version` OCC + `tracing::instrument` 구조적 로그. audit_log 자동 INSERT 는 SP5-iii 에서 |
| 4 안전성 | sqlx parameterized binding only, `RepoError` sealed, no panics, no `unsafe`, `is_unique_violation()` 명시 처리 |
| 5 가시성 | 모든 repo 메서드 `#[tracing::instrument]`, PII 미노출 (`skip(self)`, `fields` 화이트리스트) |
| 6 SSOT | migration `V001_01-V003_05` 가 SSOT. repo SELECT/INSERT 가 그걸 따름. `sqlx::query!()` macro 채택은 별도 ADR (compile-time SSOT 강화 — SP5-i 범위 외) |
| 7 명확성 | 도메인 용어 그대로, 시그니처 명확, error variants 한국어 해요체로 추상화 — `RepoError` 자체는 도메인 측에서 정의 (HTTP 응답 매핑은 후속) |

---

## 11. Follow-up items (production 배포 전)

1. **`sqlx::query!()` macro 채택 검토** — compile-time schema check. ADR 작성 가치
2. **audit_log 자동 INSERT** — SP5-iii (Outbox + Aggregate transactional save)
3. **HTTP 응답 매핑** — `RepoError → IntoResponse` 별도 sub-project
4. **Connection pool 운영 튜닝** — `max_connections`, idle timeout — Pulumi (SP8) 와 함께
5. **PostGIS injection 방지 추가 검증** — 모든 좌표 입력은 도메인 값 객체 (`Point<f64>`) 통과 후만 SQL 도달

---

## 12. 후속 sub-project 시드

- **SP5-ii**: Insights BC RDS Repository (Bookmark / SearchHistory / AnalysisReport / Notification)
- **SP5-iii**: Audit + Pipeline + Operations BC Repository + Outbox 트랜잭션 패턴
- **SP4**: 외부 API ingestion + R2 Reader 구현체 (Parcel/Building/IC/Mfr/RealTransaction/CourtAuction)
