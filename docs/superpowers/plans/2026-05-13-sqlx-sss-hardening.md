# SQLx SSS Hardening Assessment And Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development`
> or `superpowers:executing-plans` before implementing this plan. Steps use checkbox
> (`- [ ]`) syntax for tracking.

**Goal:** Keep SQLx, but make database access compile-time checked where it can be checked,
and make every remaining runtime SQL use explicit, reviewed exceptions.

**Architecture:** SQLx stays as the repository-layer database toolkit. Domain and application
crates must remain SQLx-free. Stable literal SQL moves to SQLx macros plus committed `.sqlx`
metadata; genuinely dynamic SQL remains runtime SQL with a documented reason and focused tests.

**Tech Stack:** Rust, SQLx 0.8, PostgreSQL/PostGIS, `cargo sqlx prepare`, committed `.sqlx`,
CI drift checks, and local hook checks.

---

## Strict Verdict

SQLx is the right default for this codebase.

The previous draft was directionally right, but it was not precise enough to execute as-is.
The biggest issue is ordering: `.sqlx/` only becomes valuable once at least one `query!`,
`query_as!`, or `query_scalar!` macro exists. With zero SQLx query macros in Rust source,
`cargo sqlx prepare` cannot validate the existing `sqlx::query(...)` calls.

This means the SSS path is not "generate `.sqlx` first, then maybe convert queries later."
The SSS path is:

1. Convert one small, stable repository method to SQLx macros.
2. Generate and commit `.sqlx/`.
3. Add CI/hook checks that fail on SQL/schema drift.
4. Convert the remaining stable queries in bounded batches.
5. Keep runtime SQL only where the SQL shape is intentionally dynamic.

## Evidence Snapshot

Checked on 2026-05-13.

Gongzzang:

- Runtime SQL calls in Rust files: `214`
  - `sqlx::query(`: `129`
  - `sqlx::query_scalar(`: `12`
  - `sqlx::query_as(`: `73`
- SQLx query macros in Rust files: `0`
  - `query!`: `0`
  - `query_as!`: `0`
  - `query_scalar!`: `0`
- Row extraction calls: `try_get`: `247`
- `.sqlx/`: absent
- `lefthook.yml`: `cargo sqlx prepare --workspace --check` can be skipped by `|| echo ...`
- Migration SSOT conflict exists:
  - `AGENTS.md` and `docs/ssot-matrix.md` point to `db/migration/V*.sql`
  - Actual SQLx migration source is `migrations/`
  - `db/migration/README.md` still describes old `V<NNN>__...` conventions
- Pool configuration is scattered:
  - `services/api/src/main.rs`: `max_connections(5)`
  - `services/outbox-publisher/src/main.rs`: `max_connections(2)`
  - `services/api/src/bin/raw_capture_sync.rs`: `max_connections(2)`
  - `crates/api-health-recorder/src/main.rs`: direct `PgPool::connect`

Platform-core note:

- Runtime SQL calls in Rust files: `149`
  - `sqlx::query(`: `128`
  - `sqlx::query_scalar(`: `15`
  - `sqlx::query_as(`: `6`
- SQLx query macros in Rust files: `0`
- `.sqlx/`: absent
- Platform-core has no `.github/` or `lefthook.yml` at this snapshot, so CI/hook tasks must be
  created there rather than copied from Gongzzang.

## What The Old Draft Got Right

- Keeping SQLx is the correct decision.
- Runtime-only SQL forfeits SQLx's strongest safety feature.
- `.sqlx/` should be committed once query macros exist.
- The fake-pass lefthook behavior is not enterprise-grade.
- `migrations/` must be the migration SSOT.
- Pool construction should be centralized.

## What The Old Draft Got Wrong Or Overstated

- `.sqlx/` is not a useful driver task before any query macro exists.
- `cargo sqlx prepare` does not validate plain `sqlx::query(...)` calls.
- "All new code must use macros" is too blunt. Stable literal SQL should use macros; dynamic
  filter/sort/query-builder code can remain runtime SQL with a documented exception.
- SeaORM being "no value" is too absolute. The current decision is "not needed now" because
  explicit SQL plus existing repository boundaries fit the codebase better.
- Clorinde/Cornucopia are not dismissed forever. They are just not the next move because SQLx
  already exists here, has lower migration cost, and gives enough safety once macros are used.
- PostGIS SRID lint is useful, but it is a separate DB-quality hardening task, not the core SQLx
  macro hardening task.

## Target SQLx Policy

Use this as the rule for future work:

- Domain and application crates must not import SQLx.
- Repository and Unit-of-Work crates may use SQLx.
- Stable literal SQL must use `query!`, `query_as!`, or `query_scalar!`.
- Dynamic SQL must use parameter binding and a small allow-list comment explaining why a macro
  cannot be used.
- Dynamic SQL should prefer `sqlx::QueryBuilder<Postgres>` or a small local builder over ad hoc
  string concatenation.
- `.sqlx/` is committed and treated as query/schema contract metadata.
- CI runs both:
  - Offline compile/check using committed `.sqlx/`
  - Live `cargo sqlx prepare --workspace --check` against migrated Postgres
