# migrations/

## 개요

이 디렉토리는 SQLx 마이그레이션의 단일 출처(SSOT)예요. PostgreSQL 스키마의 모든
변경은 여기에 *forward-only* SQL 파일로 누적돼요.

## 명명 규칙

`V<major>_<minor>__<snake_case>.sql` 형식을 따라요.

예시:

- `V001_01__core_tables.sql`
- `V001_02__listing_tables.sql`
- `V002_01__db_roles.sql`

규칙:

- **major** — 큰 변경 묶음 (V001 = 초기 18 테이블, V002 = role/권한)
- **minor** — major 내부 분할. 1500줄 룰을 강제하기 위해 ≤500줄/파일을 권장해요
- **snake_case 이름** — *변경 의도*를 표현해요 (`add_listing_index`,
  `drop_legacy_column` 등). 테이블 이름이 아니라 "이 PR이 무엇을 하는가"를 적어요

## 적용 순서

SQLx는 파일명 알파벳 정렬 순으로 적용해요
(`V001_01__...` < `V001_02__...` < `V002_01__...`).
새 마이그레이션은 항상 *마지막* 번호 다음에 추가하세요.

## 롤백 정책 — Forward-only

운영에서는 **절대** 과거 마이그레이션 SQL을 수정하지 않아요. 한 번 머지된
파일은 immutable이에요.

실수를 정정하려면 *새* 마이그레이션을 추가해 되돌려요
(예: `V003_01__revert_X.sql`).

로컬 개발에서는 다음 한 줄로 DB를 처음부터 재구성할 수 있어요:

```bash
sqlx database drop -y && sqlx database create && sqlx migrate run --source migrations
```

## 로컬 검증

루트에서 한 줄이면 끝이에요:

```bash
bash scripts/sqlx-migrate.sh
```

사전 조건:

- Docker Compose 기동 (`infrastructure/docker/`)
- sqlx-cli 설치

sqlx-cli가 없다면:

```bash
cargo install sqlx-cli --version 0.8.2 --locked --no-default-features --features postgres,rustls
```

## CI 검증

`.github/workflows/db-migrations.yml`이 PR마다 자동으로 돌아요 (Task 26).
PG17 + PostGIS 컨테이너를 띄우고 모든 마이그레이션을 적용한 뒤 테이블 카운트를
검증해요. 실패하면 머지가 차단돼요.

## 블루-그린 호환 변경 패턴 (DDL 안전성)

DDL은 **별도 PR**로 분리하세요. 코드 변경과 같이 묶으면 롤백 단위가 깨져요.

- **새 컬럼 추가**: NULL 허용으로 추가 → 백필 → NOT NULL 변환 (3-step)
- **컬럼 제거**: 코드에서 미참조 확인 → 1주 대기 → `DROP COLUMN`
- **인덱스 추가**: 운영에서는 `CREATE INDEX CONCURRENTLY`로 lock을 회피해요

이 패턴을 지키면 두 버전의 앱이 동시에 같은 DB를 바라봐도 깨지지 않아요.

## 참고 링크

- SQLx migrations:
  <https://docs.rs/sqlx/latest/sqlx/migrate/struct.Migrator.html>
- SQLx CLI: <https://github.com/launchbadge/sqlx/tree/main/sqlx-cli>
