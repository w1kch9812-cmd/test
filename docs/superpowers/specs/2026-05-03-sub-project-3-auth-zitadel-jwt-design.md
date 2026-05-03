# Sub-project 3: Auth — Zitadel JWT 핵심 게이트 (Spec)

| | |
|---|---|
| 작성일 | 2026-05-03 |
| 상태 | Approved |
| 선행 | Sub-project 1 (Charter + Monorepo), Sub-project 2 (DB + Core 도메인), [ADR-0005](../../adr/0005-auth-zitadel.md) |
| 후속 | SP4 (외부 API), SP5 (Repository SQLx 구현), SP3-ii (소셜 로그인) |

---

## 1. 개요

공짱 API 의 모든 요청이 `Authorization: Bearer <jwt>` 를 거쳐 Zitadel 발급
access_token 으로 검증되도록 핵심 인증 게이트를 구축해요. 검증된 사용자
정보는 핸들러로 주입되고, 처음 접속한 사용자는 DB 에 `User` Aggregate 가
자동 생성돼요.

본 sub-project 는 ADR-0005 가 정한 Zitadel 셀프호스트 결정을 따르되,
*인증 게이트의 최소 작동 시스템* 만 다뤄요. 소셜 로그인, NICE 본인인증,
2FA, 사업자번호 검증 등은 별도 후속 sub-project 에서 진행해요.

---

## 2. 범위 (Scope)

### 포함
- Zitadel access_token JWT 검증 미들웨어 (Axum tower layer)
- JWKS (`/oauth/v2/keys`) 페치 + 1 시간 TTL 캐시 + kid miss 시 재페치
- `User` Aggregate 자동 생성 (first sign-in)
- `Role` enum 5 종 (Buyer / Seller / Broker / Developer / Enterprise) + 가드 인프라
- `User.roles: Vec<Role>` 필드 + DB 마이그레이션 (`30005`)
- `UserRepository::find_by_zitadel_sub` 메서드
- 한국어 해요체 에러 응답 (`401` / `403` JSON 표준)
- Walking Skeleton 3 endpoint 인증 게이트 적용 (`/healthz` 만 public)
- CI 워크플로우에 진짜 Zitadel 컨테이너 통합 (e2e round-trip)

### 미포함 (후속 SP 에서 처리)
- 소셜 로그인 (Google / Kakao / Naver / Apple)
- NICE 본인인증
- 2FA (WebAuthn / TOTP)
- 사업자번호 / 공인중개사 자격 검증
- Redis 세션 (Zitadel 이 세션 보유, 우리는 stateless JWT 검증만)
- endpoint 별 권한 매트릭스 (각 endpoint 가 도입될 때 적용)
- Refresh token 처리 (frontend 책임)
- Logout / token revocation (Zitadel 측에서 처리)

---

## 3. 아키텍처

```
┌──────────────────────────────────────────────────┐
│  HTTP Request (Authorization: Bearer <jwt>)      │
└────────────────────┬─────────────────────────────┘
                     ▼
       ┌─────────────────────────────────┐
       │ Tower middleware (crates/auth)  │
       │  1. Bearer 추출                 │
       │  2. JWKS 캐시 lookup (kid)      │
       │     · 미스 → /oauth/v2/keys 페치│
       │  3. RS256 서명 검증             │
       │  4. exp / nbf / iss / aud 검사  │
       │  5. claims → sub, email, name   │
       │  6. UserRepository              │
       │     .find_by_zitadel_sub(sub)   │
       │     · None → User::try_new      │
       │       + repo.save               │
       │  7. AuthenticatedUser           │
       │     Extension 주입              │
       └────────────────┬────────────────┘
                        ▼
       ┌─────────────────────────────────┐
       │ Handler                         │
       │  Extension<AuthenticatedUser>   │
       │  → user.id, user.roles          │
       │  (선택) require_role(Role::X)   │
       └─────────────────────────────────┘
```

`/healthz` 는 public list 에 등록되어 미들웨어 우회.

---

## 4. 컴포넌트 정의

### 4.1 `crates/auth/` (새 crate, name = `auth`)

```
crates/auth/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs                  (모듈 선언 + crate-level 문서)
    ├── verifier.rs             (JwtVerifier — JWKS 페치 + 서명 검증)
    ├── jwks_cache.rs           (kid → DecodingKey 캐시, 1h TTL, lazy refetch)
    ├── claims.rs               (Claims 구조체 — sub, email, name, exp, iss, aud)
    ├── middleware.rs           (Tower layer + 함수 → Extension 주입)
    ├── extractor.rs            (AuthenticatedUser extractor for Axum)
    ├── role.rs                 (Role enum + RequireRole guard)
    └── errors.rs               (AuthError + IntoResponse)
```

