# Sub-project Observability — Production-grade 관측성 + Audit chain hardening (Spec)

| | |
|---|---|
| 작성일 | 2026-05-06 |
| 상태 | Draft |
| 선행 | SP3 (Auth) / SP5-iii/iv (MutationContext + audit_log/outbox 패턴) / SP7-iii (drift detector) |
| 후속 | SP8 IaC Pulumi (Sentry/Loki/Prometheus stack 배포) |
| 추정 | 8 task, 3-4일 |

---

## 1. 개요

현재 SSS 7 기둥 검증 시 **§ 2 자동강제, § 3 추적성, § 5 가시성** 3 개가
production-grade 결격. 본 SP 가 한 번에 닫음:

**현재 결격 사례 (감사)**:

1. **`audit_log.before_state` = `NULL` 항상** — diff 추적 불가. "왜 이 매물이
   변경됐는가" 의 *before* 모름.
2. **`audit_log.ip_address`/`user_agent` = `NULL` 항상** — `MutationContext` 에
   필드는 있으나 handler 가 채우지 않음. 보안 incident 후 분석 불가.
3. **`correlation_id` = `cor_<ULID>` 자동 생성** — client → API → DB → outbox
   체인 간 ID 끊김. 외부 (Zitadel / data.go.kr) 응답 trace 불가.
4. **panic / error 침묵** — Sentry 미통합. 사용자가 신고해야 알게 됨.
5. **outbox publisher metrics 없음** — published / failed / lag 모름.
6. **health check 단일 endpoint** (`/healthz`) — DB / Redis 다운 시 silent.
7. **drift detector alert (SP7-iii)** — Issue 만 생성. realtime 알림 X.

**SSS 4 기둥 동시 close**:

| 기둥 | 본 SP 가 닫는 것 |
|---|---|
| § 2 자동강제 | panic/error → Sentry alert. drift → routed alert. health probe 자동 |
| § 3 추적성 | before_state 보존. ip/ua 자동. X-Request-Id 전 체인 propagation |
| § 4 안전성 | incident 분석 자료 풍부. health check failover 가능 |
| § 5 가시성 | tracing → Loki, metrics → Prometheus, errors → Sentry — production 운영 가능 |

---

## 2. 범위

### 포함

#### A. X-Request-Id correlation chain (T2)

**Axum middleware** (`crates/auth/src/request_id.rs` 신규 또는 `services/api/src/http/`):

- 요청 들어오면 `X-Request-Id` header 검사. 있으면 사용, 없으면 `req_<ULID>` 생성
- `req.extensions_mut().insert(RequestId(...))` 으로 downstream extractor 주입
- 응답 header 에 `X-Request-Id: <id>` 자동 echo (debugging UX)
- `tracing::Span::record("request_id", ...)` 으로 모든 trace 에 자동 attach

**Next.js proxy 갱신** (`apps/web/proxy.ts`):

- inbound `X-Request-Id` 있으면 propagate, 없으면 frontend 가 생성
- `NEXT_PUBLIC_API_BASE_URL` 호출 시 header 자동 추가 (api client `ky`
  beforeRequest hook)
- 응답 header 받으면 `tracing` 또는 Sentry context tag 로 보존

**MutationContext 통합**: `http_user_action(auth, action)` 헬퍼가 `X-Request-Id`
extension 을 읽어 `correlation_id` 로 사용 — 자동 생성 (현재) 대신 *전체 체인
공유* ID. 외부 호출 (V-World / data.go.kr) 시에도 `X-Request-Id` outbound
header 추가.

#### B. MutationContext auto-inject middleware (T3)

신규 Axum extractor `MutationContextBuilder`:

```rust
// services/api/src/http/mutation_ctx.rs (확장)

pub struct MutationContextBuilder {
    pub auth: AuthenticatedUser,
    pub correlation_id: String,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
}

impl<S: Send + Sync> FromRequestParts<S> for MutationContextBuilder { ... }

impl MutationContextBuilder {
    pub fn build(self, action: &str) -> MutationContext {
        MutationContext::new_user_action(self.auth.user.id, self.correlation_id, action)
            .with_client_info_optional(self.client_ip, self.user_agent)
    }
}
```

