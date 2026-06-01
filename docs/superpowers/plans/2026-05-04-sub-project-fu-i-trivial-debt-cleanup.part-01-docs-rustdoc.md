# Sub-project FU-i Trivial Debt Cleanup - Part 01: Docs And Rustdoc Cleanup

Parent index: [Sub-project FU-i Trivial Debt Cleanup](./2026-05-04-sub-project-fu-i-trivial-debt-cleanup.md).

## Phase A: docs-only

### Task 1: FU 12 + 13 + 17 (docs/rustdoc only)

**Files (modify):**
- `docs/superpowers/specs/2026-05-02-sub-project-2-db-core-domain-design.md` (FU 12)
- `docs/superpowers/specs/2026-05-03-sub-project-5-iii-audit-pipeline-operations-rds-design.md` (FU 13)
- `crates/domain/audit/audit-log/src/repository.rs` (FU 17)
- `crates/operations/operations-meta/src/repository.rs` (FU 17)

- [ ] **Step 1: FU 12 — `listing_photo` prefix 정정**

`docs/superpowers/specs/2026-05-02-sub-project-2-db-core-domain-design.md` 의 `listing_photo` 테이블 inline comment 찾아 정정:

```bash
grep -n "ph_\.\.\." docs/superpowers/specs/2026-05-02-sub-project-2-db-core-domain-design.md
```

찾은 라인:
```sql
id char(30) primary key,                            -- ph_...
```
변경:
```sql
id char(30) primary key,                            -- lph_... (3-char prefix invariant; was `ph_` in earlier drafts)
```

- [ ] **Step 2: FU 13 — AuditLog spec § 4.3 mock SQL 정정**

`docs/superpowers/specs/2026-05-03-sub-project-5-iii-audit-pipeline-operations-rds-design.md` § 4.3 의 PgImpl 패턴 mock 안 `INSERT INTO audit_log` 절 (틀린 컬럼: `metadata` / `occurred_at` / `client_ip`):

```bash
grep -n "INSERT INTO audit_log\|metadata, correlation_id, occurred_at, client_ip" docs/superpowers/specs/2026-05-03-sub-project-5-iii-audit-pipeline-operations-rds-design.md
```

기존 (각각 등장 위치마다):
```sql
INSERT INTO audit_log (
    id, actor_id, action, resource_kind, resource_id,
    metadata, correlation_id, occurred_at, client_ip, user_agent
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
```

수정:
```sql
INSERT INTO audit_log (
    id, actor_id, action, resource_kind, resource_id,
    before_state, after_state,
    ip_address, user_agent,
    correlation_id, created_at
)
VALUES ($1, $2, $3, $4, $5, NULL, $6, $7::inet, $8, $9, $10)
```

§ 5.1 / § 5.2 시퀀스 다이어그램 안의 audit_log INSERT 설명도 `metadata` → `after_state`, `client_ip` → `ip_address`, `occurred_at` → `created_at` 으로 정정.

§ 11 SSS 매핑 표 에서 FU 13 항목 (있으면) 또는 § 12 (Follow-up items) 에 "✅ FU 13 closed by SP-FU-i" 표기.

- [ ] **Step 3: FU 17 — `AuditLogRepository` rustdoc 갱신**

`crates/domain/audit/audit-log/src/repository.rs` 의 `find_by_resource` 와 `find_by_actor` rustdoc:

`find_by_resource` 위 doc 갱신:
```rust
/// `resource_kind` + `resource_id` 로 audit log 조회.
///
/// 결과는 `created_at` desc, 최대 `limit` 건. admin audit 화면에서 자주 사용.
///
/// # Errors
///
/// DB 통신 실패 시 [`RepoError::Database`].
async fn find_by_resource(
    &self,
    resource_kind: &str,
    resource_id: &str,
    limit: u32,
) -> Result<Vec<AuditLog>, RepoError>;
```

`find_by_actor` 위 doc 갱신:
```rust
/// 특정 사용자가 일으킨 audit log 조회 (`since` 시점부터).
///
/// 결과는 `created_at` desc, 최대 `limit` 건. admin 의 사용자별 활동 추적용.
///
/// # Errors
///
/// DB 통신 실패 시 [`RepoError::Database`].
async fn find_by_actor(
    &self,
    actor_id: &Id<UserMarker>,
    since: DateTime<Utc>,
    limit: u32,
) -> Result<Vec<AuditLog>, RepoError>;
```

- [ ] **Step 4: FU 17 — `OperationsMetaRepository::find_unacknowledged_alerts` rustdoc 정정**

`crates/operations/operations-meta/src/repository.rs` 의 `find_unacknowledged_alerts` 위 rustdoc:

기존:
```rust
/// 미응답 alert 를 오래된 순(`created_at` ASC) 으로 최대 `limit` 건 반환.
```

변경:
```rust
/// 미응답 alert 를 *severity* 우선 (`critical > error > warning > info`) +
/// 동순위 내 `created_at` `DESC` 로 최대 `limit` 건 반환. spec § 5.5
/// `system_alert_unack_idx (severity, created_at desc) where acknowledged_at is null`
/// partial index 활용.
```

- [ ] **Step 5: 로컬 검증**

```bash
cd c:/Users/User/Desktop/gongzzang_2
cargo check -p audit-log-domain -p operations-meta-domain
cargo clippy -p audit-log-domain -p operations-meta-domain --all-features -- -D warnings
```

doc 만 변경했으므로 컴파일 + clippy 모두 통과. 기존 단위 테스트 그린 유지.

- [ ] **Step 6: Commit + push**

```bash
git add docs/superpowers/specs/2026-05-02-sub-project-2-db-core-domain-design.md \
        docs/superpowers/specs/2026-05-03-sub-project-5-iii-audit-pipeline-operations-rds-design.md \
        crates/domain/audit/audit-log/src/repository.rs \
        crates/operations/operations-meta/src/repository.rs
git commit -m "docs(sp-fu-i-t1): close FU 12 / FU 13 / FU 17 — spec & rustdoc 정정

FU 12: listing_photo inline prefix `ph_` → `lph_` (3-char invariant)
FU 13: AuditLog spec § 4.3 mock SQL ↔ 실제 schema 정합 (metadata → before_state/after_state,
       client_ip → ip_address, occurred_at → created_at)
FU 17: AuditLogRepository::find_by_resource/find_by_actor rustdoc 갱신 (limit/since 의미 명시),
       OperationsMetaRepository::find_unacknowledged_alerts rustdoc 정정 (severity DESC + created_at DESC)

코드 변경 0 (rustdoc 만). spec ↔ schema ↔ trait doc SSOT 회복."
git push
gh run list --branch main --limit 3
gh run watch <CI-run-id> --exit-status
```

3 워크플로우 그린 확인 (markdown link check + clippy + 등 모두).

---
