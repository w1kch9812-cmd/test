# ADR 0023 — Audit 2026-05-08 hardening (auth/raw-capture/JTI/observability)

| | |
|---|---|
| 작성일 | 2026-05-08 |
| 상태 | Accepted (partial — 잔여 항목 handoff) |
| 선행 | Codex `/codex:rescue` audit 2026-05-08 (session `019e0525`) |

## 후속 상태 (2026-05-28)

Raw-capture 관련 handoff 는 ADR 0034 및 Platform Core M3.2 physical extraction 으로
superseded 되었다. `PgRawCapture`, `R2RawCapture`, `raw_capture_sync`,
`raw-capture-client` 는 gongzzang workspace 에 존재하면 안 되며, Catalog raw lineage 는
Platform Core 책임이다.

## 결정

Codex audit (2026-05-08) 의 **15 finding 중 7건 즉시 fix + 8건 박제 후 handoff**. 즉시 fix = 보안 critical + observability major + 안전한 minor. handoff = 큰 refactor (file split, startup `Result<>` 패턴) + 외부 작업 (`.env` rotation, infra OIDC).

## 컨텍스트

`/codex:rescue` 가 read-only 모드에서 발견한 finding:

- **Critical (5)**: `.env` 노출 / `/internal/auth/event` 무인증 / NoOp raw capture production 사용 / NoOp building reader empty fallback / `_mapbox` private API
- **Major (6)**: startup `expect`/`unwrap` (api + outbox) / JTI fail-open / 4 파일 500줄 초과 / silent map errors
- **Minor (4)**: `XXX` marker / inline style / React return type / `unknown as` cast

Codex sandbox = read-only → 직접 fix 불가 → Claude 가 verify-then-fix path 진행.

## 즉시 fix (7건)

| # | finding | fix | 파일 |
|---|---|---|---|
| Critical 1 | `/internal/auth/event` 무인증 — audit log 임의 inject 가능 | `X-Internal-Auth` header *constant-time* 검증 (`subtle::ConstantTimeEq`), production 에서 secret 미설정 시 startup fail-fast, BFF 측 `emitAuthEvent` SSOT helper (3 routes 통합) | `services/api/src/routes/auth_event.rs`, `services/api/src/main.rs`, `apps/web/lib/auth/internal-event.ts`, `apps/web/lib/env.ts`, `apps/web/app/api/auth/{callback,logout,refresh}/route.ts` |
| Critical 2 | `NoOpRawCapture` production 사용 — `raw_response JSONB` 미저장 위반 | production env (`APP_ENV`/`NODE_ENV`) 에서 `VWORLD_API_KEY` 미설정 시 `fail_fast_production`. NoOp 자체 폐기는 `PgRawCapture` wire 후속 (TODO 박제) | `services/api/src/main.rs` |
| Critical 3 | `NoOpBuildingRegisterReader` empty fallback — silent data loss | production 에서 `DATA_GO_KR_API_KEY`/`ODP_SERVICE_KEY` 둘 다 없으면 `fail_fast_production`. live reader wire 는 SP4-iii-a 후속 | `services/api/src/main.rs` |
| Major 3 | JTI denylist Redis error → fail-open (production 보안 risk) | `AuthState.fail_closed_on_denylist_error` 신규 — production 시 Redis error → 401 (fail-closed). dev 는 fail-open 유지 (UX) | `crates/auth/src/middleware.rs`, `services/api/src/main.rs` |
| Major 6 | `listing-map.tsx` silent swallow — production 에서 *왜 폴리곤 안 보이는지* 불가시 | `logMapLayerFailure` helper — JSON structured log + kind (core/optional) → log aggregation grep 가능 | `apps/web/components/listings/listing-map.tsx` |
| Minor 1 | `migrations/10001_core_tables.sql` 의 `XXX-XX-XXXXX` 주석 (banned `XXX` marker) | `format 000-00-00000` 으로 변경 | `migrations/10001_core_tables.sql` |
| Minor 2+3 | `loading.tsx` inline style + `Promise<React.ReactElement>` 반환 타입 누락 | token class `text-[var(--color-muted-fg)]` + 명시 타입 | `apps/web/app/(authenticated)/listings/loading.tsx` |

## 박제 후 handoff (8건)

