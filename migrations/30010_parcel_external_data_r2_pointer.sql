-- V003_10: parcel_external_data → R2 pointer table 화 (ADR 0026).
--
-- ADR 0026 채택 후 raw 응답은 R2 (S3-호환 객체 저장소) 에 영구 저장:
--   bronze/{source}/{yyyy}/{mm}/{dd}/{pnu}_{epoch_ms}.json
--
-- 본 테이블은 *raw 보유* 가 아닌 *R2 pointer + 메타* 로 변경:
--   - 기존 `raw_response jsonb` 컬럼 → nullable 화 (forward-only, drop 은 후속)
--   - 신규 `r2_object_key varchar(500)` 추가 — R2 객체 키 reference
--   - 신규 `raw_byte_size bigint` — R2 PUT 후 누적 사이즈 (감사 / quota 계산)
--   - 기존 (pnu, source) PK → (pnu, source, fetched_at) 합성 PK 로 확장 *예정*
--     (UPSERT 의미 손실 방지 위해 본 migration 에서는 미실행. 별도 ADR 필요)
--
-- forward-only 정책 (migrations/README.md): 본 migration 는 jsonb 컬럼을 *비활성*
-- 시킬 뿐, drop 안 함. 기존 row 는 그대로 보존되어 historical query 가능.
--
-- 후속 작업 (별도 migration):
--   - 기존 raw_response 데이터를 R2 로 백필 (운영 batch)
--   - 백필 완료 + monitoring 30일 무사고 → raw_response 컬럼 drop
--
-- 도메인 측 trait: crates/data-clients/raw-capture/src/lib.rs RawCapture
-- 활성 구현체: services/api/src/r2_raw_capture.rs R2RawCapture (ADR 0026)
-- 폐기 구현체: crates/db/src/raw_capture.rs PgRawCapture (dead code, 후속 정리)

-- 1) raw_response NOT NULL 제거 — R2 가 SSOT 이므로 jsonb 적재 의무 X.
alter table parcel_external_data
    alter column raw_response drop not null;

-- 2) R2 pointer 컬럼 — `bronze/{source}/{yyyy}/{mm}/{dd}/{pnu}_{epoch_ms}.json`.
--    NULL 이면 *아직 R2 에 저장 안 됨* (백필 미완료 또는 R2 PUT 실패 + 디스크 fallback 적재).
alter table parcel_external_data
    add column r2_object_key varchar(500);

-- 3) raw 사이즈 — quota / cost monitoring 용. NULL 이면 측정 안 됨.
alter table parcel_external_data
    add column raw_byte_size bigint;

-- 4) R2 키 lookup 인덱스 — operator 가 키로 row 역추적할 때.
create index parcel_external_data_r2_key_idx
    on parcel_external_data (r2_object_key)
    where r2_object_key is not null;
