# SP10.5-B Backend Data Correctness Design - Part 03: Acceptance, Integration Changes, Tasks, And SSS Mapping

Parent index: [SP10.5-B Backend Data Correctness Design](./2026-05-08-sp10-5-b-backend-data-correctness-design.md).

## 10. Acceptance Criteria

### 10.1 컴파일 / Lint

- cargo check --workspace 경고 0
- cargo clippy --workspace -- -D warnings 통과
- cargo fmt --check 통과
- cargo sqlx prepare --check 통과 (신규 마이그레이션 포함)
- biome check apps/ 통과

### 10.2 Unit Tests

| 테스트 | 검증 내용 |
|---|---|
| sanitizer::tests::allowlist_drops_unknown | /response/body/items/item/0/ownerNm 폐기 확인 |
| sanitizer::tests::allowlist_retains_permitted | 7-path 허용 필드 보존 확인 |
| sanitizer::tests::schema_hash_deterministic | 동일 입력 → 동일 hash |
| sanitizer::tests::schema_hash_version_sensitivity | sanitizer_version 변경 시 hash 변경 |
| pii_vault::tests::kms_fail_fast | KMS mock 실패 → capture Err 반환 |
| access_log::tests::audit_insert_fail_fast | audit INSERT mock 실패 → admin endpoint 403 |

### 10.3 Integration Tests

services/api/tests/sp10_backend_data_correctness.rs (신규):

- PII fixture → SanitizingRawCapture → Tier 1에 ownerNm 없음 확인
- Tier 2 vault INSERT → KMS 암호화 확인 (테스트 환경: localstack KMS 또는 mock)
- GET /api/admin/raw_vault/:source/:pnu 성공 → decrypted raw 반환
- admin role 없는 JWT → 403
- purpose 누락 → 400
- raw_vault_access_log 레코드 존재 확인
- GET /healthz/ready shape: building_reader: live, vault_kms: ok 확인
- NoOp reader 시 building_reader: degraded 확인

기존 services/api/tests/sp10_panel_endpoints.rs:29-36, 148-250 재작성: app_builder(state) 로 실 router 생성 후 axum_test::TestClient 사용.

### 10.4 Migration

- sqlx migrate run forward 성공
- sqlx migrate revert rollback 성공 (각 마이그레이션 DOWN 스크립트 포함)
- 멱등성: 동일 마이그레이션 재실행 시 오류 없음

### 10.5 Health Shape 검증