#### `JwtVerifier`
```rust
pub struct JwtVerifier {
    issuer: String,        // 예: https://zitadel.gongzzang.local
    audience: String,      // OIDC client_id
    cache: JwksCache,
    http: reqwest::Client, // JWKS 페치
}

impl JwtVerifier {
    pub async fn new(issuer: String, audience: String) -> Result<Self, AuthError>;
    pub async fn verify(&self, token: &str) -> Result<Claims, AuthError>;
}
```

#### `JwksCache`
- `RwLock<HashMap<String /* kid */, (DecodingKey, Instant /* fetched_at */)>>`
- TTL 1 시간, 만료된 entry 는 재페치
- kid miss → 즉시 재페치 (한 번만)

#### `AuthenticatedUser` extractor
```rust
pub struct AuthenticatedUser {
    pub user: User,         // user-domain 의 Aggregate
    pub claims: Claims,
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = AuthError;
    async fn from_request_parts(
        parts: &mut Parts, _state: &S
    ) -> Result<Self, Self::Rejection>;
}
```

미들웨어가 이미 `Extension<AuthenticatedUser>` 를 넣어두면 extractor 는 그걸
꺼내요. 미들웨어가 인증 안 한 경로 (`/healthz`) 에서는 extractor 사용 시 401.

#### `Role` enum (shared-kernel)
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Role {
    Buyer,        // 매수자
    Seller,       // 매도자
    Broker,       // 공인중개사
    Developer,    // 시행사
    Enterprise,   // 기업 (대기업/단체 임직원)
}

impl Role {
    pub const fn as_db_str(&self) -> &'static str;  // "buyer" 등
    pub fn from_db_str(s: &str) -> Option<Self>;
}
```

#### `require_role` 함수
```rust
pub fn require_role(user: &AuthenticatedUser, role: Role) -> Result<(), AuthError> {
    if user.user.roles.contains(&role) { Ok(()) } else { Err(AuthError::InsufficientRole { needed: role }) }
}
```

핸들러 패턴:
```rust
async fn admin_only(auth: AuthenticatedUser) -> Result<..., AuthError> {
    require_role(&auth, Role::Enterprise)?;
    // ... business logic
}
```

### 4.2 `crates/domain/core/shared-kernel/` 변경
- `src/role.rs` 추가 (`Role` enum)
- `src/lib.rs` 에 `pub mod role` 노출

### 4.3 `crates/domain/core/user/` 변경
- `entity.rs` `User` 구조체에 `pub roles: Vec<Role>` 필드 추가
- `try_new` 시그니처에 `roles: Vec<Role>` 파라미터 추가 (≥0, 중복 허용 안 함 → 검증)
- `User::add_role(&mut self, role: Role)` / `remove_role` 도메인 메서드 (`version` bump 포함)
- `UserRepository` trait 에 `find_by_zitadel_sub` 메서드 추가:
  ```rust
  async fn find_by_zitadel_sub(&self, sub: &str) -> Result<Option<User>, RepoError>;
  ```
- `PgUserRepository` (db crate) 에 구현 추가

### 4.4 `migrations/30005_user_roles.sql` (새 마이그레이션)
```sql
-- V003_05: user.roles 컬럼 추가 (5종 역할)
-- spec § 6.4, sub-project 3 (Auth)

alter table "user"
    add column roles text[] not null default '{}';

-- CHECK: 모든 원소가 허용된 역할명
alter table "user"
    add constraint user_roles_valid_chk check (
        roles <@ array['buyer','seller','broker','developer','enterprise']::text[]
    );

