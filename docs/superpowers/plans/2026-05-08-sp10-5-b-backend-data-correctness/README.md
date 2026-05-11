# SP10.5-B: Backend Data Correctness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** PII allowlist (default-deny) + Two-tier KMS-encrypted vault + PIPA 4원칙 자동 강제 + Vault RBAC + audit log + Building reader live wiring 으로 패널 backend data correctness 를 SSS-grade PIPA-compliant 로 hardening 한다.

**Architecture:** `SanitizingRawCapture` wrapper (정제) + `DualTierCapture` fan-out composer (Tier 2 vault 먼저 호출 → fail-fast → Tier 1 sanitized) + `PgPiiVaultCapture` (AWS KMS envelope encryption + Row-Level Security) + 5 신규 마이그레이션 (30010~30014) + admin RBAC endpoint + Tokio cleanup task + axum-test 실 router 통합 테스트.

**Tech Stack:** Rust workspace (tokio, axum, sqlx, async-trait, tracing, chrono) + PostgreSQL (RLS, composite FK) + AWS KMS (aws-sdk-kms) + AES-256-GCM (aes-gcm) + sha2 + ZITADEL JWT + axum-test 15.0.

**Spec SSOT:** [`docs/superpowers/specs/2026-05-08-sp10-5-b-backend-data-correctness-design.md`](../../specs/2026-05-08-sp10-5-b-backend-data-correctness-design.md) (commit `8a616f5`, v4, 715 줄).

---

## File Structure (spec §12 SSOT)

신규 파일 (8 Rust + 5 SQL + 1 Pulumi + 1 test):

| File | Purpose |
|---|---|
| `crates/data-clients/raw-capture/src/sanitizer.rs` | `RawSanitizer` trait + `SanitizedRaw` struct + `AllowlistSanitizer` + schema_hash |
| `crates/data-clients/raw-capture/src/capture.rs` | `SanitizingRawCapture<C>` wrapper + `DualTierCapture<S, V>` fan-out composer |
| `crates/data-clients/raw-capture/src/sources/data_go_kr_building.rs` | 7-path allowlist const (spec §5.2) |
| `crates/data-clients/raw-capture/src/sources/vworld_parcel.rs` | 9-path + envelope const (spec §5.3) |
| `crates/db/src/pii_vault.rs` | `PgPiiVaultCapture` (KMS `GenerateDataKey` + AES-256-GCM encrypt + INSERT) |
| `crates/db/src/access_log.rs` | `PgVaultAccessLog` (admin 조회 audit INSERT, fail-fast) |
| `services/api/src/state.rs` | `AppState` 정의 (DB/Redis pool, building_reader_status, vault_kms_status, kms_client) |
| `services/api/src/cleanup.rs` | Tokio interval task (`expires_at < now()` DELETE) |
| `services/api/src/lib.rs` | `pub fn app_router(state: AppState) -> Router` factory export (integration test 용) |
| `services/api/src/routes/admin/mod.rs` | admin module declaration |
| `services/api/src/routes/admin/raw_vault.rs` | `GET /api/admin/raw_vault/:source/:pnu` (ZITADEL admin + purpose + ticket_id) |
| `migrations/30010_source_taxonomy_expansion.sql` | V-World 'vworld' → 'vworld_parcel' rename + CHECK 확장 + backfill UPDATE |
| `migrations/30011_pii_vault.sql` | Tier 2 vault table + RLS policy + composite FK (`char(19)`/`varchar(40)`) |
| `migrations/30012_external_data_lineage.sql` | `license`, `api_version`, `sanitizer_version`, `schema_hash` 컬럼 ADD + legacy backfill |
| `migrations/30013_raw_vault_access_log.sql` | 7-컬럼 audit log (user_id, source, pnu, purpose, ticket_id, accessed_at, request_id) |
| `migrations/30014_external_data_expires_constraint.sql` | `expires_at` NULL backfill → NOT NULL → CHECK > fetched_at → index |
| `infra/kms-key.ts` | Pulumi `aws.kms.Key` ("pii-vault-key", rotation, deletion window 30d) |
| `services/api/tests/sp10_backend_data_correctness.rs` | PII fixture + vault RLS + audit log + health degraded 통합 테스트 |

