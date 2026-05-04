-- V003_05: parcel_external_data 테이블 — 외부 API raw_response 보존 (SP4-iii-d).
--
-- 모든 외부 API 응답을 raw 그대로 보존해 1년 후에도 *원본 그대로 재현* 가능
-- (감사·재현·분쟁 시 증빙). (pnu, source) 합성 PK — 같은 필지 같은 source 는
-- 단일 row (UPSERT). source 는 enum-like CHECK 제약 — 후속 SP4-iii-a/b/c 에서
-- 추가 source 시 ALTER TABLE.
--
-- 도메인 측 trait: crates/data-clients/raw-capture/src/lib.rs RawCapture
-- DB 구현체: crates/db/src/raw_capture.rs PgRawCapture

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

-- 시계열 쿼리 (cleanup / TTL 점검) 용 BRIN 인덱스 — append-mostly 패턴에 최적.
create index parcel_external_data_fetched_brin_idx
    on parcel_external_data using brin(fetched_at);