create index user_roles_gin_idx on "user" using gin (roles);
```

### 4.5 `services/api/` 변경
- `AppState` 에 `Arc<JwtVerifier>` 추가
- `main.rs` 에서 환경 변수 `ZITADEL_ISSUER`, `ZITADEL_AUDIENCE` 로 verifier 초기화
- 라우터 구조:
  ```rust
  let public = Router::new().route("/healthz", get(health));
  let protected = Router::new()
      .route("/users/:id", get(get_user))
      .layer(auth::middleware::layer(verifier.clone(), repo.clone()));
  let app = public.merge(protected).with_state(state);
  ```
- 기존 `POST /users` 핸들러 **제거**. User 생성은 미들웨어 first-sign-in 자동 생성으로 대체. 이전에는 클라이언트가 `zitadel_sub` 를 직접 보냈지만, JWT 도입 후에는 신뢰할 수 없는 path 라 폐기.

### 4.6 CI 워크플로우 (`walking-skeleton.yml`)
- services 섹션에 Zitadel 컨테이너 추가:
  - 이미지: `ghcr.io/zitadel/zitadel:latest`
  - 환경 변수: `ZITADEL_DATABASE_POSTGRES_*`, `ZITADEL_EXTERNALSECURE=false`, `ZITADEL_TLS_ENABLED=false`
  - 의존: 같은 Postgres 컨테이너 (전용 DB 분리)
- 사전 setup 단계:
  1. Zitadel admin token 획득 (Zitadel CLI 또는 management API)
  2. OIDC 앱 (project + application) 생성 → `audience` (client_id) 확보
  3. machine user (service account) + JWT key 생성
  4. machine user JWT 발급 → `$ZITADEL_TEST_TOKEN` 환경 변수
- 테스트 단계:
  1. `curl http://localhost:8080/healthz` → 200 (인증 없이도 통과)
  2. `curl http://localhost:8080/users/usr_xxx` 토큰 없이 → 401
  3. `curl -H "Authorization: Bearer $ZITADEL_TEST_TOKEN" http://localhost:8080/users/<자동생성된 id>` → 200, body JSON 검증
  4. 두 번째 호출 → 같은 User id 반환

---

## 5. 데이터 흐름 (시퀀스)

### 5.1 정상 플로우 (인증된 요청)

```
[1] Client → frontend → Zitadel 로그인
[2] Zitadel → Client: access_token (RS256 JWT)
[3] Client → API: GET /listings
                    Authorization: Bearer <jwt>
[4] AuthMiddleware:
    a. Bearer 헤더 파싱
    b. JWT header.kid 추출
    c. JwksCache.get(kid) → 캐시 hit/miss
       · miss → reqwest GET <issuer>/oauth/v2/keys → 캐시 갱신
    d. jsonwebtoken::decode::<Claims>(jwt, &decoding_key, &Validation { algorithms: RS256, iss, aud })
    e. 검증 통과 → Claims { sub, email, name }
    f. UserRepository.find_by_zitadel_sub(&claims.sub).await?
       · Some(user) → 그대로 사용
       · None → User::try_new(Id::new(), &claims.sub, claims.email, &claims.name, UserKind::Individual, vec![], now)
                + repo.save(&user)
    g. Extension::insert(AuthenticatedUser { user, claims })
[5] Handler → Extension<AuthenticatedUser> 추출 → 비즈니스 로직
```

### 5.2 자동 생성 시 기본값
- `id`: 새 ULID
- `zitadel_sub`: claims.sub
- `email`: claims.email (Zitadel JWT 표준 claim)
- `display_name`: claims.name (없으면 claims.preferred_username, 없으면 sub 일부)
- `user_kind`: `Individual` (corporation 은 후속 SP 사업자번호 검증 단계에서 변경)
- `roles`: `vec![]` (역할 부여는 어드민이 별도 endpoint 로)
- `created_at`/`updated_at`: 현재 시각
- `version`: 1

### 5.3 동시성 (race condition)
같은 사용자가 처음 로그인 시 동시에 2 요청이 오면 양쪽 다 `find_by_zitadel_sub` → `None` → `save` 시도 → 두 번째 INSERT 가 unique 제약 위반.

대응:
- DB 레벨: `user.zitadel_sub` 에 `UNIQUE` 제약 (이미 spec § 5.1 에 있음)
- repo `save` 에서 unique violation 캐치 → `find_by_zitadel_sub` 재호출 → 기존 row 반환
- AuthMiddleware 는 이를 alias 패턴으로 추상화 (미들웨어가 race 한 번 흡수)

---

## 6. 에러 응답

### 6.1 매핑 표

| 상황 | HTTP | error_code | 메시지 (해요체) |
|---|---|---|---|
| `Authorization` 헤더 누락 | 401 | `AUTH_MISSING_TOKEN` | 인증 토큰이 필요해요 |
| Bearer 형식 아님 | 401 | `AUTH_INVALID_FORMAT` | 토큰 형식이 잘못됐어요 |
| JWT 파싱 실패 | 401 | `AUTH_MALFORMED_TOKEN` | 토큰을 해석할 수 없어요 |
| kid 매칭 키 없음 | 401 | `AUTH_UNKNOWN_KEY` | 토큰 서명 키를 찾을 수 없어요 |
| 서명 검증 실패 | 401 | `AUTH_INVALID_SIGNATURE` | 토큰이 유효하지 않아요 |
| `exp` 만료 | 401 | `AUTH_TOKEN_EXPIRED` | 토큰이 만료됐어요. 다시 로그인해 주세요 |
| `nbf` 미도래 | 401 | `AUTH_TOKEN_NOT_YET_VALID` | 토큰이 아직 사용할 수 없어요 |
| `iss` 불일치 | 401 | `AUTH_INVALID_ISSUER` | 토큰 발급자가 일치하지 않아요 |
| `aud` 불일치 | 401 | `AUTH_INVALID_AUDIENCE` | 토큰 대상이 일치하지 않아요 |
| `sub` 누락 | 401 | `AUTH_MISSING_SUBJECT` | 토큰에 사용자 정보가 없어요 |
| User 자동 생성 실패 | 500 | `AUTH_USER_PROVISION_FAILED` | 사용자 등록에 실패했어요. 잠시 후 다시 시도해 주세요 |
| 권한 부족 | 403 | `AUTH_INSUFFICIENT_ROLE` | 이 작업을 수행할 권한이 부족해요 |

