# Sub-project Observability — 계획

| | |
|---|---|
| 작성일 | 2026-05-06 |
| 상태 | Approved |
| 선행 spec | [`2026-05-06-sub-project-observability-design.md`](../specs/2026-05-06-sub-project-observability-design.md) |
| 추정 | 8 task, 3-4일 |

---

## T1 — spec + plan 커밋

이 commit. `docs(sp-obs): spec + plan -- production 관측성 + audit chain hardening`

---

## T2 — `RequestId` extension + Axum middleware + Next.js proxy propagation

**대상**:
- `services/api/src/http/request_id.rs` (신규) — `RequestId` 구조체 + middleware,
  tracing span 자동 attach
- `services/api/src/main.rs` — 모든 router 가 `request_id_layer` 거치도록 (auth_layer 보다 먼저)
- `apps/web/lib/api.ts` — ky `beforeRequest` hook 가 `crypto.randomUUID()` 또는
  `req_<ULID>` 추가
- `apps/web/proxy.ts` — inbound `X-Request-Id` propagate

**테스트**:
- 단위 (Axum): id 생성 / 응답 echo / span attach
- 통합 (CI smoke): `curl -H 'X-Request-Id: test-xyz'` → 응답 header `X-Request-Id: test-xyz`

**commit**: `feat(sp-obs-t2): X-Request-Id correlation chain (Axum middleware + Next.js proxy)`

---

## T3 — `MutationContextBuilder` extractor + auto-inject

**대상**:
- `services/api/src/http/mutation_ctx.rs` 확장:
  - `MutationContextBuilder` struct + `FromRequestParts` impl
  - `extract_client_ip` (X-Forwarded-For first, fallback ConnectInfo)
  - `build(action) -> MutationContext` 가 `with_client_info_optional` 호출
- `crates/domain/core/shared-kernel/src/mutation.rs`:
  - `with_client_info_optional(ip, ua)` 신규 (둘 다 Option)
- 모든 handler 가 `Extension<AuthenticatedUser> + ConnectInfo<SocketAddr>` →
  `MutationContextBuilder` 로 마이그
- 5 affected handlers (`create_listing` / `patch_listing` / 4 transitions / photos)

**테스트**:
- shared-kernel `mutation.rs` 단위 2 (with_client_info_optional)
- api 단위 4 (extractor — auth 없음 reject / xff 우선 / direct ip / ua trim 500)

**commit**: `feat(sp-obs-t3): MutationContextBuilder extractor (auth + ip + ua + correlation auto-inject)`

---

## T4 — PgRepository `before_state` snapshot 패턴

**대상**: `crates/db/src/{user,listing,listing_photo,bookmark,analysis_report}.rs`

각 PgRepository `save` (또는 `delete`) 가:

1. tx 시작
2. `SELECT row_to_json(...)` for current row → `before_state` (None if INSERT)
3. UPSERT new aggregate
4. after_state = `serde_json::to_value(&aggregate).ok()` (또는 SELECT row_to_json
   다시 — 결정 per repo)
5. INSERT audit_log with both states + `__metadata__` nesting (`ctx.metadata` 가
   `Some` 일 때만)
6. outbox_event 기존
7. commit

**`__metadata__` nesting 헬퍼** (`crates/db/src/audit_log_metadata.rs` 신규 또는
inline):

```rust
fn merge_metadata(after_state: Option<Value>, metadata: Option<&Value>) -> Option<Value> {
    match (after_state, metadata) {
        (Some(Value::Object(mut obj)), Some(meta)) => {
            obj.insert("__metadata__".to_owned(), meta.clone());
            Some(Value::Object(obj))
        }
        (Some(s), Some(meta)) => Some(serde_json::json!({"__state__": s, "__metadata__": meta})),
        (s, _) => s,
    }
}
```

**Notification / SearchHistory 는 패턴 변경 없음** — append-only (insert-only)
또는 mark_read 처럼 metadata-driven.

**테스트**:
- 통합 5 신규 (각 repo 1 sample, update 시 before_state 가 이전 값 검증)

**commit**: `feat(sp-obs-t4): audit_log before_state capture in 5 PgRepositories (full diff trail)`

---

## T5 — Sentry init Rust + Next.js

**대상**:
- workspace `Cargo.toml` deps: `sentry = "0.34"`, `sentry-tracing = "0.34"`
- `services/api/src/main.rs`, `services/outbox-publisher/src/main.rs`,
  `crates/api-health-recorder/src/main.rs` — `init_sentry()` helper
- env: `SENTRY_DSN` (없으면 silent disabled), `APP_ENV`, `GIT_SHA` (build-time)
- `crates/auth/Cargo.toml` 의 verifier panic point 재검토 — sentry capture
- `apps/web/`:
  - `@sentry/nextjs` install
  - `apps/web/instrumentation.ts` — Sentry.init (이미 placeholder)
  - `NEXT_PUBLIC_SENTRY_DSN` env

