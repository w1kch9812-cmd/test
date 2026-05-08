# SP10.5-B: Backend Data Correctness — SSS-grade PIPA-compliant Hardening

| | |
|---|---|
| 작성일 | 2026-05-08 |
| 상태 | Draft |
| 결정 ADR | 0024 (Allowlist), 0025 (Two-tier vault), 0026 (TTL), 0027 (RBAC + audit), 0028 (AWS KMS Pulumi) |
| 목적 | 패널 backend data correctness 를 SSS-grade PIPA-compliant 로 hardening |
| 추정 | 5~7 영업일 |

---

## 1. 목표

1. **PII 기본 차단**: 외부 API 응답에 포함될 수 있는 개인정보(소유자명, 연락처 등)를 수집 시점에 Allowlist 기반으로 자동 폐기한다.
2. **이중 저장 분리**: 정제된 데이터는 기존 parcel_external_data (Tier 1), 원문 full-raw는 KMS 암호화된 parcel_external_data_pii_vault (Tier 2)에 분리 보관한다.
3. **PIPA 4원칙 강제**: 수집 목적 한정·최소 수집·보유 기간·파기를 시스템이 자동 강제한다.
4. **Vault 접근 RBAC + 감사**: 원문 조회는 ZITADEL admin role + purpose code + ticket_id 요건을 충족해야만 허용하며 모든 접근이 기록된다.
5. **Building reader 실 연결**: NoOpBuildingRegisterReader 교체로 data.go.kr 건축물대장 실 데이터 수신을 활성화한다.
6. **통합 테스트 실 router 전환**: 핸들러 로직 재구현 방식에서 진짜 Axum router 호출 방식으로 교체해 회귀 감지력을 높인다.
7. **Readiness degraded 표시**: /healthz/ready 응답이 building_reader, vault_kms 상태를 포함하도록 확장된다.

---

## 2. 비목표

- 필드 단위 토큰화(field-level tokenization) — Phase-2 FU 항목
- GDPR right-to-erasure 구현 — Phase-2 FU 항목
- AI 어시스턴트 경로(apps/ai-assistant/) 연동 — AGENTS.md §3 별도 모듈
- Pulumi 외 AWS 콘솔 직접 변경 — AGENTS.md §1 절대 규칙
- 공공 데이터 재배포 · 오픈소스 공개 — AGENTS.md §6 사용자 확인 필요 항목

---

## 3. 핵심 추상화

### 3.1 RawSanitizer trait

```rust
// crates/data-clients/raw-capture/src/sanitizer.rs (신규)
pub trait RawSanitizer: Send + Sync {
    /// source_id 별 allowlist 로 raw JSON 을 정제한다.
    fn sanitize(&self, raw: &serde_json::Value) -> SanitizedRaw;
}

pub struct SanitizedRaw {
    pub value: serde_json::Value,
    pub dropped_count: usize,
    pub schema_hash: String,
    pub sanitizer_version: u32,
}
```

### 3.2 AllowlistSanitizer

```rust
pub struct AllowlistSanitizer {
    pub source: String,
    pub allowed_paths: Vec<String>,
    pub sanitizer_version: u32,
}
impl RawSanitizer for AllowlistSanitizer { /* ... */ }
```

### 3.3 SanitizingRawCapture

```rust
// crates/data-clients/raw-capture/src/capture.rs (신규)
pub struct SanitizingRawCapture<C: RawCapture> {
    inner: C,
    sanitizer: Arc<dyn RawSanitizer>,
}

impl<C: RawCapture> RawCapture for SanitizingRawCapture<C> {
    async fn capture(&self, source: &str, pnu: &str, raw: serde_json::Value) -> Result<()> {
        let sanitized = self.sanitizer.sanitize(&raw);
        if sanitized.dropped_count > 0 {
            tracing::warn!(
                target = "raw.capture.schema_drift",
                source = source,
                schema_hash = %sanitized.schema_hash,
                dropped_count = sanitized.dropped_count,
            );
        }
        self.inner.capture(source, pnu, sanitized.value).await?;
        Ok(())
    }
}
```

### 3.4 Two-tier Sink

- **Tier 1** (PgRawCapture): parcel_external_data.raw_response — 정제된 JSON
- **Tier 2** (PgPiiVaultCapture): parcel_external_data_pii_vault.ciphertext_blob — KMS 암호화 full-raw

### 3.5 DualTierCapture (Fan-out Composer)