### 6.2 응답 본문
```json
{
  "error_code": "AUTH_TOKEN_EXPIRED",
  "message": "토큰이 만료됐어요. 다시 로그인해 주세요"
}
```

### 6.3 로깅
- 401/403 응답은 `tracing::warn!` 로 기록 (sub claim, error_code, 요청 path)
- raw 토큰은 절대 로깅하지 않음 (개인정보)
- 5xx 는 `tracing::error!` + 스택트레이스

---

## 7. JWT 검증 상세

### 7.1 알고리즘
- **RS256 만 허용**. Zitadel 기본. `none`/`HS*` 은 거부 (downgrade attack 방지)

### 7.2 검증 항목
- `alg = RS256`
- `kid` 가 JWKS 캐시에 존재
- 서명 (RSA public key)
- `exp > now` (clock skew 허용 30 초)
- `nbf <= now` (있을 경우)
- `iss == ZITADEL_ISSUER` 환경 변수 값
- `aud` 에 `ZITADEL_AUDIENCE` 포함 (배열 또는 문자열 모두 허용)
- `sub` 비어있지 않음

### 7.3 JWKS 캐시 정책
- 첫 요청 시 lazy fetch
- 1 시간 TTL
- kid 미스 → 즉시 재페치 (회전 대응)
- 페치 실패 → 에러 (`AUTH_INVALID_SIGNATURE`)
- 백그라운드 갱신 없음 (요청 시 lazy)

---

## 8. RBAC 모델 (인프라만)

### 8.1 5 종 역할
- `Buyer` — 매수자 (산업용 부동산 매입 예정자)
- `Seller` — 매도자 (매물 등록자)
- `Broker` — 공인중개사
- `Developer` — 시행사 (분양 사업자)
- `Enterprise` — 기업 (대기업/단체 임직원, 향후 multi-tenant)

### 8.2 저장 위치
- DB: `user.roles text[]` (5 종 중 0 개 이상 보유 가능)
- JWT custom claim: 미사용 (DB 가 단일 출처)
- Zitadel role/scope 매핑은 SP3-ii (소셜 로그인) 또는 후속에서 정함

### 8.3 가드 레벨
- 본 sub-project 는 `require_role(role)` 함수와 `RequireRole` extractor 인프라만 제공
- 실제 endpoint 별 매트릭스는 endpoint 가 도입될 때 적용
- Walking Skeleton 의 `/users/:id` 는 인증만 요구, 역할 체크 없음

---

## 9. 환경 변수

| 변수 | 예시 | 용도 |
|---|---|---|
| `ZITADEL_ISSUER` | `https://zitadel.gongzzang.local` | iss claim 검증 |
| `ZITADEL_AUDIENCE` | `<oidc-client-id>` | aud claim 검증 |
| `DATABASE_URL` | `postgres://...` | 기존 (User 조회용) |

미설정 시 init 단계 `panic` (구성 실수를 런타임 외부 동작 전에 차단).

---

## 10. 테스트 전략

### 10.1 단위 테스트 (`crates/auth/`)
- `JwksCache`: hit / miss / TTL 만료 / 동시 lookup (≥6 tests)
- `JwtVerifier::verify`:
  - 정상 토큰 → Claims 반환
  - 만료된 토큰 → `AUTH_TOKEN_EXPIRED`
  - 서명 잘못된 토큰 → `AUTH_INVALID_SIGNATURE`
  - kid 미스 + JWKS 페치 실패 → 에러
  - iss/aud 불일치 (≥10 tests)
- `Role::as_db_str` / `from_db_str` 라운드트립 (5 tests)
- `require_role` 통과/실패 (≥3 tests)
- `AuthError` → `IntoResponse` (HTTP status + JSON body, ≥10 tests)