**테스트**:
- 단위 4 (init helper — DSN 없음 → None / 있음 → Some / release tag / env tag)

**commit**: `feat(sp-obs-t5): Sentry init (Rust + Next.js, env-driven, silent disabled if no DSN)`

---

## T6 — OTLP / Prometheus 골격

**대상**:
- workspace dep: `tracing-opentelemetry`, `opentelemetry-otlp`, `prometheus`
- `services/api`, `services/outbox-publisher`:
  - `init_tracing()` 가 OTLP layer 조건부 add (env `OTLP_ENDPOINT` 있을 때)
  - service.name, service.version 자동 set
- `services/outbox-publisher/src/main.rs`:
  - `prometheus::Registry` + Counter / Histogram / Gauge
  - `axum` mini-server `:9091/metrics` (또는 hyper 직접 — 가벼움)
  - `tick` loop 가 metric record

**테스트**:
- 단위 (init): registry 등록 검증
- 통합 (smoke): curl localhost:9091/metrics → content-type + counter present

**commit**: `feat(sp-obs-t6): OTLP exporter + Prometheus metrics (env-driven, services/api + outbox-publisher)`

---

## T7 — Health check 강화 + drift detector → Sentry alert

**대상**:
- `services/api/src/routes/health.rs` (신규):
  - `liveness` `/healthz` (항상 200)
  - `readiness` `/healthz/ready` (db ping + redis ping if configured)
  - `db_health` `/healthz/db` (debug — production 은 internal access only, FU)
- `crates/api-health-recorder/src/main.rs` — Issue 생성 후 Sentry capture_message
  추가

**테스트**:
- 통합 (api/tests): liveness 200 / readiness db down → 503 / readiness
  정상 → 200

**commit**: `feat(sp-obs-t7): health checks (liveness/readiness/db) + drift detector Sentry alert`

---

## T8 — workspace 검증 + push + SSOT

- 로컬 `cargo clippy --workspace --all-features --all-targets -- -D warnings` 그린
- 로컬 `pnpm -F web run typecheck test` 그린
- push → 5 CI workflow 그린
- SSOT 갱신:
  - `docs/superpowers/roadmap.md` SP-Obs ✅
  - `memory/project_progress.md` 본문
  - `MEMORY.md` index
  - FU 90-96 list

**commit**: `docs(sp-obs-t8): SP-Observability 종료 -- production 관측성 + audit chain hardening`

---

## 변경 파일 요약

| 분류 | 파일 | 변경 |
|---|---|---|
| domain | `crates/domain/core/shared-kernel/src/mutation.rs` | with_client_info_optional |
| audit infra | `services/api/src/http/{request_id,mutation_ctx}.rs` | 신규 + 확장 |
| audit infra | `services/api/src/main.rs` | request_id_layer + sentry init + OTLP |
| audit infra | `services/api/src/routes/health.rs` | 신규 |
| PgRepo | `crates/db/src/{user,listing,listing_photo,bookmark,analysis_report}.rs` | before_state snapshot |
| outbox | `services/outbox-publisher/src/main.rs` | Prometheus metrics + sentry init + OTLP |
| api-health | `crates/api-health-recorder/src/main.rs` | sentry capture_message |
| frontend | `apps/web/{instrumentation.ts,lib/api.ts,proxy.ts}` | Sentry init + X-Request-Id propagation |
| workspace | `Cargo.toml` | deps (sentry / tracing-opentelemetry / opentelemetry-otlp / prometheus) |
| 통합 테스트 | `services/api/tests/observability_integration.rs` 또는 인접 | 신규 |
| docs | spec + plan + roadmap + project_progress + MEMORY | 신규/갱신 |

총 ~25 파일.

---

## 위험 요소

- **5 PgRepository before_state 변경 = 광범위**: 모든 통합 테스트 영향. 기존
  audit_log assertion (예: `audit_count == 1`) 이외 추가로 *before_state 값
  검증* 필요. 기존 테스트 갱신 부담 있음
- **Sentry 0.34.x 가 sentry-tracing 0.34 와 sync**: workspace 도 같은 minor.
  나중에 upgrade 시 양쪽 같이
- **OTLP / Prometheus 1차 init only**: 외부 endpoint 부재 (Loki / Prometheus
  scraper 둘 다 SP8 IaC 가 배포). 본 SP 종료 후 *코드 path 는 production-ready*,
  *infra 는 다음 SP*
- **MutationContextBuilder extractor adoption**: 5 handlers 수정. 패턴 깨끗 —
  `Extension(auth)` + manual ctx build 라인 5+ 줄 → extractor 1 줄. 일관성 ↑
- **`__metadata__` nesting trade-off**: query 시 `WHERE after_state ->
  '__metadata__' -> 'kind' = ?` — 가능하지만 indexable X. FU 90 (별도 컬럼)
  마이그 권장