기존 파일 수정 ([spec §11](../../specs/2026-05-08-sp10-5-b-backend-data-correctness-design.md) 표):

| File | Lines | Change |
|---|---|---|
| `crates/data-clients/vworld/src/reader.rs` | 71 | source literal `"vworld"` → `"vworld_parcel"` (또는 신규 `pub const RAW_CAPTURE_SOURCE: &str`) — **30010 마이그과 동일 PR** |
| `crates/data-clients/raw-capture/src/lib.rs` | 7-18 | doc comment 예시 `"vworld"` → `"vworld_parcel"` |
| `crates/db/src/raw_capture.rs` | 14-56 | lineage-aware 신규 메서드 (8-인자 INSERT) 추가 |
| `services/api/src/main.rs` | 210-221 | V-World capture wire: `PgRawCapture` → `DualTierCapture { sanitized, vault }` 합성체 |
| `services/api/src/main.rs` | 331-335 | `/healthz/ready` 핸들러 → `ReadinessResponse` 반환형 |
| `services/api/src/main.rs` | 390-413 | `Arc::new(NoOpBuildingRegisterReader)` → `Arc::new(DataGoKrBuildingReader::new(client, dual_tier))` |
| `services/api/src/routes/health.rs` | 45-125 | 신규 `ReadinessResponse { status, checks: ReadinessChecks }` 정의 |
| `services/api/tests/sp10_panel_endpoints.rs` | 29-36, 148-250 | `spawn_test_app()` 로컬 헬퍼 → `app_router(test_state)` + `axum_test::TestServer` |

---

## Task Overview

T1~T7 spec §13 literal 매핑. 각 task 는 한 파일에 모든 step 포함.

| Task | File | Step 수 | 시간 | Description |
|---|---|---|---|---|
| T1 | [T1-sanitizer-infra.md](T1-sanitizer-infra.md) | 10 | 6h | RawSanitizer trait + AllowlistSanitizer + SanitizingRawCapture |
| T2 | [T2-allowlists-migration.md](T2-allowlists-migration.md) | 11 | 8h | Allowlists + V-World rename (migration 30010 + reader.rs:71) |
| T3 | [T3-vault-kms-lineage.md](T3-vault-kms-lineage.md) | 14 | 16h | Vault migrations + PgPiiVaultCapture + KMS + DualTierCapture |
| T4 | [T4-ttl-cleanup.md](T4-ttl-cleanup.md) | 8 | 4h | expires_at TTL constraint + Tokio cleanup task |
| T5 | [T5-building-reader-live.md](T5-building-reader-live.md) | 8 | 4h | Building reader live wiring (NoOp swap, has_key/fail_fast 유지) |
| T6 | [T6-admin-rbac-audit.md](T6-admin-rbac-audit.md) | 12 | 12h | Vault admin endpoint + audit log + RBAC (migration 30013) |
| T7 | [T7-integration-health.md](T7-integration-health.md) | 10 | 8h | Integration test 재작성 + Health degraded (app_router export) |

**Total:** 73 step · 약 58 시간 · **5~7 영업일** (spec 추정 일치)

---

## TDD Pattern

모든 implementation step 은:

1. **Failing test** (실제 Rust code block)
2. **Run test** — `cargo test ...` → FAIL with specific error
3. **Minimal implementation** (실제 code block)
4. **Run test** — `cargo test ...` → PASS
5. **Commit** — Conventional Commits `<type>(sp10-5-b-T{N}): <description>`

Migration step 은:

1. SQL 작성
2. `cargo sqlx migrate run` (forward)
3. 검증 SQL (`psql -c "SELECT ..."`)
4. `cargo sqlx migrate revert` 확인 (rollback safety)
5. Commit

---

## Acceptance Criteria (spec §10 SSOT)

### 10.1 컴파일 / Lint

- `cargo check --workspace` 경고 0
- `cargo clippy --workspace -- -D warnings` 통과
- `cargo fmt --check` 통과
- `cargo sqlx prepare --check` 통과 (신규 마이그레이션 포함)
- `biome check apps/` 통과