목표: ≥40 tests, ≥90% 커버리지

### 10.2 단위 테스트 (`crates/domain/core/user/`)
- `User::try_new` 와 `roles` 필드 포함/검증 (≥4 추가)
- `add_role` / `remove_role` 도메인 메서드 (≥6 추가)
- `UserRepository` trait `find_by_zitadel_sub` 시그니처 + object safety (≥1)

### 10.3 통합 테스트 (`services/api`)
- `tests/auth_middleware_integration.rs`:
  - 토큰 없이 보호된 endpoint → 401
  - 잘못된 토큰 → 401
  - 만료된 토큰 → 401
  - 유효한 토큰 + 첫 호출 → User 자동 생성, 200
  - 같은 토큰 재호출 → 같은 User id, 1 row in DB
  - `/healthz` 토큰 없이 → 200
- mock JwtVerifier (test feature flag) 또는 testcontainers Zitadel

### 10.4 CI E2E (`walking-skeleton.yml`)
- 진짜 Zitadel 컨테이너 부팅
- machine user JWT 로 round-trip
- 401/200 시나리오 검증

---

## 11. 검증 기준 (DoD)

본 sub-project 는 다음을 모두 만족해야 종료:

1. `crates/auth/` 신규 crate, ≥40 단위 테스트, 커버리지 ≥90%
2. `User` Aggregate 에 `roles` 필드 + `find_by_zitadel_sub` repository 메서드 추가, 기존 테스트 + 신규 테스트 모두 그린
3. `migrations/30005_user_roles.sql` 적용, `db-migrations.yml` CI 그린
4. `services/api` 미들웨어 적용, `POST /users` 제거
5. `.github/workflows/walking-skeleton.yml` Zitadel 컨테이너 통합, e2e round-trip 그린
6. 3 CI 워크플로우 (CI / db-migrations / walking-skeleton) 모두 그린
7. 누적 단위 테스트 ≥1080 (1017 + 60+)
8. `tarpaulin` ≥90% 커버리지 게이트 통과
9. `cargo clippy --workspace --all-targets -- -D warnings` 통과
10. `cargo deny check` 통과
11. 모든 파일 ≤500 권장 / ≤1500 강제 만족

---

## 12. SSS 7 기둥 매핑

| 기둥 | SP3 적용 |
|---|---|
| 1 일관성 | 모든 protected endpoint 가 동일한 미들웨어로 보호. 예외 0 |
| 2 자동 강제 | default deny — public 명시 안 한 endpoint 는 인증 필수. 컴파일러 + tarpaulin 게이트 |
| 3 추적성 | 401/403 로깅 + sub claim. correlation_id 는 SP7 (관측성) 에서 추가 |
| 4 안전성 | RS256 만 허용 (downgrade 차단), `Role` enum (오타 차단), `Id<UserMarker>` (타입 차단), 환경 변수 미설정 시 init panic |
| 5 가시성 | 401/403 `tracing::warn!`, 5xx `tracing::error!`. raw 토큰 절대 로깅 안 함 |
| 6 SSOT | User.roles 는 DB 단일 출처. JWT custom claim 안 씀 |
| 7 명확성 | error_code (`AUTH_*`), 한국어 해요체 메시지, 환경 변수명 명시적 |

---

## 13. Follow-up items (production 배포 전)

1. **Zitadel 키 회전 운영 검증** — 실제 운영에서 1 시간 캐시가 적정한지, 회전 시 grace period 동작 확인
2. **Refresh token 처리** — frontend (SP6) 가 access_token 만료 전에 refresh 하도록 가이드 필요
3. **Logout 흐름** — Zitadel `/end_session` endpoint 호출 + frontend cookie 삭제 가이드
4. **endpoint 별 RBAC 매트릭스** — endpoint 가 도입될 때 spec 에 *어떤 role 이 필요한지* 명시 필수
5. **multi-tenant 분리** — Enterprise role 사용자가 자기 조직 데이터만 보도록 ABAC 추가 (SP3-iii 또는 SP6)
6. **Rate limit** — 인증 미통과 요청에 대한 IP 기반 rate limit (SP7)

---

## 14. 후속 sub-project 시드

- **SP3-ii**: 소셜 로그인 (Google / Kakao / Naver / Apple) — Zitadel external IdP 설정
- **SP3-iii**: NICE 본인인증 통합 (별도 PG 결제·휴대폰 인증)
- **SP3-iv**: 2FA (WebAuthn / TOTP) — Zitadel 측 설정
- **SP4**: 외부 API 통합 (V-World, 법제처, data.go.kr) — 인증된 사용자 컨텍스트로 호출
