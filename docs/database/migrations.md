# 마이그레이션 운영 가이드

PostgreSQL 17 + PostGIS 3.5 마이그레이션 운영을 위한 실무 안내예요.

## 1. 명명 규칙

`<MMmmm>_<snake_case>.sql` 형식이에요. 5자리 정수 버전(`MM` = major × 10000,
`mmm` = minor)에 snake_case 의도를 붙여요. sqlx-cli가 첫 번째 `_` 앞을 i64로
파싱하기 때문에 정수만 허용돼요.

- **major (만 단위)**: 큰 변경 묶음 (1xxxx = 초기 18 테이블, 2xxxx = role + 트리거)
- **minor**: major 내부 분할. 한 파일이 500줄을 넘지 않게 강제하기 위한 분리예요
- **snake_case 이름**: *변경 의도*를 짧게 표현 (`add_listing_index`, `drop_legacy_column` 등)

예:

- `10001_core_tables.sql` — Core BC 3 테이블 (user, listing, listing_photo)
- `10002_insights_tables.sql` — Insights BC 5 테이블
- `20001_db_roles.sql` — 3 role
- `20002_audit_immutable_trigger.sql` — UPDATE/DELETE 박탈 트리거

## 2. 적용 순서

`sqlx`는 정수 버전 오름차순으로 적용해요 (`10001` < `10002` < `20001`).
새 마이그레이션은 항상 *마지막* 버전 다음에 추가해요.

## 3. Forward-only 정책

운영에서는 절대 과거 마이그레이션 SQL을 수정하지 않아요. 한 번 머지된 파일은 immutable.

실수를 정정하려면 *새* 마이그레이션을 추가해 되돌려요 (예: `30001_revert_X.sql`).

로컬 개발에서는 다음 한 줄로 DB를 처음부터 재구성할 수 있어요:

```bash
sqlx database drop -y && sqlx database create && sqlx migrate run --source migrations
```

## 4. 로컬 검증

루트에서 한 줄이면 끝이에요:

```bash
bash scripts/sqlx-migrate.sh
```

사전 조건:

- `infrastructure/docker/`의 Compose 스택 기동 (PG17 + PostGIS + Valkey)
- `sqlx-cli` 설치

`sqlx-cli`가 없다면:

```bash
cargo install sqlx-cli --version 0.8.2 --locked --no-default-features --features postgres,rustls
```

## 5. CI 검증

`.github/workflows/db-migrations.yml`이 PR/main push마다 자동 실행해요.

- PG17+PostGIS 서비스 컨테이너 기동
- 모든 마이그레이션(`1xxxx` Core/Insights/System/Pipeline/Operations + `2xxxx` role/트리거) 적용
- `tests/migrations/test_v001_full.sh` 실행 (18 테이블 + 인덱스 + SRID 4326 검증)
- `tests/migrations/test_v002_audit_immutable.sh` 실행 (3 role + 트리거 + 권한 매트릭스 검증)

실패 시 머지 차단돼요.

## 6. 블루-그린 호환 변경 패턴

DDL은 별도 PR로 분리해요. 코드 변경과 같이 묶으면 롤백 단위가 깨져요.

- **새 컬럼 추가**: NULL 허용으로 추가 → 백필 → NOT NULL 변환 (3-step)
- **컬럼 제거**: 코드에서 미참조 확인 → 1주 대기 → `DROP COLUMN`
- **인덱스 추가**: 운영에서는 `CREATE INDEX CONCURRENTLY`로 lock을 회피해요. 단, sqlx는 마이그레이션을 트랜잭션으로 감싸기 때문에 `CONCURRENTLY`는 *별도 파일*에 넣고 첫 줄에 `-- sqlx:no-tx` 마커를 붙여 트랜잭션을 꺼요

이 패턴을 지키면 두 버전의 앱이 동시에 같은 DB를 바라봐도 깨지지 않아요.

## 7. 마이그레이션 실패 복구

마이그레이션이 중간에 실패하면 `_sqlx_migrations` 테이블이 부분 적용 상태를 기록해요.

```bash
sqlx migrate info --source migrations    # 적용 상태 확인
```

복구 절차:

- **로컬**: `sqlx database drop -y && sqlx database create && sqlx migrate run` (DB 처음부터 재구성)
- **운영**: 절대 손으로 `_sqlx_migrations`을 건드리지 마세요. 새 *fix-forward* 마이그레이션(`<MMmmm>_fix_<원인>.sql`)을 PR로 올려서 진행하세요

## 8. retention + archive

- `audit_log` — RDS 1년, 이후 R2 IA 6년 (총 7년 — PIPA + ISMS-P 요구)
- `outbox_event` — published 후 30일 후 삭제
- `notification` — 365일 후 자동 삭제
- `search_history` — 90일 후 user_id 가명화, 1년 후 삭제 (PIPA)
- 기타 retention 정책은 `docs/compliance/retention.md` (작성 예정)

## 9. 참고 링크

- SQLx migrations: <https://docs.rs/sqlx/latest/sqlx/migrate/struct.Migrator.html>
- SQLx CLI: <https://github.com/launchbadge/sqlx/tree/main/sqlx-cli>
- spec § 5 (RDS 18 테이블): `docs/superpowers/specs/2026-05-02-sub-project-2-db-core-domain-design.md`
- spec § 6 (DB role): 같은 파일 § 6