### 10.2 Unit Tests (필수)

| 테스트 | Task | 검증 |
|---|---|---|
| `sanitizer::tests::allowlist_drops_unknown` | T1 | `ownerNm` 등 PII 폐기 |
| `sanitizer::tests::allowlist_retains_permitted` | T1 | 7-path 허용 필드 보존 |
| `sanitizer::tests::schema_hash_deterministic` | T1 | 동일 입력 → 동일 hash |
| `sanitizer::tests::schema_hash_version_sensitivity` | T1 | sanitizer_version 변경 시 hash 변경 |
| `capture::tests::dual_tier_vault_first_failfast` | T3 | Tier 2 Err → Tier 1 호출 차단 |
| `pii_vault::tests::kms_fail_fast` | T3 | KMS mock 실패 → capture Err |
| `access_log::tests::audit_insert_fail_fast` | T6 | audit INSERT 실패 → admin endpoint 403 |

### 10.3 Integration Tests (신규 / 재작성)

- `services/api/tests/sp10_backend_data_correctness.rs` 신규 — PII fixture / Tier 1 sanitized 검증 / Tier 2 vault 암호화 / admin RBAC 403/400/200 / audit log INSERT / health degraded
- `services/api/tests/sp10_panel_endpoints.rs` 재작성 — `app_router(test_state)` 실 router + `axum_test::TestServer`

### 10.4 Migration

- `sqlx migrate run` forward 5건 (30010~30014) 성공
- `sqlx migrate revert` 각 rollback 검증
- 멱등성: 동일 마이그레이션 재실행 시 오류 없음

### 10.5 Health Shape

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

Production 환경에서 `DATA_GO_KR_API_KEY` 미설정 시 `fail_fast_production` panic 으로 부팅 차단.

---

## ADR 후보

| ADR | 결정 |
|---|---|
| 0024 | PII Redaction Allowlist Default-deny |
| 0025 | Two-Tier Raw Capture Vault (KMS envelope + RLS) |
| 0026 | PIPA TTL Policy per Source (30일) |
| 0027 | Vault Access RBAC + Audit Log (admin role + purpose + ticket_id) |
| 0028 | AWS KMS via Pulumi (인프라 = 코드만) |

각 ADR 은 T3/T6/T7 commit 과 동일 PR 에 포함.

---

## Execution Order

T1 → T2 → T3 → T4 → T5 → T6 → T7

T2 의 마이그레이션 30010 + reader.rs:71 변경은 **동일 PR** 묶임 (마이그만 적용되고 코드가 'vworld' 그대로 INSERT 시 backfill 직후 다시 'vworld' 발생).

T3 의 마이그레이션 30011/30012 + AWS KMS 의존성 + DualTierCapture 도 **동일 PR** (PgPiiVaultCapture 가 vault 테이블 없이는 컴파일 가능하나 통합 테스트 깨짐).

---

## Risk Tracking

[spec §14](../../specs/2026-05-08-sp10-5-b-backend-data-correctness-design.md) 의 6 risk:

1. KMS 의존성 (AWS 지연/장애) — localstack + vault_kms degraded health check
2. Sanitizer drift — schema_hash 이력 + P1 Sentry alert (dropped_count >= 10)
3. Cleanup race — PostgreSQL MVCC (트랜잭션 단위 DELETE)
4. RLS bypass — `SET LOCAL app.role` 트랜잭션 시작 시 강제 적용
5. Audit INSERT 경합 — vault SELECT 와 동일 트랜잭션 + 실패 시 500
6. Migration 다운타임 (NOT NULL) — 30014 가 backfill UPDATE 선행

---

## Self-Review Notes

- Plan v1 (804f316, 폐기): Codex 작성, spec 8 mismatch (raw_captures, pii_vault, FK TEXT, V Flyway, etc.)
- Plan v2 (e20062a, 폐기): Codex 재작성, 잔여 5 mismatch (etl-base-layer crate, sanitizing_capture.rs 분리, cleanup_task 위치)
- Plan v3 (이 폴더): Claude 작성 (spec literal follow) + Codex 검수 (옵션 E++), 폴더 분해로 1500 한도 자동 통과
