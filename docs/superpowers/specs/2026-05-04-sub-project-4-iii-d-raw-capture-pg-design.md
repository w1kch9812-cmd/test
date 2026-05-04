# Sub-project 4-iii-d: RawCapture trait 분리 + PgRawCapture (Spec)

| | |
|---|---|
| 작성일 | 2026-05-04 |
| 상태 | Approved |
| 선행 | SP4-ii (V-World + NoOpRawCapture) |
| 후속 | SP4-iii-a (data.go.kr 건축물대장/토지대장), SP4-iii-b (실거래가), SP4-iii-c (법제처) |
| 닫는 FU | FU 27 (parcel_external_data 테이블 + DB 저장 RawCapture) |

---

## 1. 개요

SP4-ii 가 도입한 `RawCapture` trait 은 현재 vworld-client 안에 살고 있고 default impl 은 `NoOpRawCapture` (`tracing::info!` 로 메타데이터만 발행). 후속 외부 API (data.go.kr / 법제처) 도 같은 trait 을 쓰려면 trait 자체가 별도 crate 이어야 깔끔. 또한 NoOp 만으로는 *raw 보존* 약속을 지키지 못함 — DB 저장 구현체 (`PgRawCapture`) + 마이그레이션 (`parcel_external_data`) 이 필요해요.

본 SP 가 닫는 부채:
- **SSS 3번 (추적성)**: NoOp 단계로 멈춰있던 raw 보존 약속을 DB 저장으로 완성
- **SSS 4번 (안전성)**: 1년 후에도 *원본 그대로 재현* 가능 (감사·분쟁 시 증빙)
- **SSS 1번 (일관성)**: trait 위치 정리 — vworld-client 안 →  `crates/data-clients/raw-capture/` 표준 위치. 후속 API 도 동일 trait 채택

---

## 2. 범위

### 포함
- **신규 lib crate `crates/data-clients/raw-capture/`**:
  - `RawCapture` trait — vworld-client 에서 이동 (정확히 동일 시그니처)
  - `NoOpRawCapture` — 동일 (tracing::info! target=`raw.capture`)
  - `RawCaptureError` enum — `Sink(String)` variant
- **마이그레이션 V003_05**: `parcel_external_data` 테이블
  ```sql
  create table parcel_external_data (
      pnu char(19) not null,
      source varchar(40) not null check (source in (
          'vworld',
          'data_go_kr_building',
          'data_go_kr_land',
          'data_go_kr_realtransaction',
          'korean_law'
      )),
      raw_response jsonb not null,
      fetched_at timestamptz not null,
      expires_at timestamptz,
      primary key (pnu, source)
  );
  create index parcel_external_data_fetched_brin_idx
      on parcel_external_data using brin(fetched_at);
  ```
  - `pnu` 19자리 + `source` 합성 PK — 같은 필지 같은 source 는 단일 row (UPSERT)
  - `expires_at` nullable — 캐시 정책 (TTL) 적용 시 사용 (SP4-iii-a/b 에서 활용)
  - BRIN 인덱스 — `fetched_at` 시계열 쿼리 (오래된 데이터 cleanup)
- **`crates/db/src/raw_capture.rs` 신규**:
  - `PgRawCapture { pool: PgPool }` — `RawCapture` trait 구현체
  - UPSERT 패턴 — 같은 (pnu, source) 재호출 시 raw_response 갱신 + fetched_at 갱신
- **vworld-client 갱신**:
  - `RawCapture` / `NoOpRawCapture` import 를 신규 crate 로 변경
  - `raw_capture.rs` 모듈 제거 (또는 re-export 만)
- **워크스페이스 갱신**:
  - `members` 에 `crates/data-clients/raw-capture` 추가
  - `crates/db/Cargo.toml` 에 `raw-capture-client` dep 추가
- **통합 테스트**:
  - `crates/db/tests/raw_capture_integration.rs`:
    - `pg_raw_capture_inserts_row` — 저장 후 row 1 row + raw_response JSONB round-trip
    - `pg_raw_capture_upserts_on_same_pnu_source` — 재호출 시 row 1 row 유지, fetched_at 갱신
    - `pg_raw_capture_different_sources_separate_rows` — (pnu, vworld) + (pnu, data_go_kr_building) → 2 rows
