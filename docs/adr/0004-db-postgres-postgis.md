# ADR-0004: DB — PostgreSQL 17 + PostGIS

| | |
|---|---|
| 작성일 | 2026-05-01 |
| 상태 | Accepted |
| 결정자 | 운영자 |

## 컨텍스트

산업용 부동산 = **공간 데이터 중심** (필지 폴리곤, 건축물 좌표, 산업단지 영역). 한국 공공 API(V-World)가 PostGIS 친화 좌표계 사용. 추후 임베딩(pgvector, Phase 3+)도 같은 DB에 저장 → 단일 DB SSOT.

## 결정

- **DB**: PostgreSQL 17
- **공간 확장**: PostGIS 3.5+
- **벡터 확장 (Phase 3+)**: pgvector
- **ORM/SQL**: SQLx (compile-time SQL 검증)
- **마이그레이션**: sqlx migrate (또는 Atlas 병용 — sub-project 2에서 결정)
- **좌표계 정책**: 저장 4326, 연산 5179, 타일 3857

## 대안

- **CockroachDB / YugabyteDB**: 분산, 그러나 PostGIS 약함, 비용 큼
- **MySQL + GeoSpatial**: PostGIS 대비 기능 부족
- **MongoDB + Geospatial Index**: 정규화 부족, 트랜잭션 약함
- **Prisma ORM**: PostGIS 까다로움, raw SQL 자유도 낮음. 우리는 SQLx 직접
- **Diesel ORM**: async 부진, PostGIS 지원 약함

## 결과

- 긍정: PostGIS 30년 검증, GIST 인덱스 압도적 성능, 트랜잭션 강함, 단일 DB로 OLTP+공간+벡터 통합, 한국 시장 표준 (V-World/data.go.kr 호환)
- 부정: 단일 DB의 수평 확장 한계 (일정 규모 후 read replica + sharding 필요), Multi-AZ Multi-region 비용
- 영향 영역: `crates/db/`, `db/migration/`, `crates/geo/`, `crates/embedding/`

## 재검토 트리거

- 단일 인스턴스가 r7g.4xlarge 한계 도달 시 → sharding (Citus) 또는 분산 DB
- 글로벌 진출 시 active-active 멀티 리전 → Aurora Postgres 또는 CockroachDB 재평가
- 벡터 검색 부하가 OLTP 영향 시 → 별도 Qdrant/Milvus 분리

## 참조

- → @docs/data/postgres.md (작성 예정)
- → @docs/data/postgis.md
- → @docs/data-sources/v-world.md
