-- SP9-T2: listing polygon denormalize columns
-- ADR 0016 (PMTiles 100% base layer) — listing 에 polygon 정보 denormalize 하여
-- PostGIS polygon 테이블 없이 행정구역/지목 기반 검색 가능하게 함.
--
-- 기존 schema 와의 관계:
--   parcel_pnu char(19) not null          ← 10001_core_tables.sql:32 에 이미 존재
--   listing_pnu_idx on listing(parcel_pnu) ← 10001_core_tables.sql:61 에 이미 존재
-- 본 마이그레이션은 신규 컬럼 4개 + 신규 인덱스 2개 + parcel_pnu format CHECK 만 추가.
--
-- 신규 컬럼은 모두 nullable. 신규 listing 은 T4 (parcel-lookup hook) 후 채워짐.
-- 기존 row 의 백필은 별도 작업 (월간 재매핑 cron, T6 이후).

alter table listing
    add column admin_code varchar(10),                  -- 시도(2) + 시군구(3) + 읍면동(3) + 리(2) 표준
    add column parcel_land_use_type varchar(20),        -- e.g. 'FactorySite', 'Commercial' (Phase 1 enum 미정 — 추후 CHECK 별도)
    add column parcel_zoning varchar(20),               -- 용도지역 (e.g. '일반공업지역')
    add column parcel_lookup_at timestamptz;            -- 마지막 lookup 시각. polygon 갱신 cron 이 stale 검출

-- 검색 대량화 대비 인덱스 (concurrently 는 sqlx tx 안에서 못 쓰므로 일반 CREATE INDEX 사용)
create index listing_admin_code_idx on listing(admin_code) where admin_code is not null;
create index listing_land_use_type_idx on listing(parcel_land_use_type) where parcel_land_use_type is not null;

-- parcel_pnu 19자리 숫자 format invariant (기존 NOT NULL 컬럼이므로 NULL 분기 불필요)
alter table listing
    add constraint listing_parcel_pnu_format_chk
    check (parcel_pnu ~ '^[0-9]{19}$');
