-- sqlx:no-tx
-- V003_11: parcel_external_data.r2_object_key partial 인덱스 (concurrent, ADR 0026 step 2).
--
-- migrations/README.md § 인덱스 추가 정책: production 에서 `CREATE INDEX` 는
-- ACCESS EXCLUSIVE LOCK → write 차단. `CONCURRENTLY` 로 회피.
-- `CONCURRENTLY` 는 트랜잭션 안 불가 → 첫 줄 `-- sqlx:no-tx` 마커로 sqlx 가
-- 본 파일을 트랜잭션 wrap 안 하도록.
--
-- partial index — `r2_object_key IS NOT NULL` 만 (백필 전 NULL row 다수 예상).
-- 인덱스 사이즈 절약 + lookup 효율 동시.

create index concurrently if not exists parcel_external_data_r2_key_idx
    on parcel_external_data (r2_object_key)
    where r2_object_key is not null;
