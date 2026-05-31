# SP10.5-B Backend Data Correctness Design - Part 02: Vault, Cleanup, Drift, And Production Rules

Parent index: [SP10.5-B Backend Data Correctness Design](./2026-05-08-sp10-5-b-backend-data-correctness-design.md).

## 6. Two-tier Vault 설계

### 6.1 Tier 1 테이블 (기존 + 컬럼 추가)

```sql
-- migrations/30014_external_data_lineage.sql
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
-- migrations/30013_pii_vault.sql
-- ADR 근거: parcel_external_data PK 가 (pnu, source) composite 이고 컬럼 타입이
-- char(19) / varchar(40) 이므로 vault 테이블도 *완전히 동일 타입* 사용 (PostgreSQL FK 는
-- referencing/referenced 컬럼 타입이 정확히 일치해야 함). source CHECK 도 fail-safe 로
-- 별도 추가 (DRY 안 되지만 vault 단독으로도 enum 보장).
CREATE TABLE parcel_external_data_pii_vault (
  id               UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
  pnu              char(19)    NOT NULL,
  source           varchar(40) NOT NULL CHECK (source IN (
      'vworld',                          -- legacy alias (30012 backfill 이전 row 호환용)
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

**source taxonomy 확장 (선행 마이그레이션 30012)** — V-World 다중 endpoint 대비. 기존 `parcel_external_data` CHECK 제약([migrations/30006_parcel_external_data.sql:13-19](../../../migrations/30006_parcel_external_data.sql#L13-L19)) 은 `vworld` 만 허용 → §5.3 의 `vworld_parcel` 사용 위해 30012 가 30013 (vault) 보다 먼저 적용. **번호 배정 근거 (v5 patch)**: 기존 migrations/ 에 30007 (api_health_check), 30008 (user_ci_external_account), 30009 (listing_polygon_denormalize), 30010 (parcel_external_data_r2_pointer), 30011 (parcel_external_data_r2_key_idx) 가 이미 점유되어 SP10.5-B 신규 마이그는 30012~30016 로 일괄 재배정.

```sql
-- migrations/30012_source_taxonomy_expansion.sql
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
-- migrations/30016_external_data_expires_constraint.sql
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
| 11. 보유 기간 | expires_at NOT NULL CHECK | migration 30016 constraint |
| 12. 파기 | Tokio cleanup task + audit log | integration: cleanup task |
| 13. 접근 제어 | ZITADEL admin role + PostgreSQL RLS | integration: non-admin → 403 |
| 14. 감사 | raw_vault_access_log INSERT | integration: log 레코드 확인 |
| 15. 인프라 코드화 | KMS = infra/kms-key.ts Pulumi | Pulumi preview CI |

---
