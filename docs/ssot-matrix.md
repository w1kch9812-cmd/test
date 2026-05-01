# SSOT 매트릭스

> 정보별 진짜 SSOT(Single Source of Truth)와 그것의 사본, 그리고 SSOT 위반을 자동 차단하는 룰.

---

## 1. SSOT 매트릭스 표

| 정보 종류 | 진짜 SSOT | 사본 (재구성 가능) | 위반 자동 차단 |
|---------|---------|---------|---------|
| **사용자 데이터** | PostgreSQL `user` 테이블 | Redis 세션, 검색 인덱스 | DB 외 직접 변경 금지 (linter) |
| **공공 API raw 응답** | DB의 `raw_response JSONB` | Redis 캐시, 분석 마트 | 컬럼 누락 검증 (sqlx 스키마) |
| **비즈니스 규칙** | `crates/domain/*` Rust 코드 | 문서, 테스트 (둘 다 코드 따라옴) | 도메인 외부 비즈니스 로직 = clippy lint |
| **API 계약** | Rust 코드 + utoipa 매크로 | `openapi.json` (자동), TS 타입 (자동) | TS 타입 수동 작성 차단 (dependency-cruiser) |
| **DB 스키마** | `db/migration/V*.sql` | Rust 타입 (sqlx 자동 검증) | 수동 ALTER TABLE 금지 |
| **인프라 설정** | Pulumi TypeScript 코드 | AWS 콘솔 (절대 수동 변경 금지) | Pulumi `refresh` drift 감지 → 알림 |
| **시크릿** | AWS Secrets Manager / Vault | `.env.example`은 placeholder만 | gitleaks |
| **도메인 용어** | `docs/glossary.md` | 모든 코드/UI/문서 사용 | grep CI 룰 |
| **도구 버전** | `rust-toolchain.toml` + `package.json#packageManager` | CI/Docker가 *읽기*만 | 직접 install 명령 차단 |
| **의존성 버전** | `Cargo.lock` + `pnpm-lock.yaml` | 보조 환경이 *그대로* 사용 | 수동 install 금지 |
| **시간** | DB는 UTC TIMESTAMPTZ | 응답/UI에서만 KST 변환 | 타입 시스템 (timezone-aware) |
| **좌표** | DB는 EPSG:4326 | 5179(연산), 3857(타일) | SRID 미지정 차단 |
| **사용자 권한** | Zitadel + DB `user_role` | 클라이언트 캐시 | JWT scope 검증 |
| **에러 코드** | `crates/api-types/error.rs` enum | OpenAPI spec, TS 타입 | enum exhaustive match |
| **컨벤션** | `docs/conventions/*.md` | 도구 설정 (biome.json, clippy.toml) | 도구가 자동 강제 |
| **결정 이력** | `docs/adr/NNNN-*.md` | (다른 곳 인용은 링크) | 새 결정은 코드 작성 *전* ADR 필수 |
| **메모리 (자동)** | `memory/*.md` (MEMORY.md 인덱스) | (없음 — 컨텍스트별 동적) | 직접 수정 OK, 인덱스만 |
| **SSS 헌법** | `docs/sss-charter.md` | (다른 곳 인용은 링크) | (헌법 자체) |
| **글로서리** | `docs/glossary.md` | (모든 코드/UI/문서) | grep CI |

---

## 2. 문서 SSOT (도메인 폴더 단위)

```
docs/
├── sss-charter.md          ← SSS 정의 SSOT
├── glossary.md             ← 도메인 용어 SSOT
├── ssot-matrix.md          ← 이 문서 (메타 SSOT)
│
├── adr/                    ← 모든 결정 이력 SSOT
├── conventions/            ← 코드 스타일 SSOT
├── data-sources/           ← 외부 API 카탈로그 SSOT
│
├── infrastructure/         ← 인프라/배포 SSOT
├── auth/                   ← 인증/권한 SSOT
├── data/                   ← DB/PostGIS/마이그레이션 SSOT
├── cache-messaging/        ← 캐시/메시징 SSOT
├── backend/                ← Rust 백엔드 SSOT
├── api/                    ← OpenAPI/REST SSOT
├── observability/          ← 관측성 SSOT
├── security/               ← 보안 SSOT
├── testing/                ← 테스트 전략 SSOT
├── frontend/               ← Next.js/UI SSOT
├── governance/             ← 거버넌스/문서 SSOT
├── compliance/             ← 컴플라이언스 SSOT
└── cost/                   ← 비용 추정 SSOT
```

각 폴더 = 한 도메인의 SSOT. 다른 폴더에서 같은 정보 작성 = SSOT 위반.