핸들러 1줄로:

```rust
pub async fn create_listing(
    State(state): State<ListingsState>,
    builder: MutationContextBuilder,
    Json(body): Json<CreateListingRequest>,
) -> Result<...> {
    // ...
    let ctx = builder.build("create_listing");
    repo.save(&listing, ctx).await?;
}
```

이러면 `auth + correlation + client_ip + user_agent` 모두 자동 — 핸들러가 잊어도
시스템이 채움 (§ 2 자동강제).

`X-Forwarded-For` header 우선 (proxy 통과 시), 없으면 `ConnectInfo<SocketAddr>`.
`User-Agent` header 그대로 (≤500자 trim).

#### C. AuditLog `before_state` capture (T4)

**핵심 변경** — PgRepository `save` 가 *before snapshot* 을 같은 tx 안에서 SELECT 후 INSERT:

```rust
// 패턴 (PgListingRepository::save 등):

// 1. tx 시작
// 2. SELECT 현재 row (없으면 None) → before_state JSON
// 3. UPSERT new aggregate
// 4. SELECT updated row → after_state JSON
// 5. INSERT audit_log (before_state, after_state, ip, ua, ...)
// 6. INSERT outbox_event for each ctx.events
// 7. commit
```

영향 받는 PgRepository (8 개): User / Listing / ListingPhoto / Bookmark
(listing+external) / SearchHistory / AnalysisReport / Notification / 기타 audit
대상.

**Trade-off**: SELECT 추가 cost — 단일 row 라 무시 가능 (~0.5ms). batch 작업
(예: `mark_all_read_by_kind`) 은 *aggregate state 가 individual row 가 아니므로*
별도 — `metadata` 에 `rows_affected` 만 (현재 패턴 유지).

**`metadata` 보관 위치 결정**:

- 1차 = `metadata` 를 `after_state` 의 `__metadata__` key 안에 nest (schema
  변경 없이). aggregate JSON 에 reserved key.
- 후속 = V003_08 마이그로 별도 `metadata jsonb` 컬럼 (FU 90).

#### D. Sentry init (T5)

**Rust** (`services/api/src/main.rs`, `services/outbox-publisher/src/main.rs`,
`crates/api-health-recorder/src/main.rs`):

- `sentry` crate (workspace dep)
- `SENTRY_DSN` env (없으면 disabled)
- `release` = `env!("CARGO_PKG_VERSION")` + `git rev-parse --short HEAD`
  (build-time set)
- `environment` = `APP_ENV` env (`production` / `staging` / `dev`)
- `before_send` filter — `correlation_id` 자동 tag attachment
- panic hook 자동 capture
- `tracing::error!` ↔ Sentry `capture_message` integration

**Next.js** (`apps/web/`):

- `@sentry/nextjs` install (이미 instrumentation.ts placeholder 있음)
- `SENTRY_DSN` (`NEXT_PUBLIC_SENTRY_DSN`) env
- source maps upload 는 SP8 IaC 단계 (CI 배포 step)

**1차 = init only**. 실 DSN 미설정 시 silent disabled (개발 환경 호환). production
은 SP8 IaC 가 DSN 주입.

#### E. tracing → OTLP exporter (T6 — 골격만)

**Rust**:

- `tracing-opentelemetry` + `opentelemetry-otlp` workspace dep
- `OTLP_ENDPOINT` env (없으면 disabled)
- `tracing_subscriber` Registry 에 `OpenTelemetryLayer` add
- service.name = `api` / `outbox-publisher` / `api-health-recorder`

**1차 = init only**. production 은 SP8 IaC 가 endpoint 주입 (Loki via Grafana
Agent 또는 OTLP collector).

#### F. Outbox publisher Prometheus metrics (T7)