`SanitizingRawCapture` 는 sanitization 단독 책임(정제 후 Tier 1 sink 에 전달). Tier 1 + Tier 2 fan-out 은 `DualTierCapture` 가 담당한다.
Tier 2 (vault) 를 먼저 호출하여 fail-fast 를 보장한다.

```rust
// crates/data-clients/raw-capture/src/capture.rs
/// Tier 1 (sanitized) + Tier 2 (vault) fan-out. Tier 2 먼저 호출 → fail-fast 보장.
pub struct DualTierCapture<S, V> {
    sanitized: S, // SanitizingRawCapture<PgRawCapture>  — Tier 1 정제 경로
    vault: V,     // PgPiiVaultCapture                   — Tier 2 원문 vault 경로
}

#[async_trait]
impl<S, V> RawCapture for DualTierCapture<S, V>
where
    S: RawCapture + Send + Sync,
    V: RawCapture + Send + Sync,
{
    async fn capture(&self, source: &str, pnu: &str, raw: serde_json::Value) -> Result<()> {
        // Tier 2 먼저: vault INSERT 실패 → 전체 fail-fast, Tier 1 기록 차단
        self.vault.capture(source, pnu, raw.clone()).await?;
        // Tier 1: 정제된 JSON 저장
        self.sanitized.capture(source, pnu, raw).await
    }
}
```

`services/api/src/main.rs` 에서 wiring:
```rust
let capture = DualTierCapture {
    sanitized: SanitizingRawCapture::new(pg_raw_capture, sanitizer),
    vault: PgPiiVaultCapture::new(pool.clone(), kms_client.clone()),
};
```

---

## 4. 데이터 흐름

```
외부 API (data.go.kr / V-World)
        |
        v
  Reader (DataGoKrBuildingReader / VWorldParcelReader)
        |  raw serde_json::Value
        v
  DualTierCapture  (§3.5 fan-out composer)
        |                                         |
        | Tier 2 먼저 (fail-fast)                 | Tier 1
        v                                         v
  PgPiiVaultCapture               SanitizingRawCapture<PgRawCapture>
  full raw + KMS 암호화                정제된 JSON (allowlist)
        |                                         |
        v                                         v
  parcel_external_data_pii_vault    parcel_external_data
  .ciphertext_blob (BYTEA, KMS)     .raw_response (JSONB)
        |                                         |
        v                                         v
  expires_at TTL 90d/30d            expires_at TTL 90d/30d
  cleanup Tokio task                cleanup Tokio task
```

Tier 2 INSERT 실패 시 `DualTierCapture` 가 즉시 `Err` 를 반환하여 Tier 1 기록 자체를 차단하는 fail-fast 가 적용된다.

---

## 5. PII Redaction 정책

### 5.1 Allowlist 채택 근거 (ADR 0024)

Denylist 방식은 신규 필드 추가 시 PII 누출을 막지 못한다. Allowlist(default-deny)는 unknown field 를 자동 폐기하므로 API provider schema 변경에 안전하다.

### 5.2 data_go_kr_building Day-1 Allowlist

| # | JSON Pointer | 내용 |
|---|---|---|
| 1 | /response/header/resultCode | 응답 코드 |
| 2 | /response/header/resultMsg | 응답 메시지 |
| 3 | /response/body/items/item/*/mgmBldrgstPk | 건축물대장 PK |
| 4 | /response/body/items/item/*/bldNm | 건물명 |
| 5 | /response/body/items/item/*/mainPurpsCdNm | 주용도 |
| 6 | /response/body/items/item/*/totArea | 연면적 |
| 7 | /response/body/items/item/*/useAprDay | 사용승인일 |

소유자명(ownerNm), 주민등록번호, 연락처 등 모든 비허용 필드는 정제 단계에서 폐기된다.

### 5.3 vworld_parcel Allowlist

| 경로 | 내용 |
|---|---|
| /response/result/featureCollection/features/*/geometry | 지적 폴리곤 좌표 (EPSG:4326) |
| /response/result/featureCollection/features/*/properties/pnu | 필지 고유번호 |
| /response/result/featureCollection/features/*/properties/jibun | 지번 |
| /response/result/featureCollection/features/*/properties/bchk | 본번/부번 구분 |
| /response/result/featureCollection/features/*/properties/addr | 도로명주소 |

허용 경로 외 모든 필드는 동일하게 폐기된다.

### 5.4 Schema Hash

```
schema_hash = SHA-256(
  source_id || ":" || sanitizer_version || ":" || sorted_retained_json_paths.join(",")
)
```