| # | finding | handoff 사유 |
|---|---|---|
| Critical (env) | `.env` 의 V-World/R2/Naver/V-World account secrets workspace 노출 | 사용자 작업 — 외부 서비스 dashboard 에서 rotation. 코드 path 0. (`docs/superpowers/handoff/2026-05-08-codex-audit-handoff.md`) |
| Critical 5 | `_mapbox` private API | ADR 0019 박제 — Naver SDK architectural limitation. fail-safe (polling timeout + raster fallback) 으로 mitigation. 우회 = Naver SDK 폐기 = 1-3개월 |
| Major 1 | `services/api/main.rs` startup `expect`/`unwrap` (workspace `expect_used/unwrap_used = deny` 위반) | startup 전체를 `Result<...>` 패턴으로 refactor — 큰 작업. 현재 `fail_fast_production` helper 가 *부분 fix* (env 검증 path 만). 잔여 = pool/listener/verifier init 의 `.expect(...)` 들 |
| Major 2 | `services/outbox-publisher/main.rs` startup `expect` | 동일 — Result 패턴 refactor |
| Major 4 | `routes/listings.rs` 1428줄 (500 권장 초과) | 모듈 분할 = 별도 sub-project (file split). AGENTS.md § 1 의 1500 *절대* 한도 안 |
| Major 5 | `etl-base-layer/main.rs` 753 + `r2_upload.rs` 628 + `parser.rs` 788 줄 | 동일 — 별도 split refactor. cognitive complexity 4건은 본 commit 에서 `#[allow]` + 사유 박제 |
| Minor 4 | `panel/registry.ts` 의 `unknown as` cast | factory + variance erasure 패턴 (gongzzang-develop reference 차용) — 의도된 framework design. `tsd` type-level test 추가 = 별도 작업 |
| Minor 4b | `apps/web/app/api/proxy/[...path]/route.ts:75` `unknown as` | 외부 API 응답 narrow — schema guard 추가 = 별도 작업 |

## 검증 (이번 commit)

- `cargo clippy --workspace --all-targets -- -D warnings` ✅ 그린
- `pnpm typecheck` (apps/web) ✅ 그린
- `pnpm lint` (apps/web) ✅ 그린 (17 warnings = e2e probe + listing-map structured log 의 의도된 console.*)

## 영향

### 신규
- `apps/web/lib/auth/internal-event.ts` — `emitAuthEvent` SSOT helper
- `docs/adr/0023-audit-2026-05-08-hardening.md` (본 파일)

### 수정
- `services/api/Cargo.toml` — `subtle = "2"` (constant-time secret 비교)
- `services/api/src/main.rs` — `fail_fast_production` helper + 3 부위 fail-fast
- `services/api/src/routes/auth_event.rs` — `X-Internal-Auth` header 검증
- `crates/auth/src/middleware.rs` — `fail_closed_on_denylist_error` field
- `apps/web/lib/env.ts` — `INTERNAL_AUTH_SECRET` zod schema
- `apps/web/app/api/auth/{callback,logout,refresh}/route.ts` — `emitAuthEvent` helper 사용
- `apps/web/components/listings/listing-map.tsx` — `logMapLayerFailure` helper
- `apps/web/app/(authenticated)/listings/loading.tsx` — token class + return type
- `migrations/10001_core_tables.sql` — `XXX` marker 폐기
- `services/etl-base-layer/src/main.rs` — `#[allow(clippy::cognitive_complexity)]` 4 함수 (audit defer)
- `.env`, `apps/web/.env.local`, `apps/web/.env.local.example` — `INTERNAL_AUTH_SECRET` placeholder

## 후속 (handoff 작업)

1. **`.env` secret rotation** — V-World API/account, R2 keys, Naver client secret 모두 외부 dashboard 에서 회전 (사용자 작업)
2. **startup `Result<...>` 패턴 refactor** — `fn main() -> Result<(), AppError>`, `pool/listener/verifier` 모두 `?` 전파, audit-defer 박제 폐기
3. **PgRawCapture wire** — `services/api/src/main.rs` 의 `NoOpRawCapture::new()` → `PgRawCapture::new(pool.clone())` 교체. `parcel_external_data.source` CHECK 정합 검증 필수
4. **DataGoKr live reader wire** — `NoOpBuildingRegisterReader` → SP4-iii-a 의 `DataGoKrBuildingReader` 교체
5. **file split** — `routes/listings.rs` (1428) / `etl-base-layer/main.rs` (753) / `r2_upload.rs` (628) / `parser.rs` (788) → 모듈 분리
6. **`unknown as` cleanup** — `panel/registry.ts` 는 `tsd` type-level test 추가, `proxy route` 는 zod schema guard

## 참고

- Codex session: `019e0525-dbfa-7f22-84fe-9fbccee36f3f` (read-only audit)
- audit findings dump: `node codex-companion.mjs result task-mow81q9n-36jgec`
- handoff doc: `docs/superpowers/handoff/2026-05-08-codex-audit-handoff.md` (작성 예정)
> Current status (2026-05-28): Historical raw-capture and Catalog observability
> findings are superseded by ADR 0034 and Platform Core M3.2 extraction. Do not
> implement `PgRawCapture`, `R2RawCapture`, `raw_capture_sync`, or local Catalog
> API health paths in Gongzzang.
