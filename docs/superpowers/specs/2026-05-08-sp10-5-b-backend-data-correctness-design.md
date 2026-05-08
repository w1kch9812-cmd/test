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

기존 `RawCapture` trait 시그니처([crates/data-clients/raw-capture/src/lib.rs:46-52](../../../crates/data-clients/raw-capture/src/lib.rs#L46-L52)) 와 *완전히 일치* 하는 impl 을 채택. 인자 순서 `(pnu, source, raw: &Value, fetched_at: DateTime<Utc>)` 와 `Result<(), RawCaptureError>` 반환을 그대로 따른다.

```rust
// crates/data-clients/raw-capture/src/capture.rs (신규)
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use raw_capture_client::{RawCapture, RawCaptureError};
use std::sync::Arc;

pub struct SanitizingRawCapture<C: RawCapture> {
    inner: C,
    sanitizer: Arc<dyn RawSanitizer>,
}

#[async_trait]
impl<C: RawCapture + Send + Sync> RawCapture for SanitizingRawCapture<C> {
    async fn capture(
        &self,
        pnu: &str,
        source: &str,
        raw: &serde_json::Value,
        fetched_at: DateTime<Utc>,
    ) -> Result<(), RawCaptureError> {
        let sanitized = self.sanitizer.sanitize(raw);
        if sanitized.dropped_count > 0 {
            tracing::warn!(
                target: "raw.capture.schema_drift",
                pnu = %pnu,
                source = %source,
                schema_hash = %sanitized.schema_hash,
                dropped_count = sanitized.dropped_count,
                "raw_response sanitizer dropped unknown fields"
            );
        }
        // SanitizedRaw.value 는 owned. inner trait 이 borrow 받으므로 새 binding 으로 넘김.
        let sanitized_value = sanitized.value;
        self.inner
            .capture(pnu, source, &sanitized_value, fetched_at)
            .await
    }
}
```

**self-review 노트** — `PgRawCapture`([crates/db/src/raw_capture.rs:14-56](../../../crates/db/src/raw_capture.rs#L14-L56)) INSERT 시그니처는 현재 `(pnu, source, raw, fetched_at)` 4-인자만 받고 `schema_hash` / `sanitizer_version` 컬럼은 모른다. v1 에서 두 가지 길:

- 길 A (선택): `SanitizingRawCapture` 가 `schema_hash` / `sanitizer_version` 을 *tracing/telemetry 메타데이터* 로만 발행하고 PgRawCapture INSERT 자체는 4-인자 그대로. DB 의 신규 컬럼은 별도 `crates/db/src/raw_capture.rs` 시그니처 확장이 필요 (T3 에 합류).
- 길 B: `RawCapture` trait 자체에 `metadata: SanitizerMetadata` optional 인자를 추가. trait 변경은 모든 impl 영향 → 비용 큼.

**길 A 채택**. T3 에서 `PgRawCapture` 를 `(pnu, source, raw, fetched_at, sanitizer_version, schema_hash)` 6-인자로 *별도 신규 메서드 또는 신규 wrapper struct* 로 확장.

### 3.4 Two-tier Sink

- **Tier 1** (PgRawCapture): parcel_external_data.raw_response — 정제된 JSON
- **Tier 2** (PgPiiVaultCapture): parcel_external_data_pii_vault.ciphertext_blob — KMS 암호화 full-raw

### 3.5 DualTierCapture (Fan-out Composer)

`SanitizingRawCapture` 는 sanitization 단독 책임(정제 후 Tier 1 sink 에 전달). Tier 1 + Tier 2 fan-out 은 `DualTierCapture` 가 담당한다.
Tier 2 (vault) 를 먼저 호출하여 fail-fast 를 보장한다.

```rust
// crates/data-clients/raw-capture/src/capture.rs
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use raw_capture_client::{RawCapture, RawCaptureError};

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
    async fn capture(
        &self,
        pnu: &str,
        source: &str,
        raw: &serde_json::Value,
        fetched_at: DateTime<Utc>,
    ) -> Result<(), RawCaptureError> {
        // Tier 2 먼저: vault INSERT 실패 → 전체 fail-fast, Tier 1 기록 차단.
        self.vault.capture(pnu, source, raw, fetched_at).await?;
        // Tier 1: sanitized 경로 (raw 는 inner sanitizer 가 폐기/정제).
        self.sanitized.capture(pnu, source, raw, fetched_at).await
    }
}
```

`services/api/src/main.rs` (210-221, 390-413 wiring 영향) 에서 — 현재 `PgRawCapture` 단독 주입을 `DualTierCapture` 합성체로 교체:

```rust
let sanitizer = AllowlistSanitizer::for_source("data_go_kr_building"); // 또는 vworld_parcel
let pg_raw_capture = PgRawCapture::new(pool.clone());
let pii_vault       = PgPiiVaultCapture::new(pool.clone(), kms_client.clone());
let capture: Arc<dyn RawCapture> = Arc::new(DualTierCapture {
    sanitized: SanitizingRawCapture::new(pg_raw_capture, Arc::new(sanitizer)),
    vault: pii_vault,
});
```

기존 `Arc::new(PgRawCapture::new(pool.clone()))` 직접 주입(T7 audit-fix `b784e76` 에서 도입) 은 위 합성체로 대체된다.

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

근거 SSOT: [docs/data-sources/v-world.md](../../data-sources/v-world.md) 의 `LP_PA_CBND_BUBUN` properties 표 + 실 fixture [real_parcel_boundary_gangnam_yeoksam_737.json](../../../crates/data-clients/vworld/tests/fixtures/real_parcel_boundary_gangnam_yeoksam_737.json). 공시지가 (`jiga`) 는 PIPA 상 *공개 행정정보* (개인정보 아님) 이며 패널 핵심 표시 필드 → allowlist 포함.

| 경로 | 내용 | 매핑 |
|---|---|---|
| /response/result/featureCollection/features/*/geometry | 지적 폴리곤 좌표 (EPSG:4326) | `Parcel.geom: MultiPolygonSrid` |
| /response/result/featureCollection/features/*/properties/pnu | 필지 고유번호 (19자리) | `Parcel.pnu` |
| /response/result/featureCollection/features/*/properties/jibun | 지번 ("737 대" 형식) | `Parcel.jibun_address` + `Parcel.land_use_type` |
| /response/result/featureCollection/features/*/properties/bonbun | 본번 (4자리) | (예비; 향후 사용) |
| /response/result/featureCollection/features/*/properties/bubun | 부번 (4자리) | (예비; 향후 사용) |
| /response/result/featureCollection/features/*/properties/addr | 풀주소 (지번주소) | `Parcel.jibun_address` (우선) |
| /response/result/featureCollection/features/*/properties/jiga | 공시지가 (₩/m²) | `Parcel.official_land_price_per_m2` |
| /response/result/featureCollection/features/*/properties/gosi_year | 공시 연도 | `Parcel.gosi_year_month` |
| /response/result/featureCollection/features/*/properties/gosi_month | 공시 월 | `Parcel.gosi_year_month` |
| /response/service/* | service 메타 (operation, version, time) | drift 진단 |
| /response/status | OK / NOT_FOUND / ERROR envelope | status 분기 |
| /response/error/* | code / text (ERROR 케이스) | `ParseError::VWorldApi` 매핑 |
| /response/record/* | total / current | pagination 진단 |
| /response/page/* | total / current / size | pagination 진단 |

**비허용 (자동 폐기)**: spec/fixture 에 등장하지 않는 모든 추가 필드. V-World provider 가 future schema 확장 시 schema_hash drift alert (§8.3) 가 P2/P1 로 발화 → ADR 후 allowlist 갱신.

이전 v3 의 `bchk` 항목은 V-World 응답에 *존재하지 않는 필드* — fixture/docs 와 mismatch 였으므로 v4 에서 제거. `bonbun + bubun` 으로 대체.

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
-- migrations/30012_external_data_lineage.sql
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
-- migrations/30011_pii_vault.sql
-- ADR 근거: parcel_external_data PK 가 (pnu, source) composite 이고 컬럼 타입이
-- char(19) / varchar(40) 이므로 vault 테이블도 *완전히 동일 타입* 사용 (PostgreSQL FK 는
-- referencing/referenced 컬럼 타입이 정확히 일치해야 함). source CHECK 도 fail-safe 로
-- 별도 추가 (DRY 안 되지만 vault 단독으로도 enum 보장).
CREATE TABLE parcel_external_data_pii_vault (
  id               UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
  pnu              char(19)    NOT NULL,
  source           varchar(40) NOT NULL CHECK (source IN (
      'vworld',                          -- legacy alias (30010 backfill 이전 row 호환용)
      'vworld_parcel',                   -- 지적 폴리곤 endpoint
      'data_go_kr_building',
      'data_go_kr_land',
      'data_go_kr_realtransaction',
      'korean_law'
  )),
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

**source taxonomy 확장 (선행 마이그레이션 30010)** — V-World 다중 endpoint 대비. 기존 `parcel_external_data` CHECK 제약([migrations/30006_parcel_external_data.sql:13-19](../../../migrations/30006_parcel_external_data.sql#L13-L19)) 은 `vworld` 만 허용 → §5.3 의 `vworld_parcel` 사용 위해 30010 이 30011 (vault) 보다 먼저 적용. **번호 배정 근거**: 기존 migrations/ 에 30007 (api_health_check), 30008 (user_ci_external_account), 30009 (listing_polygon_denormalize) 가 이미 점유되어 충돌 회피 필요 → SP10.5-B 신규 마이그는 30010~30014 로 일괄 배정.

```sql
-- migrations/30010_source_taxonomy_expansion.sql
-- V003_07a: source taxonomy expansion — V-World 다중 endpoint 대비.
-- 'vworld' 는 legacy alias 로 유지하되, 신규 INSERT 는 구체 endpoint name 사용.
ALTER TABLE parcel_external_data DROP CONSTRAINT parcel_external_data_source_check;
ALTER TABLE parcel_external_data ADD CONSTRAINT parcel_external_data_source_check
    CHECK (source IN (
        'vworld',                          -- legacy
        'vworld_parcel',                   -- 지적 폴리곤 endpoint
        'data_go_kr_building',
        'data_go_kr_land',
        'data_go_kr_realtransaction',
        'korean_law'
    ));

-- 기존 'vworld' row 들을 backfill (legacy alias 정리)
UPDATE parcel_external_data SET source = 'vworld_parcel' WHERE source = 'vworld';
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

근거 SSOT: [docs/data-sources/data-go-kr.md](../../data-sources/data-go-kr.md) 의 "캐시 정책" 표 + V-World 운영 정책. raw_response 의 `expires_at` 은 *cache hit TTL* 과 동일 의미로 사용 — 만료 후 재호출/재정제 가 cleanup task 트리거.

| source | TTL | 근거 |
|---|---|---|
| data_go_kr_building | 30일 | docs/data-sources/data-go-kr.md:60 ("건축물대장 변동 빈도 낮음") SSOT 일치 |
| data_go_kr_land | 30일 | docs/data-sources/data-go-kr.md:61 — 토지대장 동일 정책 (FU; v1 미사용) |
| vworld_parcel | 30일 | V-World 캐시 운영 정책 (Redis 24h cache + raw 30일 retention 분리; raw 는 audit/재현) |
| korean_law | 90일 | 법령 변동 빈도 낮음 (FU; v1 미사용) |

**raw vs cache 분리**: PIPA 21조 "보유 기간" 관점에서 raw_response 의 `expires_at` 은 *목적 달성 후 파기* 기준. 패널 표시 + drift 진단 목적에 30일 충분. 이전 v3 의 90일은 docs SSOT 와 mismatch → v4 에서 30일로 정합.

### 7.2 NOT NULL CHECK 제약 (ADR 0026)

```sql
-- migrations/30014_external_data_expires_constraint.sql
-- 기존 NULL 레코드 backfill 선행 (NOT NULL 제약 추가 전)
UPDATE parcel_external_data
   SET expires_at = fetched_at + INTERVAL '90 days'  -- 가장 긴 TTL 기준 보수적 backfill
 WHERE expires_at IS NULL;

ALTER TABLE parcel_external_data
  ALTER COLUMN expires_at SET NOT NULL;

-- parcel_external_data 의 시점 컬럼은 fetched_at (captured_at 아님 — DDL 30006 참조).
ALTER TABLE parcel_external_data
  ADD CONSTRAINT check_expires_future
  CHECK (expires_at > fetched_at);

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
| crates/data-clients/raw-capture/src/lib.rs | 40-89 | RawCapture trait 시그니처 *유지* (`pnu, source, raw: &Value, fetched_at` / `Result<(), RawCaptureError>`). 신규 `SanitizingRawCapture` + `DualTierCapture` 를 동일 모듈 또는 `capture.rs` 신규 파일로 추가 후 `pub use` re-export |
| crates/db/src/raw_capture.rs | 14-56 | 기존 `PgRawCapture::insert` 4-인자 유지. 신규 메서드 또는 wrapper 로 `(pnu, source, raw, fetched_at, sanitizer_version, schema_hash, license, api_version)` 8-인자 INSERT path 추가 (T3 — lineage 컬럼 30008 대응) |
| crates/data-clients/data-go-kr/src/building_register/reader.rs | 117-128 | 변경 없음 (wrapper transparent — 기존 `raw_capture.capture(pnu, source, &raw, now)` 호출이 이미 신규 trait 시그니처 일치) |
| crates/data-clients/data-go-kr/src/building_register/reader.rs | 37 | `RAW_CAPTURE_SOURCE = "data_go_kr_building"` 유지 |
| crates/data-clients/vworld/src/reader.rs | 35-96 | `VWorldParcelReader::new(client, raw_capture)` 시그니처 *유지*. 단 호출자가 주입하는 `Arc<dyn RawCapture>` 가 `DualTierCapture` 합성체로 교체됨 |
| crates/data-clients/vworld/src/reader.rs | **71** | `.capture(pnu.as_str(), "vworld", &raw, now)` 의 source string literal `"vworld"` → `"vworld_parcel"` 변경 필수. 30010 taxonomy 마이그레이션과 동시 적용 (마이그레이션은 backfill, 코드 변경은 신규 INSERT 가 정확한 enum 사용). 권장: `pub const RAW_CAPTURE_SOURCE: &str = "vworld_parcel";` 신규 const 도입 후 참조 (data-go-kr building reader 의 `RAW_CAPTURE_SOURCE` 패턴 따라) |
| crates/data-clients/raw-capture/src/lib.rs | 7-18 | doc comment 의 예시 `capture.capture("...", "vworld", ...)` 를 `"vworld_parcel"` 로 업데이트 (legacy alias 명시) |
| services/api/src/main.rs | 210-221 | V-World capture wire 가 현재 `Arc::new(PgRawCapture::new(pool.clone()))` 직접 주입 (b784e76) → `DualTierCapture { sanitized: SanitizingRawCapture::new(PgRawCapture, AllowlistSanitizer::for_source("vworld_parcel")), vault: PgPiiVaultCapture::new(pool, kms) }` 합성체로 교체 |
| services/api/src/main.rs | 331-335 | `/healthz/ready` 라우트의 핸들러를 신규 `ReadinessResponse` 반환형으로 교체. AppState 에 `building_reader_status` / `vault_kms_status` 핸들 추가 |
| services/api/src/main.rs | 390-413 | `Arc::new(NoOpBuildingRegisterReader)` → `Arc::new(DataGoKrBuildingReader::new(client, dual_tier_capture.clone()))` swap. `has_key` 분기 + `is_production` 시 `fail_fast_production` 패턴 (현재 코드와 동일) 유지. 키 없고 production 이면 부팅 panic (변경 0). 키 없고 non-production 이면 NoOp 유지 + `building_reader: degraded` 표시 |
| services/api/src/routes/health.rs | 45-125 | 기존 `HealthResponse { status: String }` 유지 (liveness 용). 신규 `ReadinessResponse { status: String, checks: ReadinessChecks }` + `ReadinessChecks { db, redis, building_reader, vault_kms: String }` 정의. readiness 핸들러를 새 응답형으로 교체 |
| services/api/src/main.rs | (신규 위치) | services/api 가 현재 `app_builder(state) -> Router` 같은 factory 를 *export 하지 않음*. T7 에서 `services/api/src/lib.rs` (또는 `state.rs`) 에 `pub fn app_router(state: AppState) -> Router` 또는 `pub fn build_app(state) -> Router` 추가 — 실 통합 테스트가 main.rs 의 wiring 을 그대로 호출 가능하도록 |
| services/api/tests/sp10_panel_endpoints.rs | 29-36, 148-250 | 현재 `spawn_test_app()` 로컬 헬퍼 (핸들러 부분 재구현) → `app_router(test_state)` 직접 호출 + `axum_test::TestServer::new(app_router(...))` 패턴으로 재작성 |
| migrations/30006_parcel_external_data.sql | (선행 마이그레이션) | 30010 이 이 테이블의 source CHECK 를 확장 (변경 없음 — 30006 자체는 그대로) |

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
  30010_source_taxonomy_expansion.sql   (vworld → vworld_parcel rename, CHECK 확장)
  30011_pii_vault.sql                    (Tier 2 vault + RLS + composite FK)
  30012_external_data_lineage.sql        (license, api_version, sanitizer_version, schema_hash)
  30013_raw_vault_access_log.sql         (admin 조회 audit log)
  30014_external_data_expires_constraint.sql  (expires_at NOT NULL + CHECK + index)

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

- migrations/30010_source_taxonomy_expansion.sql: `parcel_external_data` CHECK 확장 (`vworld_parcel` 추가) + 기존 `vworld` row 를 `vworld_parcel` 로 backfill UPDATE
- **crates/data-clients/vworld/src/reader.rs:71**: source literal `"vworld"` → `"vworld_parcel"` 변경 (또는 신규 `pub const RAW_CAPTURE_SOURCE: &str = "vworld_parcel"` 도입 후 참조). data.go.kr building reader 패턴 따라 const SSOT 화 권장. **마이그레이션과 코드 변경이 동일 PR 에 묶여야 함** — 마이그만 적용되고 코드가 'vworld' 그대로 INSERT 시 backfill 직후 다시 'vworld' row 가 생김
- crates/data-clients/raw-capture/src/lib.rs:7-18 doc comment 예시 업데이트 (`vworld` → `vworld_parcel`)
- sources/data_go_kr_building.rs: 7-path allowlist const 정의
- sources/vworld_parcel.rs: V-World allowlist const 정의
- AllowlistSanitizer::for_source(&str) 팩토리 함수 export

### T3: Two-tier Vault 마이그레이션 + PgPiiVaultCapture + Lineage 컬럼

- migrations/30011_pii_vault.sql: vault table + RLS + composite FK (pnu char(19), source varchar(40))
- migrations/30012_external_data_lineage.sql: `license`, `api_version`, `sanitizer_version`, `schema_hash` 컬럼 추가 + legacy backfill (`schema_hash = 'legacy:' || md5(...)`, `sanitizer_version = 0`)
- crates/db/src/raw_capture.rs: `PgRawCapture` 에 lineage-aware 신규 메서드 또는 wrapper struct (8-인자 INSERT)
- crates/db/src/pii_vault.rs: PgPiiVaultCapture (`aws-sdk-kms` GenerateDataKey → AES-256-GCM encrypt full raw → ciphertext_blob INSERT). `RawCapture` trait impl 시그니처는 `(pnu, source, &Value, fetched_at)` 그대로
- infra/kms-key.ts: Pulumi `aws.kms.Key` ("pii-vault-key", `enableKeyRotation: true`, `deletionWindowInDays: 30`)
- Cargo.toml 의존성: `sha2` (raw-capture), `aws-sdk-kms` + `aes-gcm` (db)

### T4: expires_at NOT NULL + Cleanup Task

- migrations/30014_external_data_expires_constraint.sql: NOT NULL CHECK + index
- services/api/src/cleanup.rs: Tokio interval task
- services/api/src/main.rs: tokio::spawn(run_cleanup_task(...)) 등록

### T5: Building Reader Live Wiring

- services/api/src/main.rs:390-413: 기존 `Arc::new(NoOpBuildingRegisterReader)` → `Arc::new(DataGoKrBuildingReader::new(client, dual_tier_capture.clone()))` swap. 단 `has_key` 분기와 `if !has_key && is_production { fail_fast_production(...) }` 패턴은 *현재 코드 그대로 유지* — production 에서 `DATA_GO_KR_API_KEY` 미설정 시 부팅 panic. dev/staging 에서 키 없으면 NoOp 유지하되 AppState 의 `building_reader_status = "degraded"` 로 표시
- /healthz/ready 핸들러가 AppState 의 `building_reader_status` 를 읽어 응답 직렬화 (T7 의 `ReadinessResponse` 와 연결)

### T6: Vault Access RBAC Admin Endpoint + Audit Log

- migrations/30013_raw_vault_access_log.sql: audit log table
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
| 보유 기간 | expires_at NOT NULL CHECK | migration 30010 |
| 파기 | Tokio cleanup task + audit | integration: cleanup task 호출 |
| 접근 제어 | ZITADEL admin role + RLS | integration: non-admin → 403 |
| 감사 | raw_vault_access_log INSERT | integration: log 레코드 확인 |
| 인프라 코드화 | KMS = infra/kms-key.ts Pulumi | Pulumi preview CI |
