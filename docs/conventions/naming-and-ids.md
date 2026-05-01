# 네이밍 + ID 컨벤션

## 1. 케이스 규칙

| 영역 | 규칙 | 예시 |
|------|------|------|
| Rust 타입 | `PascalCase` | `Listing`, `Pnu` |
| Rust 함수/필드/변수 | `snake_case` | `find_by_id`, `area_m2` |
| Rust 상수 | `SCREAMING_SNAKE_CASE` | `MAX_LISTINGS` |
| Rust 모듈 | `snake_case` | `crates/domain/listing` |
| TS 타입/컴포넌트 | `PascalCase` | `Listing`, `MapView` |
| TS 함수/변수 | `camelCase` | `findById`, `areaM2` |
| TS 파일 | `kebab-case` | `listing-detail.tsx` |
| TS 폴더 | `kebab-case` | `industrial-complex/` |
| DB 테이블 | `snake_case` 단수 | `listing`, `parcel` |
| DB 컬럼 | `snake_case` | `created_at`, `owner_id` |
| API URL | `kebab-case` 복수 | `/v1/listings`, `/v1/industrial-complexes` |
| API 응답 필드 | `camelCase` | `createdAt`, `pricePerSqm` |
| 에러 코드 | `SCREAMING_SNAKE_CASE` | `LISTING_NOT_FOUND` |
| 환경 변수 | `SCREAMING_SNAKE_CASE` | `VWORLD_API_KEY` |
| Git 브랜치 | `kebab-case` + 타입 prefix | `feat/listing-search`, `fix/auth-token-leak` |

## 2. ID 전략 — ULID + 도메인 prefix

모든 ID = **ULID** (Crockford Base32, 시간순 정렬 가능, URL 안전) + 도메인 prefix.

| 도메인 | prefix | 예시 |
|--------|--------|------|
| User | `usr_` | `usr_01HXY...` |
| Listing | `lst_` | `lst_01HXY...` |
| Parcel | `prc_` | `prc_01HXY...` |
| Building | `bld_` | `bld_01HXY...` |
| IndustrialComplex | `ic_` | `ic_01HXY...` |
| Manufacturer | `mfr_` | `mfr_01HXY...` |
| RealTransaction | `rtx_` | `rtx_01HXY...` |
| CourtAuction | `cau_` | `cau_01HXY...` |
| Subscription | `sub_` | `sub_01HXY...` |
| Inquiry | `inq_` | `inq_01HXY...` |
| Bookmark | `bmk_` | `bmk_01HXY...` |
| AnalysisReport | `rpt_` | `rpt_01HXY...` |
| Notification | `ntf_` | `ntf_01HXY...` |
| AuditLog | `aud_` | `aud_01HXY...` |
| Law | `law_` | `law_01HXY...` |
| Regulation | `reg_` | `reg_01HXY...` |

PNU (필지 번호)는 한국 정부 표준 (19자리) — ID prefix 사용 안 함. 그러나 도메인 코드에서는 `Pnu` 값 객체로 캡슐화.

## 3. 도메인 용어

[../glossary.md](../glossary.md) SSOT. 글로서리 외 단어 사용 = CI 차단.

## 4. 좌표/면적/통화

| 필드 | 단위 | 예시 |
|------|------|------|
| 면적 | `_m2` (㎡) suffix | `area_m2`, `floor_area_m2` |
| 좌표 | EPSG SRID 명시 | `geometry(Polygon, 4326)` |
| 통화 | `_krw` suffix (KRW 고정) | `price_krw`, `monthly_rent_krw` |
| 시간 | UTC 저장 | `created_at TIMESTAMPTZ` |

UI 표시 시:
- 면적: ㎡ + 평 동시 (1평 = 3.305785㎡)
- 가격: 억/만원 (`850,000,000` → "8억 5,000만원")
- 시간: KST 변환

## 5. 시간 컨벤션

- DB 저장: `TIMESTAMPTZ` (UTC)
- API 응답: ISO 8601 with timezone (`2026-04-22T10:30:00+09:00`)
- UI 표시: KST 변환 + 한국식 (`2026년 4월 22일 오후 10시 30분`)

## 6. 자동 강제

- Rust: `clippy::wrong_self_convention`, `clippy::module_name_repetitions`
- TS: Biome `useNamingConvention`
- DB: sqlfluff (sub-project 2+)
- ID prefix: 자체 lint (sub-project 5+)
- 글로서리: CI grep 룰 (sub-project 5+)
