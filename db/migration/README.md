# db/migration/

PostgreSQL 마이그레이션 (sqlx migrate, sub-project 2+).

## 형식
- `V<NNN>__<description>.sql` (sqlx migrate 표준)
- 시간 순서 = 의미 (숫자 prefix 유지)
- 한 마이그레이션 = 한 PR (불가분)
- DOWN 스크립트 = 권장 (롤백 시)

## 첫 마이그레이션 (sub-project 2)
- `V001__init.sql` — extensions (postgis, pgvector future), 기본 schema
- `V002__core_user.sql` — User 테이블
- `V003__core_listing.sql` — Listing
- `V004__core_parcel.sql` — Parcel + PostGIS 인덱스
- ...

## 정책
- NOT NULL 추가 = 별도 단계 (NULL 허용 → 백필 → NOT NULL)
- 큰 테이블 ALTER = `pg-osc` (Phase 3+)
- 인덱스 = `concurrently`
- 컬럼 삭제 = 3단계 (deprecated → 사용 코드 제거 → 실제 삭제)
- 모든 마이그레이션 = sqlx 자동 검증 + Atlas drift 검증 (sub-project 2)

## 환경별
- `dev/` (선택) — 개발 시드 데이터
- production migration은 동일 V-prefix 순서로

→ ADR-0004, → @docs/conventions/sql.md