schema_hash 는 parcel_external_data.schema_hash 컬럼에 저장되어 schema drift 이력을 추적한다.

---

## 6. Two-tier Vault 설계

### 6.1 Tier 1 테이블 (기존 + 컬럼 추가)

```sql
-- migrations/30008_external_data_lineage.sql
ALTER TABLE parcel_external_data
  ADD COLUMN license           TEXT,
  ADD COLUMN api_version       TEXT,
  ADD COLUMN sanitizer_version INT  NOT NULL DEFAULT 1,
  ADD COLUMN schema_hash       TEXT;

-- § 6.1 backfill: 기존 레코드에 대한 schema_hash / sanitizer_version 초기화
-- migration 30008 실행 시 아래 UPDATE 를 함께 포함한다.
-- legacy: prefix 는 SHA-256 hash 아님을 명시적으로 표시한다.
UPDATE parcel_external_data
   SET schema_hash       = 'legacy:' || md5(raw_response::text),
       sanitizer_version = 0
 WHERE schema_hash IS NULL OR schema_hash = '';
```

### 6.2 Tier 2 테이블 (신규)

```sql
-- migrations/30007_pii_vault.sql
-- ADR 근거: parcel_external_data PK 가 (pnu, source) composite 이므로 surrogate UUID FK 는
-- 컴파일 전 fail. 변경 최소화를 위해 composite REFERENCES 채택 (옵션 A).
CREATE TABLE parcel_external_data_pii_vault (
  id               UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
  pnu              TEXT        NOT NULL,
  source           TEXT        NOT NULL,
  FOREIGN KEY (pnu, source) REFERENCES parcel_external_data(pnu, source) ON DELETE CASCADE,
  ciphertext_blob  BYTEA       NOT NULL,
  kms_key_id       TEXT        NOT NULL,
  encryption_ctx   JSONB       NOT NULL DEFAULT '{}',
  captured_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
  expires_at       TIMESTAMPTZ NOT NULL
);

ALTER TABLE parcel_external_data_pii_vault ENABLE ROW LEVEL SECURITY;

CREATE POLICY vault_admin_only ON parcel_external_data_pii_vault
  USING (current_setting('app.role', true) = 'admin');
```

### 6.3 AWS KMS Envelope Encryption (ADR 0025, ADR 0028)

Pulumi TypeScript 변경 위치: infra/kms-key.ts (신규)

```typescript
// infra/kms-key.ts
import * as aws from "@pulumi/aws";

export const piiVaultKey = new aws.kms.Key("pii-vault-key", {
  description: "gongzzang PII vault CMK",
  enableKeyRotation: true,
  deletionWindowInDays: 30,
});
```

Key Policy: services/api task role 에만 kms:GenerateDataKey + kms:Decrypt 허용. PgPiiVaultCapture 가 generate_data_key 로 DEK 생성 → full raw JSON 을 AES-256-GCM 암호화 → ciphertext_blob (encrypted_dek || iv || ciphertext) INSERT.

### 6.4 Admin Endpoint Contract (ADR 0027)

```
GET /api/admin/raw_vault/:source/:pnu
Authorization: Bearer <ZITADEL admin JWT>
Query: purpose=incident_investigation|drift_diagnosis|customer_request
       ticket_id=<string>

200 OK
{
  "source": "data_go_kr_building",
  "pnu": "1111010100100010001",
  "captured_at": "2026-05-08T00:00:00Z",
  "raw": { /* full decrypted JSON */ }
}

403 Forbidden   — admin role 미보유
400 Bad Request — purpose/ticket_id 누락
404 Not Found   — vault record 없음
```

모든 호출은 응답 전에 raw_vault_access_log 에 INSERT한다. 감사 INSERT 실패 시 응답 차단(fail-fast).

---

## 7. expires_at + Cleanup

### 7.1 Source 별 TTL

| source | TTL | 근거 |
|---|---|---|
| data_go_kr_building | 90일 | 건축물대장 변동 빈도 낮음, API 갱신 주기 반영 |
| vworld_parcel | 30일 | 지적 변경 빈도 낮으나 공공 갱신 주기 고려 |

### 7.2 NOT NULL CHECK 제약 (ADR 0026)