**현재 상태** ([services/api/src/routes/health.rs:45-125](../../../services/api/src/routes/health.rs#L45-L125)) — `HealthResponse` 는 `{ "status": "ok" }` 단순 구조. SP10.5-B 가 신규 `ReadinessResponse` 타입을 정의하고 `/healthz/ready` 핸들러에서 nested `checks` 를 직렬화하도록 확장. 기존 `liveness` (`/healthz`) 는 변경 없음 (단순 200 OK 유지).

확장 후 응답 shape:
```json
GET /healthz/ready -> 200
{
  "status": "ok",
  "checks": {
    "db": "ok",
    "redis": "ok",
    "building_reader": "live",
    "vault_kms": "ok"
  }
}
```

`status` 는 모든 `checks` 가 "ok"/"live" 일 때 "ok", 하나라도 "degraded"면 "degraded", "down"이면 "down". production 환경에서 building_reader 가 NoOp(키 미설정) 으로 부팅 시도 시 `fail_fast_production` 가 panic — `/healthz/ready` 응답 자체가 발행되지 않음 (서비스 시작 차단). dev/staging 에서만 `degraded` 응답이 가능.

---

## 11. 통합 변경 (기존 파일)

| 파일 | 라인 | 변경 내용 |
|---|---|---|
| crates/data-clients/raw-capture/src/lib.rs | 80-97 | RawCapture trait 시그니처 *유지* (`pnu, source, raw: &Value, fetched_at` / `Result<RawCaptureReceipt, RawCaptureError>`). 신규 `SanitizingRawCapture` + `DualTierCapture` 를 동일 모듈 또는 `capture.rs` 신규 파일로 추가 후 `pub use` re-export |
| crates/db/src/raw_capture.rs | 14-56 | 기존 `PgRawCapture::insert` 4-인자 유지. 신규 메서드 또는 wrapper 로 `(pnu, source, raw, fetched_at, sanitizer_version, schema_hash, license, api_version)` 8-인자 INSERT path 추가 (T3 — lineage 컬럼 30008 대응) |
| crates/data-clients/data-go-kr/src/building_register/reader.rs | 117-128 | 변경 없음 (wrapper transparent — 기존 `raw_capture.capture(pnu, source, &raw, now)` 호출이 이미 신규 trait 시그니처 일치) |
| crates/data-clients/data-go-kr/src/building_register/reader.rs | 37 | `RAW_CAPTURE_SOURCE = "data_go_kr_building"` 유지 |
| crates/data-clients/vworld/src/reader.rs | 35-96 | `VWorldParcelReader::new(client, raw_capture)` 시그니처 *유지*. 단 호출자가 주입하는 `Arc<dyn RawCapture>` 가 `DualTierCapture` 합성체로 교체됨 |
| crates/data-clients/vworld/src/reader.rs | **71** | `.capture(pnu.as_str(), "vworld", &raw, now)` 의 source string literal `"vworld"` → `"vworld_parcel"` 변경 필수. 30012 taxonomy 마이그레이션과 동시 적용 (마이그레이션은 backfill, 코드 변경은 신규 INSERT 가 정확한 enum 사용). 권장: `pub const RAW_CAPTURE_SOURCE: &str = "vworld_parcel";` 신규 const 도입 후 참조 (data-go-kr building reader 의 `RAW_CAPTURE_SOURCE` 패턴 따라) |
| crates/data-clients/raw-capture/src/lib.rs | 7-18 | doc comment 의 예시 `capture.capture("...", "vworld", ...)` 를 `"vworld_parcel"` 로 업데이트 (legacy alias 명시) |
| services/api/src/main.rs | 210-221 | V-World capture wire 가 현재 `Arc::new(PgRawCapture::new(pool.clone()))` 직접 주입 (b784e76) → `DualTierCapture { sanitized: SanitizingRawCapture::new(PgRawCapture, AllowlistSanitizer::for_source("vworld_parcel")), vault: PgPiiVaultCapture::new(pool, kms) }` 합성체로 교체 |
| services/api/src/main.rs | 331-335 | `/healthz/ready` 라우트의 핸들러를 신규 `ReadinessResponse` 반환형으로 교체. AppState 에 `building_reader_status` / `vault_kms_status` 핸들 추가 |
| services/api/src/main.rs | 390-413 | `Arc::new(NoOpBuildingRegisterReader)` → `Arc::new(DataGoKrBuildingReader::new(client, dual_tier_capture.clone()))` swap. `has_key` 분기 + `is_production` 시 `fail_fast_production` 패턴 (현재 코드와 동일) 유지. 키 없고 production 이면 부팅 panic (변경 0). 키 없고 non-production 이면 NoOp 유지 + `building_reader: degraded` 표시 |
| services/api/src/routes/health.rs | 45-125 | 기존 `HealthResponse { status: String }` 유지 (liveness 용). 신규 `ReadinessResponse { status: String, checks: ReadinessChecks }` + `ReadinessChecks { db, redis, building_reader, vault_kms: String }` 정의. readiness 핸들러를 새 응답형으로 교체 |
| services/api/src/main.rs | (신규 위치) | services/api 가 현재 `app_builder(state) -> Router` 같은 factory 를 *export 하지 않음*. T7 에서 `services/api/src/lib.rs` (또는 `state.rs`) 에 `pub fn app_router(state: AppState) -> Router` 또는 `pub fn build_app(state) -> Router` 추가 — 실 통합 테스트가 main.rs 의 wiring 을 그대로 호출 가능하도록 |
| services/api/tests/sp10_panel_endpoints.rs | 29-36, 148-250 | 현재 `spawn_test_app()` 로컬 헬퍼 (핸들러 부분 재구현) → `app_router(test_state)` 직접 호출 + `axum_test::TestServer::new(app_router(...))` 패턴으로 재작성 |
| migrations/30006_parcel_external_data.sql | (선행 마이그레이션) | 30012 가 이 테이블의 source CHECK 를 확장 (변경 없음 — 30006 자체는 그대로) |

---

## 12. v1 신규 파일

```
crates/data-clients/raw-capture/src/
  sanitizer.rs
  capture.rs
  sources/
    data_go_kr_building.rs
    vworld_parcel.rs

crates/db/src/
  pii_vault.rs
  access_log.rs

services/api/src/
  state.rs
  cleanup.rs
  routes/admin/
    mod.rs
    raw_vault.rs

migrations/
  30012_source_taxonomy_expansion.sql   (vworld → vworld_parcel rename, CHECK 확장)
  30013_pii_vault.sql                    (Tier 2 vault + RLS + composite FK)
  30014_external_data_lineage.sql        (license, api_version, sanitizer_version, schema_hash)
  30015_raw_vault_access_log.sql         (admin 조회 audit log)
  30016_external_data_expires_constraint.sql  (expires_at NOT NULL + CHECK + index)

infra/
  kms-key.ts                             (Pulumi PII vault CMK; AGENTS.md §1 — 인프라는 코드만)

services/api/src/
  lib.rs                                 (또는 main.rs 에서 app_router 분리; 통합 테스트 export 용)

services/api/tests/
  sp10_backend_data_correctness.rs
```

**Cargo.toml 의존성 추가** (T1/T3/T7 에 분산):
- `crates/data-clients/raw-capture/Cargo.toml` — `sha2` (schema hash 산출). `async-trait` / `chrono` / `tracing` / `serde_json` 은 이미 존재.
- `crates/db/Cargo.toml` — `aws-sdk-kms` (Tier 2 envelope encryption), `aes-gcm` (DEK 로 ciphertext_blob 암호화).
- `services/api/Cargo.toml` (dev-dependencies) — `axum-test` (실 router 통합 테스트), `localstack` 또는 `aws-sdk-kms` mock 헬퍼.
- `infra/package.json` — `@pulumi/aws` (KMS 리소스). 기존 의존성 확인 후 누락이면 추가.

---

## 13. Task 분해

### T1: RawSanitizer + SanitizingRawCapture Infra

- crates/data-clients/raw-capture/src/sanitizer.rs 작성: RawSanitizer trait, SanitizedRaw, AllowlistSanitizer
- crates/data-clients/raw-capture/src/capture.rs 작성: SanitizingRawCapture (schema drift tracing::warn! 포함) + DualTierCapture (Tier 2 먼저 호출 fail-fast fan-out)
- Unit test: allowlist drop/retain, schema_hash determinism

### T2: Allowlist 정의 + source taxonomy 확장

- migrations/30012_source_taxonomy_expansion.sql: `parcel_external_data` CHECK 확장 (`vworld_parcel` 추가) + 기존 `vworld` row 를 `vworld_parcel` 로 backfill UPDATE
- **crates/data-clients/vworld/src/reader.rs:71**: source literal `"vworld"` → `"vworld_parcel"` 변경 (또는 신규 `pub const RAW_CAPTURE_SOURCE: &str = "vworld_parcel"` 도입 후 참조). data.go.kr building reader 패턴 따라 const SSOT 화 권장. **마이그레이션과 코드 변경이 동일 PR 에 묶여야 함** — 마이그만 적용되고 코드가 'vworld' 그대로 INSERT 시 backfill 직후 다시 'vworld' row 가 생김
- crates/data-clients/raw-capture/src/lib.rs:7-18 doc comment 예시 업데이트 (`vworld` → `vworld_parcel`)
- sources/data_go_kr_building.rs: 7-path allowlist const 정의
- sources/vworld_parcel.rs: V-World allowlist const 정의
- AllowlistSanitizer::for_source(&str) 팩토리 함수 export

### T3: Two-tier Vault 마이그레이션 + PgPiiVaultCapture + Lineage 컬럼

- migrations/30013_pii_vault.sql: vault table + RLS + composite FK (pnu char(19), source varchar(40))
- migrations/30014_external_data_lineage.sql: `license`, `api_version`, `sanitizer_version`, `schema_hash` 컬럼 추가 + legacy backfill (`schema_hash = 'legacy:' || md5(...)`, `sanitizer_version = 0`)
- crates/db/src/raw_capture.rs: `PgRawCapture` 에 lineage-aware 신규 메서드 또는 wrapper struct (8-인자 INSERT)
- crates/db/src/pii_vault.rs: PgPiiVaultCapture (`aws-sdk-kms` GenerateDataKey → AES-256-GCM encrypt full raw → ciphertext_blob INSERT). `RawCapture` trait impl 시그니처는 `(pnu, source, &Value, fetched_at)` 그대로
- infra/kms-key.ts: Pulumi `aws.kms.Key` ("pii-vault-key", `enableKeyRotation: true`, `deletionWindowInDays: 30`)
- Cargo.toml 의존성: `sha2` (raw-capture), `aws-sdk-kms` + `aes-gcm` (db)

### T4: expires_at NOT NULL + Cleanup Task

- migrations/30016_external_data_expires_constraint.sql: NOT NULL CHECK + index
- services/api/src/cleanup.rs: Tokio interval task
- services/api/src/main.rs: tokio::spawn(run_cleanup_task(...)) 등록

### T5: Building Reader Live Wiring

- services/api/src/main.rs:390-413: 기존 `Arc::new(NoOpBuildingRegisterReader)` → `Arc::new(DataGoKrBuildingReader::new(client, dual_tier_capture.clone()))` swap. 단 `has_key` 분기와 `if !has_key && is_production { fail_fast_production(...) }` 패턴은 *현재 코드 그대로 유지* — production 에서 `DATA_GO_KR_API_KEY` 미설정 시 부팅 panic. dev/staging 에서 키 없으면 NoOp 유지하되 AppState 의 `building_reader_status = "degraded"` 로 표시
- /healthz/ready 핸들러가 AppState 의 `building_reader_status` 를 읽어 응답 직렬화 (T7 의 `ReadinessResponse` 와 연결)

### T6: Vault Access RBAC Admin Endpoint + Audit Log

- migrations/30015_raw_vault_access_log.sql: audit log table
- crates/db/src/access_log.rs: PgVaultAccessLog
- services/api/src/routes/admin/raw_vault.rs: endpoint (ZITADEL role check → vault SELECT → KMS Decrypt → audit INSERT → response)
- services/api/src/main.rs:331-335: admin route 등록

### T7: Integration Test 재작성 + Health Degraded

- services/api/src/lib.rs (또는 main.rs 분리): `pub fn app_router(state: AppState) -> Router` 신규 export. 현재 services/api 는 router/state factory 를 export 하지 않으므로 통합 테스트가 `spawn_test_app()` 로컬 헬퍼로 핸들러 부분 재구현 — 이걸 *실 main.rs wiring* 과 동일한 factory 를 export 하도록 분리
- services/api/src/state.rs: `AppState` 정의 (DB pool, redis pool, building_reader_status, vault_kms_status, kms_client 등 모든 wiring 핸들 포함). main.rs 가 `AppState::from_env()` 로 빌드 후 `app_router(state)` 호출
- services/api/tests/sp10_panel_endpoints.rs:29-36, 148-250: 핸들러 재구현 → `app_router(test_state)` 직접 호출 + `axum_test::TestServer::new(...)` 패턴
- services/api/tests/sp10_backend_data_correctness.rs (신규): PII fixture 포함 통합 테스트 (Tier 1 sanitized 검증 / Tier 2 vault 암호화 검증 / admin RBAC 403 / audit log INSERT / health degraded)
- services/api/src/routes/health.rs:45-125: 기존 `HealthResponse` 유지 + 신규 `ReadinessResponse { status, checks: ReadinessChecks }` 추가. readiness 핸들러를 `AppState` 에서 status 읽어 직렬화하도록 수정
- Cargo.toml dev-dependency: `axum-test` 추가

---

## 14. 리스크 → 완화

| 리스크 | 완화 |
|---|---|
| KMS 의존성 — AWS API 지연/장애 | localstack 로컬 테스트, vault_kms: degraded health check, circuit breaker (AGENTS.md §2) |
| Sanitizer drift — API schema 변경으로 필수 필드 폐기 | schema_hash 이력 + P1 Sentry alert (dropped_count >= 10) |
| Cleanup race — 삭제 중 조회 | PostgreSQL MVCC 보장, DELETE 는 트랜잭션 단위 |
| RLS bypass — app.role 설정 누락 | SET LOCAL app.role = ... 를 트랜잭션 시작 시 강제 적용 |
| Audit INSERT 경합 — 고부하 시 지연 | audit INSERT 는 vault SELECT 와 동일 트랜잭션, 실패 시 500(fail-fast) |
| Migration 다운타임 — NOT NULL 추가 | expires_at 은 신규 INSERT 부터만 적용, 기존 레코드 DEFAULT backfill 마이그레이션 선행 |

---

## 15. FU Phase-2

- **필드 단위 토큰화**: 소유자명 → 가역 토큰, 마스킹 뷰 제공
- **Full raw 필드 암호화**: vault ciphertext 를 column-level 분리 (현재 단일 blob)
- **GDPR right-to-erasure**: PNU 기반 전체 삭제 파이프라인
- **ADR-driven sanitizer evolution**: allowlist 변경 시 ADR PR 필수 (lefthook 검사)
- **Multi-source TTL 거버넌스**: source 별 TTL 을 DB 테이블로 관리, 런타임 조회

---

## 16. SSS 15 Axes 매핑

| Axis | SP10.5-B 구현 | 검증 방법 |
|---|---|---|
| 일관성 | DualTierCapture → SanitizingRawCapture + PgPiiVaultCapture 단일 경로 강제 | Rust trait impl 컴파일 |
| 자동 강제 | Allowlist default-deny 폐기 | unit test: allowlist_drops_unknown |
| 추적성 | schema_hash + sanitizer_version DB 저장 | sqlx prepare + integration test |
| 안전성 | Tier 2 fail-fast, audit fail-fast | unit test: kms_fail_fast, audit_insert_fail_fast |
| 가시성 | tracing::warn! + Sentry alert | drift detection test |
| SSOT | allowlist = 단일 파일 | code review |
| 명확성 | 명시적 타입/네이밍 | clippy -D warnings |
| PII 차단 | Allowlist + KMS vault | integration: PII fixture 폐기 확인 |
| 최소 수집 | 7-path allowlist | unit test: allowlist_retains_permitted |
| 목적 한정 | purpose_code + RBAC endpoint | integration: purpose 누락 → 400 |
| 보유 기간 | expires_at NOT NULL CHECK | migration 30016 |
| 파기 | Tokio cleanup task + audit | integration: cleanup task 호출 |
| 접근 제어 | ZITADEL admin role + RLS | integration: non-admin → 403 |
| 감사 | raw_vault_access_log INSERT | integration: log 레코드 확인 |
| 인프라 코드화 | KMS = infra/kms-key.ts Pulumi | Pulumi preview CI |
