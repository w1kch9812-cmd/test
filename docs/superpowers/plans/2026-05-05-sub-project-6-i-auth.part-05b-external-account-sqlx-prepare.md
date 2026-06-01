# Sub-project 6-i Auth - Part 05B: External Account and sqlx Prepare

Parent index: [Sub-project 6-i Auth - Part 05](./2026-05-05-sub-project-6-i-auth.part-05.md).
## Task 6: V004 migration + sqlx prepare hook + first-sign-in external_account insert

**Files:**
- Create: `migrations/30008_user_ci_external_account.sql`
- Modify: `crates/auth/src/middleware.rs` (first sign-in 시 external_account zitadel insert)
- Modify: `lefthook.yml` (pre-push 에 sqlx prepare --check)
- Modify: `tarpaulin.toml` (auth crate 새 모듈 포함 확인)

- [ ] **Step 6.1: migration 작성**

`migrations/30008_user_ci_external_account.sql`:

```sql
-- V003_08: SP6-i Auth Core 의 schema 자리.
-- users.ci 는 SP6-CI (KISA 본인확인) 가 채움.
-- external_account 의 kakao/naver/google 행은 SP6-Social federation 이 채움.

ALTER TABLE "user" ADD COLUMN ci VARCHAR(88) UNIQUE NULL;
COMMENT ON COLUMN "user".ci IS
  'KISA Connecting Information (88-char hash). NULL until SP6-CI verifies via NICE/Toss/PASS.';

CREATE TABLE external_account (
    id           CHAR(30) PRIMARY KEY,
    user_id      CHAR(30) NOT NULL REFERENCES "user"(id) ON DELETE CASCADE,
    provider     VARCHAR(32) NOT NULL,
    external_id  VARCHAR(255) NOT NULL,
    linked_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (provider, external_id)
);

CREATE INDEX external_account_user_idx ON external_account(user_id);
CREATE INDEX external_account_provider_idx ON external_account(provider, linked_at DESC);

COMMENT ON TABLE external_account IS
  'Multi-IdP linking. SP6-i populates only zitadel rows on first sign-in. SP6-Social federation populates kakao/naver/google.';

-- provider 값 제약 (SP6-Social 이 추가 시 ALTER 가능)
ALTER TABLE external_account
  ADD CONSTRAINT external_account_provider_chk
  CHECK (provider IN ('zitadel', 'kakao', 'naver', 'google', 'apple'));
```

(NOTE: 기존 `user` 테이블 이름이 `"user"` quoted — V001 패턴 일관 유지. id 는 `char(30)` `usr_...` 형식.)

- [ ] **Step 6.2: migration 적용 + sqlx prepare**

```
psql $DATABASE_URL -f migrations/30008_user_ci_external_account.sql
cargo sqlx prepare --workspace
```

Expected: `.sqlx/` 의 query json 갱신 (auth 가 user 테이블 select 하는 경우).

- [ ] **Step 6.3: first sign-in 시 external_account insert**

`crates/auth/src/middleware.rs` 의 first-sign-in 분기에 추가 (User 자동 생성 후, 같은 트랜잭션 또는 best-effort INSERT):

```rust
// User 자동 생성 직후
if was_first_sign_in {
    let external_id = format!("ea_{}", &uuid::Uuid::new_v4().simple().to_string()[..26]);
    if let Err(e) = sqlx::query(
        r#"
        INSERT INTO external_account (id, user_id, provider, external_id)
        VALUES ($1, $2, 'zitadel', $3)
        ON CONFLICT (provider, external_id) DO NOTHING
        "#,
    )
    .bind(&external_id)
    .bind(user.id.as_str())
    .bind(&claims.sub)
    .execute(pool)
    .await
    {
        tracing::warn!(error = %e, "external_account zitadel insert failed (best-effort)");
    }
}
```

(실제 위치는 middleware.rs 의 first sign-in 로직 확인 후 결정. 현재 코드 미확인 시 Step 7 의 코드 검토 후 정확한 위치 적용.)

- [ ] **Step 6.4: lefthook.yml 에 sqlx prepare check 추가**

`lefthook.yml` 의 `pre-push:` 섹션에 추가:

```yaml
    sqlx-prepare-check:
      run: command -v cargo >/dev/null 2>&1 && (DATABASE_URL=${DATABASE_URL:-postgres://gongzzang:gongzzang@localhost:5432/gongzzang} cargo sqlx prepare --workspace --check) || echo "cargo not installed - CI enforces"
      skip:
        - merge
        - rebase
```

- [ ] **Step 6.5: tarpaulin.toml 검토**

`tarpaulin.toml` — `crates/auth/` 가 이미 포함되어 있는지 확인. exclude 목록에 jti_denylist / audit 가 없어야 함 (90% threshold 적용).

- [ ] **Step 6.6: db-migrations workflow assertion 갱신**

`tests/migrations/test_v001_full.sh` 의 `EXPECTED_TABLES` 배열에 `external_account` 추가 (SP7-iii 에서 이미 동적 count 사용 중 — 새 테이블 1개 추가 시 자동 반영되지만 명시 등록은 필요):

```bash
# 변경 전 (SP7-iii 후 상태):
EXPECTED_TABLES=(... api_health_check)

# 변경 후 (SP6-i 추가):
EXPECTED_TABLES=(... api_health_check external_account)
```

확인 명령:

```bash
grep -n "EXPECTED_TABLES" tests/migrations/test_v001_full.sh
# 해당 라인의 배열에 external_account 추가
bash tests/migrations/test_v001_full.sh  # 로컬 검증 (필요한 환경 변수 설정 후)
```

- [ ] **Step 6.7: 전체 빌드 + clippy + test**

```
cargo check --workspace --all-features
cargo clippy --workspace --all-features --all-targets -- -D warnings
DATABASE_URL=postgres://... cargo sqlx prepare --workspace --check
```

Expected: PASS.

- [ ] **Step 6.8: Commit**

```bash
git add migrations/30008_user_ci_external_account.sql crates/auth/src/middleware.rs lefthook.yml .sqlx/ tests/migrations/ .github/workflows/db-migrations.yml
git commit -m "feat(6i-T6): V004 schema (users.ci + external_account) + sqlx prepare hook

- migrations/30008: users.ci VARCHAR(88) UNIQUE NULL (SP6-CI 채움) + external_account 테이블 (SP6-Social 채움), zitadel 한 줄만 first sign-in 시 자동 insert
- middleware.rs: first sign-in 시 external_account('zitadel', sub) INSERT (best-effort)
- lefthook.yml: pre-push 에 cargo sqlx prepare --check 추가 (V004 schema drift 차단)"
```

---