```sql
-- migrations/30010_external_data_expires_constraint.sql
ALTER TABLE parcel_external_data
  ALTER COLUMN expires_at SET NOT NULL;

ALTER TABLE parcel_external_data
  ADD CONSTRAINT check_expires_future
  CHECK (expires_at > captured_at);

CREATE INDEX idx_external_data_expires ON parcel_external_data (expires_at);
CREATE INDEX idx_pii_vault_expires ON parcel_external_data_pii_vault (expires_at);
```

### 7.3 Tokio Cleanup Task

```rust
// services/api/src/cleanup.rs (신규)
pub async fn run_cleanup_task(pool: PgPool, interval: Duration) {
    let mut ticker = tokio::time::interval(interval);
    loop {
        ticker.tick().await;
        let r1 = sqlx::query!("DELETE FROM parcel_external_data WHERE expires_at < now()")
            .execute(&pool).await;
        let r2 = sqlx::query!("DELETE FROM parcel_external_data_pii_vault WHERE expires_at < now()")
            .execute(&pool).await;
        tracing::info!(
            target = "cleanup.expires_at",
            tier1_deleted = r1.map(|r| r.rows_affected()).unwrap_or(0),
            tier2_deleted = r2.map(|r| r.rows_affected()).unwrap_or(0),
        );
    }
}
```

services/api/src/main.rs startup: tokio::spawn(run_cleanup_task(pool.clone(), Duration::from_secs(3600))).

---

## 8. Schema Drift Detection

### 8.1 Hash 산출

```rust
let mut paths: Vec<&str> = retained_paths.iter().map(|s| s.as_str()).collect();
paths.sort();
let input = format!("{}:{}:{}", source, sanitizer_version, paths.join(","));
let schema_hash = format!("{:x}", Sha256::digest(input.as_bytes()));
```

### 8.2 Warn Metric

target = "raw.capture.schema_drift" 로그를 tracing-opentelemetry 가 Grafana Loki 로 전달.

### 8.3 Sentry Alert 임계

| 조건 | 심각도 | 대응 |
|---|---|---|
| dropped_count >= 1, 5분 내 3회 | P2 | allowlist PR 검토 |
| dropped_count >= 10, 1회 | P1 | API schema 주요 변경, 즉시 대응 |

---

## 9. Production Rules (15 Axes)

AGENTS.md §10 SSS 15 Axes 기준 enforce 메커니즘:

| Axis | 규칙 | Enforce 메커니즘 |
|---|---|---|
| 1. 일관성 | 모든 capture 경로가 SanitizingRawCapture 통과 | Rust 타입 시스템: RawCapture impl 강제 |
| 2. 자동 강제 | Allowlist 외 필드 자동 폐기 | AllowlistSanitizer 컴파일타임 |
| 3. 추적성 | schema_hash, sanitizer_version DB 컬럼 | migration 30008 NOT NULL |
| 4. 안전성 | Tier 2 fail-fast, audit fail-fast | `Result<T, E>` propagation via `?` + unit tests for fail-fast paths |
| 5. 가시성 | tracing::warn! + Sentry alert | tracing-opentelemetry + Grafana Loki |
| 6. SSOT | allowlist 정의 = 단일 파일 | code review |
| 7. 명확성 | 명시적 타입/네이밍 | clippy -D warnings |
| 8. PII 차단 | Allowlist + KMS vault | integration: PII fixture 폐기 확인 |
| 9. 최소 수집 | 7-path allowlist | unit test: allowlist_retains_permitted |
| 10. 목적 한정 | purpose_code per source + RBAC endpoint | integration: purpose 누락 → 400 |
| 11. 보유 기간 | expires_at NOT NULL CHECK | migration 30010 constraint |
| 12. 파기 | Tokio cleanup task + audit log | integration: cleanup task |
| 13. 접근 제어 | ZITADEL admin role + PostgreSQL RLS | integration: non-admin → 403 |
| 14. 감사 | raw_vault_access_log INSERT | integration: log 레코드 확인 |
| 15. 인프라 코드화 | KMS = infra/kms-key.ts Pulumi | Pulumi preview CI |

---

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

production 환경에서 building_reader: degraded 시 서비스 시작 차단(fail-fast panic).

---

## 11. 통합 변경 (기존 파일)

