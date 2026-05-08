-- V003_10: parcel_external_data 에 R2 pointer 메타 컬럼 추가 (ADR 0026 step 1).
--
-- ADR 0026 채택 (Bronze raw archive → R2). 본 migration 은 *전적으로 additive* —
-- 기존 schema 보장 약화 0:
--   - 기존 `raw_response jsonb NOT NULL` 그대로 (codex round 5 review fix)
--   - 신규 nullable 컬럼 2개만 추가
--
-- raw_response NOT NULL 제거는 *별도 후속* migration (백필 + 30일 monitoring 후).
-- 그 전에 nullable 화 하면:
--   - 신규 row 가 raw_response = NULL 로 들어와도 schema 차단 X
--   - 일부 코드 path 가 fallback 으로 NULL 적재 가능
--   → 결과적으로 *기존 Bronze 보장 약화* (premature weakening). 그래서 본 migration 은 X.
--
-- 활성 구현체: services/api/src/r2_raw_capture.rs R2RawCapture
-- 폐기 (in-flight): crates/db/src/raw_capture.rs PgRawCapture (main.rs wire 제거됨, 본 commit
-- 시점에 dead code. 별도 정리 PR.)

-- 1) R2 pointer 컬럼 — `bronze/{source}/{yyyy}/{mm}/{dd}/{pnu}_{epoch_ms}.json`.
--    NULL = R2 적재 안 됨 (역사 데이터 / 백필 미완료 / R2 PUT 실패 + 디스크 fallback 적재).
alter table parcel_external_data
    add column r2_object_key varchar(500);

-- 2) raw 사이즈 — quota / cost monitoring 용. NULL 이면 측정 안 됨.
alter table parcel_external_data
    add column raw_byte_size bigint;

-- 3) R2 키 lookup 인덱스 → 별도 migration 30011 (CONCURRENTLY, no-tx 필수).
