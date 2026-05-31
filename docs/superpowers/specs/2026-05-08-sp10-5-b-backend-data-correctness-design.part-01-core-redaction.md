# SP10.5-B Backend Data Correctness Design - Part 01: Core Abstractions, Data Flow, And Redaction Policy

Parent index: [SP10.5-B Backend Data Correctness Design](./2026-05-08-sp10-5-b-backend-data-correctness-design.md).

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

기존 `RawCapture` trait 시그니처([crates/data-clients/raw-capture/src/lib.rs:84-97](../../../crates/data-clients/raw-capture/src/lib.rs#L84-L97)) 와 *완전히 일치* 하는 impl 을 채택. 인자 순서 `(pnu, source, raw: &Value, fetched_at: DateTime<Utc>)` 와 `Result<RawCaptureReceipt, RawCaptureError>` 반환 (v5 patch — receipt 도입).

```rust
// crates/data-clients/raw-capture/src/capture.rs (신규)
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use raw_capture_client::{RawCapture, RawCaptureError, RawCaptureReceipt};
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
    ) -> Result<RawCaptureReceipt, RawCaptureError> {
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
use raw_capture_client::{RawCapture, RawCaptureError, RawCaptureReceipt};

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
    ) -> Result<RawCaptureReceipt, RawCaptureError> {
        // Tier 2 먼저: vault INSERT 실패 → 전체 fail-fast, Tier 1 기록 차단.
        self.vault.capture(pnu, source, raw, fetched_at).await?;
        // Tier 1 (sanitized) — Tier 2 성공 후만 실행. caller 가 보는 receipt 는 Tier 1.
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