| 파일 | 라인 | 변경 내용 |
|---|---|---|
| crates/data-clients/raw-capture/src/lib.rs | 40-89 | RawCapture trait 유지, SanitizingRawCapture + DualTierCapture 모듈 re-export 추가 |
| crates/db/src/raw_capture.rs | 14-56 | sanitizer_version/schema_hash INSERT 포함 |
| crates/data-clients/data-go-kr/src/building_register/reader.rs | 117-128 | 변경 없음 (wrapper transparent) |
| crates/data-clients/data-go-kr/src/building_register/reader.rs | 37 | RAW_CAPTURE_SOURCE = "data_go_kr_building" 유지 |
| crates/data-clients/vworld/src/reader.rs | 35-96 | VWorldParcelReader::new 파라미터: SanitizingRawCapture wrapper 수용 |
| services/api/src/main.rs | 210-221 | V-World capture: DualTierCapture { sanitized: SanitizingRawCapture::new(pg_raw_capture, sanitizer), vault: PgPiiVaultCapture::new(...) } |
| services/api/src/main.rs | 331-335 | health route: degraded 응답 구조 포함 |
| services/api/src/main.rs | 390-413 | NoOpBuildingRegisterReader → DataGoKrBuildingReader live swap |
| services/api/src/routes/health.rs | 45-125 | /healthz/ready 응답 shape 확장 |
| services/api/tests/sp10_panel_endpoints.rs | 29-36, 148-250 | app_builder(state) 사용 실 router 호출로 재작성 |

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
  30007_pii_vault.sql
  30008_external_data_lineage.sql
  30009_raw_vault_access_log.sql
  30010_external_data_expires_constraint.sql

infra/
  kms-key.ts

services/api/tests/
  sp10_backend_data_correctness.rs
```

---

## 13. Task 분해

### T1: RawSanitizer + SanitizingRawCapture Infra

- crates/data-clients/raw-capture/src/sanitizer.rs 작성: RawSanitizer trait, SanitizedRaw, AllowlistSanitizer
- crates/data-clients/raw-capture/src/capture.rs 작성: SanitizingRawCapture (schema drift tracing::warn! 포함) + DualTierCapture (Tier 2 먼저 호출 fail-fast fan-out)
- Unit test: allowlist drop/retain, schema_hash determinism

### T2: Allowlist 정의

- sources/data_go_kr_building.rs: 7-path allowlist const 정의
- sources/vworld_parcel.rs: V-World allowlist const 정의
- AllowlistSanitizer 인스턴스 생성 함수 export

### T3: Two-tier Vault 마이그레이션 + PgPiiVaultCapture

- migrations/30007_pii_vault.sql: vault table + RLS + KMS column
- migrations/30008_external_data_lineage.sql: lineage columns
- crates/db/src/pii_vault.rs: PgPiiVaultCapture (aws-sdk-kms GenerateDataKey + AES-256-GCM encrypt + INSERT)
- infra/kms-key.ts: Pulumi KMS key 정의

### T4: expires_at NOT NULL + Cleanup Task

- migrations/30010_external_data_expires_constraint.sql: NOT NULL CHECK + index
- services/api/src/cleanup.rs: Tokio interval task
- services/api/src/main.rs: tokio::spawn(run_cleanup_task(...)) 등록

### T5: Building Reader Live Wiring

- services/api/src/main.rs:390-413: NoOp → DataGoKrBuildingReader::new(client, SanitizingRawCapture::new(...)) swap
- has_key 분기 + is_production fail-fast 유지
- /healthz/ready building_reader check 연결

### T6: Vault Access RBAC Admin Endpoint + Audit Log

- migrations/30009_raw_vault_access_log.sql: audit log table
- crates/db/src/access_log.rs: PgVaultAccessLog
- services/api/src/routes/admin/raw_vault.rs: endpoint (ZITADEL role check → vault SELECT → KMS Decrypt → audit INSERT → response)
- services/api/src/main.rs:331-335: admin route 등록

### T7: Integration Test 재작성 + Health Degraded

- services/api/src/state.rs: AppState, app_builder(state) -> Router export
- services/api/tests/sp10_panel_endpoints.rs:29-36, 148-250: app_builder 사용 실 router 호출로 재작성
- services/api/tests/sp10_backend_data_correctness.rs: PII fixture 포함 신규 통합 테스트
- services/api/src/routes/health.rs:45-125: degraded shape 확장

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
| 보유 기간 | expires_at NOT NULL CHECK | migration 30010 |
| 파기 | Tokio cleanup task + audit | integration: cleanup task 호출 |
| 접근 제어 | ZITADEL admin role + RLS | integration: non-admin → 403 |
| 감사 | raw_vault_access_log INSERT | integration: log 레코드 확인 |
| 인프라 코드화 | KMS = infra/kms-key.ts Pulumi | Pulumi preview CI |
