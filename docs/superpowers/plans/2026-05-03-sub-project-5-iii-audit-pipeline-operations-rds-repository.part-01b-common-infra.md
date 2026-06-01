# Sub-project 5-iii Audit Pipeline Operations RDS Repository - Part 01B: Common Infrastructure

Parent index: [Sub-project 5-iii Audit Pipeline Operations RDS Repository - Part 01](./2026-05-03-sub-project-5-iii-audit-pipeline-operations-rds-repository.part-01.md).

## Phase B: 공통 인프라

### Task 2: `error_map.rs` 8 신규 `MapFromSqlx` impl + `db` Cargo.toml deps

**Files:**
- Modify: `crates/db/Cargo.toml`
- Modify: `crates/db/src/error_map.rs`
- Modify: `crates/db/src/lib.rs`

- [ ] **Step 1: `crates/db/Cargo.toml` 8 도메인 deps 추가**

기존 deps 다음에 (alphabetic):
```toml
[dependencies]
# ... 기존 ...
admin-action-domain = { path = "../operations/admin-action", version = "0.1.0" }
audit-log-domain = { path = "../domain/audit/audit-log", version = "0.1.0" }
bvq-domain = { path = "../operations/business-verification-queue", version = "0.1.0" }
data-pipeline-control = { path = "../data-pipeline-control", version = "0.1.0" }
listing-domain = { path = "../domain/core/listing", version = "0.1.0" }      # 이미 있음
listing-photo-domain = { path = "../domain/core/listing-photo", version = "0.1.0" }  # 이미 있음
listing-report-domain = { path = "../operations/listing-report", version = "0.1.0" }
lrq-domain = { path = "../operations/listing-review-queue", version = "0.1.0" }
operations-meta-domain = { path = "../operations/operations-meta", version = "0.1.0" }
outbox-event-domain = { path = "../domain/audit/outbox-event", version = "0.1.0" }
shared-kernel = { path = "../domain/core/shared-kernel", version = "0.1.0" }  # 이미 있음
user-domain = { path = "../domain/core/user", version = "0.1.0" }              # 이미 있음
```

각 crate 의 실제 `[package].name` 확인 후 정정:
```bash
grep -A 1 '\[package\]' crates/operations/admin-action/Cargo.toml | head -3
# ...
```

- [ ] **Step 2: `crates/db/src/error_map.rs` 8 impl 추가**

기존 3 impl (user / listing / listing-photo) 끝에 추가:

```rust
impl MapFromSqlx for audit_log_domain::repository::RepoError {
    fn conflict() -> Self {
        // audit_log 는 immutable, OCC 없음. unique violation 도 ULID 자동 생성으로 발생 안 함.
        // 여기 도달했다면 비정상 — Database 로 fallback.
        Self::Database("unexpected conflict in audit_log".to_owned())
    }
    fn database(msg: String) -> Self {
        Self::Database(msg)
    }
}

impl MapFromSqlx for outbox_event_domain::repository::RepoError {
    fn conflict() -> Self {
        Self::Database("unexpected conflict in outbox_event".to_owned())
    }
    fn database(msg: String) -> Self {
        Self::Database(msg)
    }
}

impl MapFromSqlx for data_pipeline_control::repository::RepoError {
    fn conflict() -> Self {
        Self::Conflict
    }
    fn database(msg: String) -> Self {
        Self::Database(msg)
    }
}

impl MapFromSqlx for admin_action_domain::repository::RepoError {
    fn conflict() -> Self {
        // AdminAction 은 insert-only. id 중복 시 Conflict.
        Self::Database("unexpected conflict in admin_action".to_owned())
    }
    fn database(msg: String) -> Self {
        Self::Database(msg)
    }
}

impl MapFromSqlx for bvq_domain::repository::RepoError {
    fn conflict() -> Self {
        Self::Conflict
    }
    fn database(msg: String) -> Self {
        Self::Database(msg)
    }
}

impl MapFromSqlx for lrq_domain::repository::RepoError {
    fn conflict() -> Self {
        Self::Conflict
    }
    fn database(msg: String) -> Self {
        Self::Database(msg)
    }
}

impl MapFromSqlx for listing_report_domain::repository::RepoError {
    fn conflict() -> Self {
        Self::Conflict
    }
    fn database(msg: String) -> Self {
        Self::Database(msg)
    }
}

impl MapFromSqlx for operations_meta_domain::repository::RepoError {
    fn conflict() -> Self {
        Self::Database("unexpected conflict in operations_meta".to_owned())
    }
    fn database(msg: String) -> Self {
        Self::Database(msg)
    }
}
```

> 실제 도메인 crate 이름은 `Cargo.toml [package].name` 따라 — 위는 추정. 첫 cargo check 에서 확인.
> 각 도메인의 RepoError variant 명도 확인 — `Conflict` 가 있는지 / `NotFound` 만 있는지. 없는 도메인은 fallback 으로 `Database`.

- [ ] **Step 3: `crates/db/src/lib.rs` 8 신규 `pub mod` 선언**

```rust
//! `SQLx` `Postgres` `Repository` 구현체.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod admin_action;
pub mod audit_log;
pub mod bvq;
pub mod error_map;
pub mod listing;
pub mod listing_photo;
pub mod listing_report;
pub mod lrq;
pub mod operations_meta;
pub mod outbox;
pub mod pipeline;
pub mod user;
```

- [ ] **Step 4: 8 stub 파일 생성** (T3-T10 에서 채울)

```bash
for f in audit_log outbox admin_action bvq lrq listing_report operations_meta pipeline; do
  cat > "crates/db/src/${f}.rs" <<EOF
//! \`Pg${f}Repository\` (placeholder, 후속 task 에서 구현).
EOF
done
```

각 stub 파일 그냥 doc-comment 하나만:
```rust
//! `PgAuditLogRepository` (placeholder, T3 에서 구현).
```

- [ ] **Step 5: 로컬 검증**

```bash
cargo check -p db
cargo clippy -p db --all-features --all-targets -- -D warnings
```

Expected: 8 도메인 dep 등록 + module 선언 통과. 기존 user/listing/listing_photo unit + 2 error_map unit 통과.

- [ ] **Step 6: Commit + push**

```bash
git add crates/db/Cargo.toml crates/db/src/error_map.rs crates/db/src/lib.rs \
        crates/db/src/audit_log.rs crates/db/src/outbox.rs \
        crates/db/src/admin_action.rs crates/db/src/bvq.rs crates/db/src/lrq.rs \
        crates/db/src/listing_report.rs crates/db/src/operations_meta.rs crates/db/src/pipeline.rs
git commit -m "feat(db): 8 신규 도메인 deps + MapFromSqlx impls + module stubs (SP5-iii T2)

- db Cargo.toml: 8 도메인 (audit-log, outbox-event, pipeline, admin-action, bvq,
  lrq, listing-report, operations-meta) deps 추가
- error_map.rs: 8 신규 RepoError MapFromSqlx impl (Conflict 없는 도메인은 Database fallback)
- lib.rs: 8 신규 module 선언 (audit_log, outbox, admin_action, bvq, lrq,
  listing_report, operations_meta, pipeline) — 본 task 는 stub 만"
git push
```

3 워크플로우 그린 확인.

---

## Phase C: Audit Infrastructure
