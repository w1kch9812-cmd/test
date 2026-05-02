# migrations/

## 개요

이 디렉토리는 SQLx 마이그레이션의 단일 출처(SSOT)예요. PostgreSQL 스키마의 모든
변경은 여기에 *forward-only* SQL 파일로 누적돼요.

## 명명 규칙

`<MMmmm>_<snake_case>.sql` 형식이에요. `MMmmm`은 5자리 정수 버전이에요
(`MM` = major × 10000, `mmm` = minor). sqlx-cli가 첫 번째 `_` 앞을 i64로
파싱하기 때문에 정수만 허용돼요.

예시:

- `10001_core_tables.sql` — major 1, minor 1
- `10002_insights_tables.sql` — major 1, minor 2
- `20001_db_roles.sql` — major 2, minor 1

규칙:

- **major** — 큰 변경 묶음 (1xxxx = 초기 18 테이블, 2xxxx = role/권한)
- **minor** — major 내부 분할. 1500줄 룰을 강제하기 위해 ≤500줄/파일을 권장해요
- **snake_case 이름** — *변경 의도*를 표현해요 (`add_listing_index`,
  `drop_legacy_column` 등). 테이블 이름이 아니라 "이 PR이 무엇을 하는가"를 적어요

## 적용 순서

SQLx는 정수 버전 오름차순으로 적용해요 (`10001 < 10002 < 20001`).
새 마이그레이션은 항상 *마지막* 버전 다음에 추가하세요.

## 롤백 정책 — Forward-only

운영에서는 **절대** 과거 마이그레이션 SQL을 수정하지 않아요. 한 번 머지된
파일은 immutable이에요.

실수를 정정하려면 *새* 마이그레이션을 추가해 되돌려요
(예: `30001_revert_X.sql`).

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
- **인덱스 추가**: 운영에서는 `CREATE INDEX CONCURRENTLY`로 lock을 회피해요.
  단, sqlx는 마이그레이션을 트랜잭션으로 감싸기 때문에 `CONCURRENTLY`는
  *별도 파일*에 넣고 첫 줄에 `-- sqlx:no-tx` 마커를 붙여 트랜잭션을 꺼요

이 패턴을 지키면 두 버전의 앱이 동시에 같은 DB를 바라봐도 깨지지 않아요.

## 마이그레이션 실패 복구

마이그레이션이 중간에 실패하면 `_sqlx_migrations` 테이블이 부분 적용 상태를 기록해요.

```bash
sqlx migrate info --source migrations    # 적용 상태 확인
```

복구 절차:

- **로컬**: `sqlx database drop -y && sqlx database create && sqlx migrate run`
  (위 *로컬 개발 재구성* 한 줄과 동일)
- **운영**: 절대 손으로 `_sqlx_migrations`을 건드리지 마세요. 새 *fix-forward*
  마이그레이션(`V<next>__fix_<원인>.sql`)을 PR로 올려서 진행하세요

## 참고 링크

- SQLx migrations:
  <https://docs.rs/sqlx/latest/sqlx/migrate/struct.Migrator.html>
- SQLx CLI: <https://github.com/launchbadge/sqlx/tree/main/sqlx-cli>