`services/outbox-publisher/src/main.rs`:

- `prometheus` crate
- `outbox_published_total{aggregate_kind}` Counter
- `outbox_failed_total{aggregate_kind}` Counter
- `outbox_tick_duration_seconds` Histogram
- `outbox_lag_seconds` Gauge (oldest unpublished row age)
- `/metrics` HTTP endpoint (binding `0.0.0.0:9091`)

**1차 = local in-process metrics**. production 은 Prometheus scraper 가 9091 가져감.

#### G. Health check 강화 (T7)

`services/api/src/routes/health.rs` (확장):

- `/healthz` — liveness (process up). 항상 200
- `/healthz/ready` — readiness (db ping + redis ping if configured). 다운 시 503
- `/healthz/db` — DB 단독 (debug)

K8s/ECS 가 `liveness` (재시작 trigger) ↔ `readiness` (트래픽 cut) 구분 필요.

#### H. SP7-iii drift detector → Sentry alert routing (T7)

`crates/api-health-recorder` 수정:

- `octocrab` Issue 생성 후 *추가* — `sentry::capture_message(Level::Error)` 발화
- Sentry alert 룰 → on-call 채널 (PagerDuty / Slack — 운영자 책임, 코드 X)

### 미포함

- **Sentry source map upload CI step**: SP8 IaC
- **Loki / Prometheus stack 배포**: SP8 IaC
- **PagerDuty / Slack 통합**: 운영자 책임 (env-driven)
- **Distributed tracing across V-World / data.go.kr 외부 API**: outbound
  `X-Request-Id` 만. 외부가 echo 안 하면 trace 끊김 — 정부 API 한계
- **AuditLog full diff UI** (관리자 페이지): SP7-i 또는 별도
- **Performance metrics on every endpoint**: 별도 (FU 95)
- **structured log to JSON 강제** (현재는 `tracing` default): 1차는 OK

---

## 3. 컴포넌트

### 3.1 `MutationContextBuilder` extractor

```rust
// services/api/src/http/mutation_ctx.rs (확장)

#[derive(Debug, Clone)]
pub struct RequestId(pub String);

#[async_trait]
impl<S> FromRequestParts<S> for MutationContextBuilder
where
    S: Send + Sync,
{
    type Rejection = ProblemResponse;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth = Extension::<AuthenticatedUser>::from_request_parts(parts, _state)
            .await
            .map_err(|_| problem(...))?
            .0;

        let correlation_id = parts
            .extensions
            .get::<RequestId>()
            .map_or_else(|| format!("req_{}", Ulid::new()), |r| r.0.clone());

        let client_ip = extract_client_ip(parts);
        let user_agent = parts
            .headers
            .get("user-agent")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.chars().take(500).collect::<String>());

        Ok(Self { auth, correlation_id, client_ip, user_agent })
    }
}
```

`extract_client_ip` 가 `X-Forwarded-For` first (production proxy 환경), fallback
`ConnectInfo<SocketAddr>`.

### 3.2 X-Request-Id Axum middleware

```rust
// services/api/src/http/request_id.rs (신규)

pub async fn request_id_layer(mut req: Request<Body>, next: Next) -> Response {
    let id = req.headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map_or_else(|| format!("req_{}", Ulid::new()), str::to_owned);

    req.extensions_mut().insert(RequestId(id.clone()));

    // Span attach
    let span = tracing::info_span!("http_request", request_id = %id);
    let _enter = span.enter();

    let mut response = next.run(req).await;
    response.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(&id).unwrap_or_else(|_| HeaderValue::from_static("invalid")),
    );
    response
}
```

`main.rs` 의 모든 router 에 `.layer(middleware::from_fn(request_id_layer))` add.
auth_layer 보다 먼저 — 인증 실패해도 trace ID 부여.

### 3.3 `MutationContext::with_client_info_optional`

`shared-kernel` mutation 모듈 보강:

```rust
impl MutationContext {
    /// `Option<String>` 받아 둘 다 `Some` 일 때만 set. 둘 중 하나만 있으면 부분 set.
    #[must_use]
    pub fn with_client_info_optional(
        mut self,
        ip: Option<String>,
        ua: Option<String>,
    ) -> Self {
        self.client_ip = ip;
        self.user_agent = ua;
        self
    }
}
```

### 3.4 PgRepository `before_state` snapshot 패턴

기존 패턴 (User 예시):

```rust
async fn save(&self, user: &User, ctx: MutationContext) -> Result<(), RepoError> {
    let mut tx = self.pool.begin().await?;
    
    // [신규] 1. before snapshot
    let before_state = read_user_as_json(&mut tx, &user.id).await?;
    
    // 2. UPSERT (기존)
    upsert_user(&mut tx, user).await?;
    
    // [신규] 3. after snapshot — 신뢰 가능한 직후 read 또는 직접 user 직렬화
    let after_state = serde_json::to_value(user).ok();
    
    // 4. audit_log INSERT — before_state + after_state 모두 보존
    insert_audit_log(&mut tx, user.id.as_str(), &ctx, before_state, after_state).await?;
    
    // 5. outbox_event (기존)
    // 6. commit
}
```

`read_user_as_json` 헬퍼 — `SELECT row_to_json(...)` 사용해 단일 query.
`row_to_json` 가 NULL 처리 / postgres `inet`/`enum`/`array` 타입을 안전하게 JSON
화 (sqlx 가 native rust 타입 변환 안 거치고 db 가 JSON serialize).

**8 PgRepository 영향 (T4 작업)**:

| Repo | save 시 before/after | delete 시 before/after |
|---|---|---|
| User | both | n/a (no delete) |
| Listing | both | n/a |
| ListingPhoto | both | before, after=NULL |
| BookmarkListing | both | before, after=NULL |
| BookmarkExternal | both | before, after=NULL |
| SearchHistory | n/a (insert-only) | n/a |
| AnalysisReport | both | before, after=NULL |
| Notification | n/a (insert-only) | n/a (mark_read 가 update) |

mark_read / mark_all_read_by_kind / increment_view_count 는 별도 — 현재 metadata
에 rows_affected 만. before_state 의미 모호 → 패턴 유지.

### 3.5 Sentry init Rust

```rust
// services/api/src/main.rs (extension)

let _sentry_guard = init_sentry();

fn init_sentry() -> Option<sentry::ClientInitGuard> {
    let dsn = std::env::var("SENTRY_DSN").ok()?;
    let release = format!(
        "{}@{}",
        env!("CARGO_PKG_NAME"),
        std::env::var("GIT_SHA").unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_owned())
    );
    let env = std::env::var("APP_ENV").unwrap_or_else(|_| "dev".to_owned());

    Some(sentry::init(sentry::ClientOptions {
        dsn: dsn.parse().ok(),
        release: Some(release.into()),
        environment: Some(env.into()),
        traces_sample_rate: 0.1,  // 10% — production 트래픽 시 조절
        ..Default::default()
    }))
}
```

`tracing-subscriber` 에 `sentry-tracing::layer()` 추가 — error level → Sentry.

### 3.6 Health check 강화

```rust
// services/api/src/routes/health.rs (신규 또는 확장)

pub async fn liveness() -> StatusCode {
    StatusCode::OK
}

pub async fn readiness(
    State(state): State<HealthState>,
) -> Result<StatusCode, ProblemResponse> {
    // DB ping
    sqlx::query("select 1").fetch_one(&state.pool).await
        .map_err(|e| problem("not-ready", "db unreachable", StatusCode::SERVICE_UNAVAILABLE, ...))?;

    // Redis ping (optional — REDIS_URL 미설정 시 skip)
    if let Some(redis_pool) = &state.redis_pool {
        let mut conn = redis_pool.get().await.map_err(...)?;
        redis::cmd("PING").query_async(&mut conn).await.map_err(...)?;
    }

    Ok(StatusCode::OK)
}
```

---

