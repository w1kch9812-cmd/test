# 주석 컨벤션

## 1. 원칙: Why over What

- ❌ **What** (이 코드가 *뭘* 하는지) — 좋은 네이밍이 대신함
- ✅ **Why** (왜 이렇게 했는지) — 좋은 코드는 *왜*를 못 말함

```rust
// ❌ What
// listing의 status를 Active로 변경
listing.status = ListingStatus::Active;

// ✅ Why
// V-World API 검증 후 자동 Active 전환 (운영자 검수 미요구 — ADR-0010)
listing.status = ListingStatus::Active;

// 또는 더 좋게: 이름이 말하게
listing.activate_after_external_verification();
```

## 2. TODO 컨벤션

```
// TODO(YYYY-Q?, #issue-id): 짧은 설명
```

- 만료일(분기) + 이슈 번호 **필수**
- 만료 후 자동 PR 생성 (Phase 2+ 자체 도구)

```rust
// TODO(2026-Q3, #142): pgvector HNSW 인덱스 도입 후 임베딩 fallback 제거
fn fallback_embedding_search(...) -> Vec<Listing> { ... }
```

## 3. 금지 키워드

| ❌ | ✅ 대신 |
|---|--------|
| `HACK` | TODO + 명확한 이유 |
| `XXX` | TODO + 명확한 이유 |
| `FIXME` | TODO 또는 즉시 수정 |
| `HACK_ALLOWED_FOR_FRONTEND_TEMP` | (v2 안티패턴) — 절대 금지 |

→ Rust clippy + Biome 자체 룰에서 keyword 검출 시 deny.

## 4. 도메인 결정 근거

```rust
// ADR-0007: moka L1 + Valkey L2 — Redis SSPL 라이선스 회피
let cache = MokaCache::new(...);
```

```ts
// see: docs/adr/0010-scope-information-platform-option-a.md
// 옵션 A — LLM 생성 텍스트 사용자 노출 금지
const description = listing.descriptionFromUser; // NOT generated
```

## 5. 외부 참조

```rust
// V-World layer ID: https://www.vworld.kr/dev/v4dv_2ddataguide2_s002.do
const LAYER_ZONING: &str = "LT_C_UQ111";
```

링크는 *영구 식별자* (commit SHA, archive.org) 또는 공식 문서 표준 URL.

## 6. unsafe 또는 비표준 사용 시

`unsafe` 자체는 `forbid` (`workspace.lints`). 만약 매크로 등으로 우회 필요 시:

```rust
// SAFETY: <왜 안전한지>
// (이 코드는 [ADR-XXXX]에서 승인됨)
unsafe { ... }
```

→ Rust 표준 SAFETY 코멘트 컨벤션 따름. ADR 링크 의무.

## 7. 라이센스/저작권 헤더

- 본 프로젝트: 사내 비공개라 *모든 파일에* 저작권 헤더 필요 X
- 외부 OSS 코드 차용 시: 그 라이선스 고지 의무 (`reference/`에서 차용 후 직접 작성하면 우리 코드)

## 8. 모듈 독스트링 (Rust)

```rust
//! crates/domain/core/listing — 매물 도메인
//!
//! 매물 등록·검수·상태 머신·공개/만료 규칙.
//! → docs/glossary.md 의 `Listing` 정의 참조.

pub struct Listing { ... }
```

`//!` = 모듈 도큐멘트. `///` = 항목 도큐멘트.

## 9. TS 컴포넌트 docstring

```tsx
/**
 * 매물 상세 카드. 매물 클릭 시 모달로 띄우거나 페이지 전환.
 *
 * @see docs/frontend/canvas-markers.md — Canvas 마커와 연동
 */
export function ListingCard({ listing }: Props) { ... }
```

## 10. 자동 강제

- Rust: `clippy::missing_docs_in_private_items` (warn), `missing_docs` (warn for public)
- Rust: clippy lint `clippy::todo`, `clippy::unimplemented` deny
- Biome: TODO 형식 검출 (Phase 2+ 자체 룰)
- CI: 만료된 TODO grep + 알림 (Phase 2+)

## 11. 좋은 주석 vs 나쁜 주석 비교

```rust
// ❌ 나쁨
let i = 0; // 카운터

// ❌ 나쁨 (코드 반복)
// price를 2배로 곱한다
let doubled = price * 2;

// ✅ 좋음 (Why)
// 부가세 10% 포함 가격 — NTS API와 비교 시 사용 (sub-project 결제 단계)
let price_with_vat = price.with_vat();

// ✅ 좋음 (제약)
// PostGIS 5179는 미터 단위, 4326은 degree — st_dwithin 거리는 *연산 좌표계* 기준
let geom_5179 = parcel.geom.transform_to(5179);
```
