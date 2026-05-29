# SQL 컨벤션 (PostgreSQL + PostGIS)

## 1. 도구

- **DB**: PostgreSQL 17 + PostGIS 3.5+ + pgvector (Phase 3+)
- **마이그레이션**: sqlx migrate (Atlas 병용 검토 — sub-project 2)
- **lint·format**: sqlfluff (PostgreSQL 방언, sub-project 2+)
- **타입 검증**: SQLx compile-time (Rust)

## 2. 네이밍

- 테이블: `snake_case` 단수 (`listing`, NOT `listings`)
- 컬럼: `snake_case` (`created_at`, `owner_id`)
- 인덱스: `<table>_<columns>_<type>_idx` (`listing_pnu_btree_idx`, `parcel_geom_gist_idx`)
- 제약: `<table>_<column>_<rule>` (`listing_price_krw_positive_check`)
- FK: `<table>_<column>_fkey`
- 시퀀스: `<table>_<column>_seq`

## 3. 키워드 케이스

- 키워드: lowercase (`select`, `from`, `where`, `join`)
- 함수: lowercase (`now()`, `coalesce()`, `st_distance()`)
- 식별자: 따옴표 없이 (예약어 충돌 시만 `"order"`)

## 4. 좌표계 (SRID) 강제

- 모든 geometry 컬럼에 SRID 명시: `geometry(Polygon, 4326)`
- 저장: 항상 4326 (WGS84)
- 연산: 5179 (UTM-K) 변환 후
- 타일: 3857 (Web Mercator)

```sql
create table parcel (
    pnu char(19) primary key,
    -- ✅ SRID 명시
    geom geometry(Polygon, 4326) not null,
    -- ❌ SRID 없음 (CI 차단)
    -- geom geometry not null,
    area_m2 numeric(12, 2),
    created_at timestamptz not null default now()
);

-- ✅ GIST 인덱스 (공간)
create index parcel_geom_gist_idx on parcel using gist (geom);
```

## 5. 인덱스 정책

| 종류 | 용도 |
|------|------|
| **B-Tree** | 일반 쿼리, FK, ORDER BY |
| **GIST** | PostGIS 공간 데이터 |
| **GIN** | JSONB, 배열, 전문 검색 |
| **BRIN** | 시계열 (audit_log, real_transaction) |
| **HNSW** | pgvector 임베딩 (Phase 3+) |

## 6. 마이그레이션 안전성

- NOT NULL 추가 → DEFAULT 동반 + 별도 단계로 분리:
  1. NULL 허용 컬럼 추가
  2. 백필 (`update ... set ... where ... is null`)
  3. NOT NULL 제약 추가
- 큰 테이블 ALTER → `pg-osc` (sub-project 8+)
- 인덱스 생성 → `concurrently`
- 컬럼 삭제 → 사용 코드 제거 후 별도 PR (3단계)

## 7. 트랜잭션 정책

- Repository 메서드 = 한 트랜잭션
- Cross-aggregate 쓰기 = 도메인 이벤트 + Outbox 패턴 (직접 X)
- 격리 수준: 기본 READ COMMITTED. 동시성 핫스팟은 SERIALIZABLE + 재시도

## 8. Optimistic Locking

```sql
create table listing (
    id char(30) primary key,
    -- 동시 수정 차단
    version bigint not null default 1,
    -- ...
);

-- update 시 version 검사 + 증가
update listing
set price_krw = $1, version = version + 1
where id = $2 and version = $3;
-- 영향 받은 row 0 = conflict
```

## 9. Soft Delete

- 도메인 테이블: `deleted_at TIMESTAMPTZ NULL` 컬럼
- 쿼리: 항상 `where deleted_at is null` (또는 별도 view `_active`)
- audit_log: hard delete 불가, 무한 보존

## 10. 감사 컬럼 (모든 도메인 테이블)

```sql
created_at timestamptz not null default now(),
created_by char(30) not null references "user"(id),
updated_at timestamptz not null default now(),
updated_by char(30) not null references "user"(id),
version bigint not null default 1,
deleted_at timestamptz null
```

## 11. Gongzzang-owned external response archive

Catalog source raw lineage is Platform Core-owned. Gongzzang must not add raw
tables for parcel, building, industrial complex, manufacturer, public/reference
spatial layers, or Catalog API drift monitoring.

For a future Gongzzang-owned external adapter approved by ADR, archive tables use
an explicit owner-specific name:

```sql
create table listing_external_data (
    listing_id char(30) primary key references listing(id),
    source varchar(40) not null,  -- e.g. 'korean_law', 'nice_identity'
    raw_response jsonb not null,
    fetched_at timestamptz not null,
    expires_at timestamptz not null
);

create index on listing_external_data using gin (raw_response);
```

Raw retention = audit, replay, and dispute evidence for the owning Gongzzang
adapter only.

## 12. 금지

- ❌ SRID 없는 geometry 컬럼
- ❌ `select *` (필요한 컬럼만 명시)
- ❌ `varchar` 길이 무제한 (`text`로 명확)
- ❌ Cross-aggregate transaction (도메인 이벤트로 우회)
- ❌ Hard delete (audit log 외)
- ❌ `st_distance` 단독 사용 (성능 — `st_dwithin` + GIST)