## 4. 검증 기준 (DoD)

1. `RequestId` extension + middleware + 응답 echo + tracing span propagation —
   2 단위 테스트 + 1 통합 테스트 (header echo)
2. `MutationContextBuilder` extractor + 4 단위 테스트 (auth 없음 / xff / direct ip / ua trim)
3. `MutationContext::with_client_info_optional` + 단위 테스트 2
4. PgRepository `before_state` capture — 8 통합 테스트 (각 repo 1 sample, before
   state 가 update 시 prev value 일치 검증)
5. metadata `__metadata__` nesting — 1 통합 테스트
6. Sentry init helpers — 단위 테스트 4 (DSN 없음 / 있음 / release tagging /
   environment)
7. Health check — `/healthz` 200 / `/healthz/ready` 503 (db down) / 200 (정상)
8. workspace clippy `--all-targets` 그린
9. push → 5 CI workflow 그린
10. SSOT 갱신

---

## 5. SSS 7기둥

| 기둥 | 적용 |
|---|---|
| 1 일관성 | 모든 PgRepository 가 같은 before/after snapshot 패턴. handler 가 `MutationContextBuilder` extractor 1줄 |
| 2 자동강제 | panic/error → Sentry. health check 자동 probe. MutationContext 자동 채워짐 (잊을 수 없음) |
| 3 추적성 | request_id 전 체인 + before_state + ip + ua. compliance 가능 |
| 4 안전성 | incident 분석 자료 풍부. health check 가 트래픽 cut. drift 자동 alert |
| 5 가시성 | tracing → Loki, metrics → Prometheus, errors → Sentry, X-Request-Id 응답 echo |
| 6 SSOT | audit_log 가 진실의 단일 위치. metadata `__metadata__` nesting 결정 명시 |
| 7 명확성 | liveness vs readiness 구분. 외부 API trace 끊김 명시 (정부 API 한계) |

---

## 6. Follow-up

- **FU 90**: `audit_log.metadata jsonb` 별도 컬럼 (V003_08 마이그). 현재
  `__metadata__` nesting 은 임시 — 별도 컬럼이 query 효율 ↑
- **FU 91**: Sentry source maps upload CI step (SP8 IaC)
- **FU 92**: Distributed tracing trace context 외부 API 가 echo 하도록 합의
  (V-World / data.go.kr 협의)
- **FU 93**: AuditLog 관리자 viewer (admin 페이지)
- **FU 94**: PagerDuty / Slack 알림 routing (Sentry → 운영자 채널)
- **FU 95**: Performance metrics 모든 endpoint (latency P50/P95/P99)
- **FU 96**: log retention 정책 (Loki 30 day / cold archive 1y)

---

## 7. Risk

- **`row_to_json` overhead**: PostgreSQL 가 row 마다 JSON serialize — 단일 row
  는 ~ms 단위. 대량 batch (mark_all_read_by_kind) 는 metadata 만 (현재 유지)
- **before_state cost**: SELECT 추가 → 매 mutation 0.5ms 추가. P95 + 0.5ms.
  user impact 무시 가능
- **Sentry production load**: `traces_sample_rate: 0.1` 으로 시작. 트래픽 ↑ 시
  조절. dynamic sampling 은 Sentry SDK 내장
- **X-Forwarded-For trust**: production 에서는 *trusted proxy hop count* 정의
  필요 (마지막 1 IP 가 client). SP8 IaC 의 ALB / Cloudflare 설정과 묶임.
  1차는 `xff.split(',').first().trim()` 단순 — production 가기 전 trust boundary
  확인
- **MutationContext field 추가시 schema 변경**: 1차는 `metadata` `__metadata__`
  nesting (schema 변경 0). 추후 V003_08 마이그 (FU 90) 가 `metadata` 컬럼 추가
- **Health check leak**: `/healthz/db` 가 DB 정보 leak 가능 — production 에서는
  *공개* 만, *상세* 는 internal network 또는 admin-auth 보호 (FU)