---

## 3. 코드 SSOT (모노레포 워크스페이스)

```
crates/
├── domain/                  ← 비즈니스 규칙 SSOT
├── shared-kernel/           ← 공유 값 객체 SSOT (Pnu, Money, Geometry 등)
├── api-types/               ← API 계약 + 에러 코드 SSOT
├── data-clients/            ← 외부 API HTTP 클라이언트 (각 API 1폴더)
└── db/                      ← Repository 구현 (도메인 trait 위임)

services/                    ← 실행 가능 (API/Worker/Pipeline)
apps/                        ← UI (Next.js)
packages/                    ← TS 라이브러리 (UI/api-client/map)
```

---

## 4. 위반 자동 차단 룰 (10개)

각 룰은 *어디서* 강제되는지 명시. 사람이 지키는 게 아님.

| # | 위반 | 차단 도구 | 단계 |
|---|------|---------|------|
| 1 | 수동 작성 OpenAPI | CI에서 cargo로 `openapi.json` 생성 후 git diff (변경 시 fail) | CI |
| 2 | 수동 작성 TS 타입 (백엔드 응답용) | dependency-cruiser — `packages/api-client/types.ts`는 자동 생성만 | CI |
| 3 | AWS 콘솔 수동 변경 | Pulumi `refresh` drift 감지 → CI 알림 | CI 정기 |
| 4 | DB 스키마 수동 변경 | `flyway info` / `sqlx migrate info` mismatch | pre-push + CI |
| 5 | 시크릿 git 커밋 | gitleaks pre-commit + CI | pre-commit + CI |
| 6 | 코드 스타일 위반 | rustfmt + Biome (lefthook) | pre-commit |
| 7 | 의존성 방향 위반 | dependency-cruiser (TS) + cargo-arch (Rust) | CI |
| 8 | 파일 ≤500 / 1500 위반 | 자체 file-size hook + CI | pre-commit + CI |
| 9 | 글로서리 외 도메인 용어 | CI grep 룰 (`Property`, `Land`, `Realtor` 등) | CI |
| 10 | TODO/HACK/XXX/`_TEMP` 코드 | clippy `todo` deny + Biome 자체 룰 | pre-commit + CI |

---

## 5. 새 정보 추가 시 (워크플로우)

새 종류의 정보가 생기면:

1. **이 문서(§ 1 표)에 추가** — SSOT 위치, 사본, 차단 룰
2. **차단 룰 부재 시 룰 추가** — lefthook / CI / linter
3. **ADR 작성** (큰 결정의 경우)
4. **PR로 검토 + 승인 후 머지**

→ "정보가 두 곳에 있는데 어디가 진짜?"라는 질문이 발생하기 *전에* 표에 박힘.

---

## 6. 자체 검증

분기별로 다음 5 질문 자체 점검:

1. □ 같은 정보가 두 곳에 있으면 즉시 어느 게 SSOT인지 답 가능?
2. □ DB와 도메인 코드가 충돌하면? → **컴파일 실패 (sqlx)**
3. □ Rust 응답과 TS 타입이 충돌하면? → **TS 컴파일 실패 (자동 생성)**
4. □ AWS 콘솔에 직접 만든 리소스가 있는가? → **0개여야 함 (Pulumi)**
5. □ 같은 도메인 용어를 다르게 부르는 곳이 있는가? → **0개 (glossary 자동 검증)**

→ 5/5 = SSOT 합격. 그 외는 즉시 차단 룰 추가.

---

## 7. 안티패턴 (피해야 할 SSOT 위반)

| 안티패턴 | 사례 | 해결 |
|---------|------|------|
| **거대 단일 SSOT 파일** | docs/schema.md 1349줄, docs/site-builder.md 1447줄 | 폴더로 분해 (`docs/schema/auth.md`, `docs/schema/parcel.md`...) |
| **TS 타입 수동 + Rust 변경 따라가기** | v2의 `ALLOWED_FOR_FRONTEND_TEMP` | OpenAPI 자동 생성 |
| **AWS 콘솔에서 *살짝* 수정** | "한 번만 빠르게" | Pulumi 코드만 |
| **두 곳에 같은 도메인 용어** | "매물" vs "물건", "Property" vs "Listing" | glossary 강제 |
| **README에 정보 vs 코드 주석에 정보** | 코드 변경 후 README 까먹음 | 코드가 SSOT, README는 *링크만* |
| **시크릿 .env에 + 1Password에 둘 다** | 동기화 실패 | AWS Secrets Manager / Vault만 |