- **단위 테스트**: NoOpRawCapture (이미 vworld 안에 있음, 그대로 이동)

### 미포함

- raw_response 의 정확한 schema (각 source 별) — *jsonb* 그대로 저장, 파싱은 application layer
- 캐시 만료 (`expires_at` 정책) — SP4-iii-a/b 에서 도입
- raw_response 의 GIN 인덱스 — 조회 빈도 낮음 (감사용), 필요 시 후속
- pruning 워커 (오래된 raw 삭제) — SP9+
- compression — JSONB 가 자체 압축 (TOAST), 추가 압축 불필요

---

## 3. 컴포넌트

### 3.1 `crates/data-clients/raw-capture/Cargo.toml`
```toml
[package]
name = "raw-capture-client"
# ...
[dependencies]
async-trait = { workspace = true }
chrono = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
```

### 3.2 `crates/data-clients/raw-capture/src/lib.rs` (단일 파일 — 작음)
- `RawCapture` trait
- `NoOpRawCapture` impl
- `RawCaptureError` enum

### 3.3 `migrations/30005_parcel_external_data.sql`
- 위 § 2 SQL 그대로

### 3.4 `crates/db/src/raw_capture.rs`
```rust
pub struct PgRawCapture { pool: PgPool }

#[async_trait]
impl RawCapture for PgRawCapture {
    async fn capture(&self, pnu: &str, source: &str, raw: &Value, fetched_at: DateTime<Utc>)
        -> Result<(), RawCaptureError>
    {
        sqlx::query(r#"
            insert into parcel_external_data (pnu, source, raw_response, fetched_at)
            values ($1, $2, $3, $4)
            on conflict (pnu, source) do update set
                raw_response = excluded.raw_response,
                fetched_at = excluded.fetched_at
        "#)
        .bind(pnu).bind(source).bind(raw).bind(fetched_at)
        .execute(&self.pool).await
        .map_err(|e| RawCaptureError::Sink(e.to_string()))?;
        Ok(())
    }
}
```

### 3.5 vworld-client 갱신
- `crates/data-clients/vworld/src/raw_capture.rs` 삭제
- `crates/data-clients/vworld/src/lib.rs` 의 `pub use` 갱신:
  ```rust
  pub use raw_capture_client::{NoOpRawCapture, RawCapture, RawCaptureError};
  ```
- `crates/data-clients/vworld/Cargo.toml` deps 에 `raw-capture-client` 추가

---

## 4. 검증 기준 (DoD)

1. `crates/data-clients/raw-capture/` 신규
2. 마이그레이션 V003_05 (parcel_external_data 테이블 + BRIN 인덱스)
3. `crates/db/src/raw_capture.rs` PgRawCapture impl
4. vworld-client 가 신규 crate 의존
5. 워크스페이스 `members` + db `Cargo.toml` 갱신
6. 통합 테스트 3개 신규
7. 3 CI workflow 그린
8. clippy `--all-targets -- -D warnings` 통과 (FU 34 강화 후 첫 SP)
9. `RawCapture` trait 동작 변경 0 — vworld-client 의 호출자는 기존 동작 그대로

---

## 5. SSS 7 기둥 매핑

| 기둥 | 적용 |
|---|---|
| 1 일관성 | RawCapture trait 표준 위치 — 후속 API 도 같은 trait 채택. 같은 마이그레이션 테이블 |
| 3 추적성 | NoOp → DB 저장 — *진짜* raw 보존. 1년 후 재현 가능 |
| 4 안전성 | UPSERT idempotent. JSONB schema 보존. parameterized SQL |
| 6 SSOT | `parcel_external_data.raw_response` = 외부 API 응답의 SSOT. 파싱된 도메인 모델은 사본 |

---

## 6. Follow-up

- **FU 35**: cleanup 워커 (`fetched_at < now() - X` row 삭제) — SP9+
- **FU 36**: `expires_at` 정책 통일 — 각 source 별 TTL 적용 (SP4-iii-a/b)
- **FU 37**: GIN 인덱스 (`raw_response` 검색 필요 시)
- **FU 38**: pgvector 통합 (raw_response 의 텍스트 본문 임베딩) — Phase 3+