- Migration source is only `migrations/`.
- DB integration tests return `Result` and avoid `unwrap`/`expect` unless a test-local helper
  explicitly justifies the panic boundary.

## Task 1: Correct Migration SSOT Docs

**Files:**

- Modify: `AGENTS.md`
- Modify: `docs/ssot-matrix.md`
- Modify: `db/migration/README.md`
- Read: `migrations/README.md`

- [ ] Replace `db/migration/V*.sql` with `migrations/*.sql`.
- [ ] Document the actual naming convention from `migrations/README.md`:
  `<MMmmm>_<snake_case>.sql`.
- [ ] Change `db/migration/README.md` into a redirect note that says the folder is deprecated.
- [ ] Run:

```bash
rg "db/migration|V\\*\\.sql|V<NNN>|V001" AGENTS.md docs db migrations
```

Expected: only deliberate deprecation text remains in `db/migration/README.md`.

## Task 2: Build The First Macro Walking Skeleton

**Files:**

- Modify: `crates/db/src/user.rs`
- Modify: `crates/db/README.md`

- [ ] Pick one small stable method in `PgUserRepository`, preferably a single-row lookup with a
  fixed column list.
- [ ] Convert it from `sqlx::query(...)` plus `try_get(...)` to `sqlx::query_as!` or
  `sqlx::query!`.
- [ ] Keep existing `RepoError` mapping behavior.
- [ ] Run:

```bash
sqlx migrate run --source migrations
cargo check --workspace
```

Expected: compile succeeds while the query is validated against the migrated database.

- [ ] Temporarily break one selected column name in the macro query.
- [ ] Run:

```bash
cargo check --workspace
```

Expected: compile fails because SQLx detects the query/schema mismatch.

- [ ] Restore the correct SQL.

## Task 3: Generate And Commit `.sqlx/`

**Files:**

- Create: `.sqlx/query-*.json`
- Create: `scripts/sqlx-prepare.ps1`
- Optionally create: `scripts/sqlx-prepare.sh`

- [ ] Install the CLI if missing:

```bash
cargo install sqlx-cli --version 0.8.6 --locked --no-default-features --features postgres,rustls
```

- [ ] Run:

```bash
sqlx migrate run --source migrations
cargo sqlx prepare --workspace
```

Expected: `.sqlx/query-*.json` files are generated for the macro query.

- [ ] Run an offline compile with no live DB dependency:

```bash
SQLX_OFFLINE=true cargo check --workspace
```

Expected: compile succeeds using committed `.sqlx/` metadata.

## Task 4: Make Hook And CI Checks Real

**Files:**

- Modify: `lefthook.yml`
- Modify: `.github/workflows/ci.yml`
- Modify or create: `.github/workflows/sqlx-prepare.yml`

- [ ] Remove the `|| echo "sqlx prepare check skipped ..."` fake-pass path.
- [ ] Keep a clear local developer message when `cargo` or `sqlx` is missing, but do not report
  success for a failed SQLx check.
- [ ] Add a CI job that starts Postgres/PostGIS, runs `sqlx migrate run --source migrations`,
  and then runs:

```bash
cargo sqlx prepare --workspace --check
```

Expected: CI fails if `.sqlx/` is stale or SQL no longer matches the schema.

- [ ] Add an offline Rust check job:

```bash
SQLX_OFFLINE=true cargo check --workspace
```

Expected: CI can compile without depending on a live DB for every Rust check.

## Task 5: Convert Stable Queries By Batch

**Files:**

- Modify in batches: `crates/db/src/*.rs`
- Test: matching `crates/db/tests/*_integration.rs`

- [ ] Start with `user.rs`.
- [ ] Continue with one repository file per PR.
- [ ] For each file, classify every query as:
  - `macro`: fixed SQL shape, compile-time check required
  - `dynamic`: runtime SQL allowed with reason
  - `test-only`: acceptable only in integration test setup/assertions
- [ ] After each batch, run:

```bash
cargo sqlx prepare --workspace
SQLX_OFFLINE=true cargo check --workspace
cargo test --workspace
```

Expected: `.sqlx/` updates are committed together with the code that introduced them.

## Task 6: Centralize Pool Configuration

**Files:**

- Create: `crates/db/src/pool.rs`
- Modify: `crates/db/src/lib.rs`
- Modify: `services/api/src/main.rs`
- Modify: `services/outbox-publisher/src/main.rs`
- Modify: `services/api/src/bin/raw_capture_sync.rs`
- Modify: `crates/api-health-recorder/src/main.rs`

- [ ] Add a `PgPoolSettings` struct with explicit defaults.
- [ ] Build pools through one helper wrapping `PgPoolOptions`.
- [ ] Keep per-service max-connection overrides explicit.
- [ ] Run:

```bash
cargo check --workspace
cargo test --workspace
```

Expected: every service uses the shared pool builder.

## Sources

- SQLx README: <https://github.com/launchbadge/sqlx>
- SQLx `query!` offline mode docs:
  <https://docs.rs/sqlx/latest/sqlx/macro.query.html#offline-mode>
- SQLx migrations docs:
  <https://docs.rs/sqlx/latest/sqlx/migrate/index.html>
- SQLx test database docs:
  <https://docs.rs/sqlx/latest/sqlx/attr.test.html>
