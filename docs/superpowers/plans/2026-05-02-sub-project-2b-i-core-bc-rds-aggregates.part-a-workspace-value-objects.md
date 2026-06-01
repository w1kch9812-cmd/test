# Sub-project 2b-i - Part A: Workspace Restructure And Shared-Kernel Value Objects

Parent index: [Sub-project 2b-i Core BC RDS Aggregates](./2026-05-02-sub-project-2b-i-core-bc-rds-aggregates.md).
# Sub-project 2b-i: Core BC RDS Aggregates (User, Listing, ListingPhoto) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development. Steps use `- [ ]` syntax.
>
> **CRITICAL:** Before each task, re-read [memory/feedback_subproject_2a_lessons.md](../../../memory/feedback_subproject_2a_lessons.md). 4 패턴 (spec verbatim, 값 객체 표준, 마이그 파일명, tarpaulin strict)을 dispatch에 자동 반영.

**Goal:** Spec § 8.1 워크스페이스 구조로 재정렬 + Core BC RDS *동적* Aggregate 3개 (User, Listing, ListingPhoto) 구현. Repository trait (port)만 정의 — 구현체는 sub-project 5.

**Architecture:** 3 phase, 13 task.
- **Phase A (Task 1):** 워크스페이스 재구조 (shared-kernel 이동 + 3 BC crate scaffolding)
- **Phase B (Tasks 2-7):** 추가 값 객체 6개 (ListingType, TransactionType, ListingStatus, ContactVisibility, ListingTitle, Description) + 기존 shared-kernel에 합류
- **Phase C (Tasks 8-13):** 3 Aggregate 구현 + Repository trait + 최종 검증

**Tech Stack:** Rust 1.85, async-trait, chrono, geo-types, ulid (이미 workspace dep). 신규: `async-trait` (Repository trait용).

**Patterns from sub-project 2a (강제):**
1. Spec § X line range *직접 인용*. Plan 본문 paraphrase 신뢰 X
2. 값 객체 표준 패턴 (`#[derive]` 7-trait, `#[serde(transparent)]`, `try_new`/`Display`/`FromStr`, `# Errors`/`# Panics` rustdoc)
3. 마이그 없음 (Plan 2b-i는 Rust only — DB 변경 없음)
4. tarpaulin = 최후 진실. CI green 게이트

**Pre-flight (Task 1 시작 전):**
- [ ] Plan 2a + 2a-fixup 완료 + CI green (`72a4036` or later)
- [ ] `git status` clean
- [ ] `crates/shared-kernel/` 11 모듈 (admin_division ~ time)

---

## File Structure (목표 — Phase A 완료 시점)

### 워크스페이스 (재구조)

```
crates/
├── domain/
│   └── core/
│       ├── shared-kernel/        (← crates/shared-kernel/ 에서 이동)
│       ├── user/                 신규
│       ├── listing/              신규
│       └── listing-photo/        신규
```

### 신규 BC crate 구조 (각각 동일 패턴)

```
crates/domain/core/<bc>/
├── Cargo.toml                    workspace deps + shared-kernel + async-trait
├── src/
│   ├── lib.rs                    pub mod entity; pub mod errors; pub mod repository;
│   ├── entity.rs                 Aggregate struct + 도메인 메서드
│   ├── errors.rs                 BcXxxError enum (thiserror)
│   └── repository.rs             #[async_trait] trait XxxRepository
└── README.md                     ≤30줄, 해요체
```

---

## Task 1: 워크스페이스 재구조 — shared-kernel 이동 + 3 BC crate scaffold

**스펙 참조:** spec § 8.1 (lines 756-805)

**Files:**
- Move: `crates/shared-kernel/` → `crates/domain/core/shared-kernel/` (git mv 사용)
- Create: `crates/domain/core/user/{Cargo.toml, src/lib.rs, README.md}` (3 파일)
- Create: `crates/domain/core/listing/{...}` (3 파일)
- Create: `crates/domain/core/listing-photo/{...}` (3 파일)
- Modify: 루트 `Cargo.toml` workspace.members 갱신 (shared-kernel 경로 + 3 신규)
- Modify: workspace.dependencies에 `async-trait = "0.1"` 추가

**검증:** `cargo check --workspace` + `cargo test --workspace --no-run` 모두 통과 — 코드 변경 없으니 빈 BC crate 3개 + shared-kernel 11 모듈이 컴파일만 되면 됨.

- [ ] Step 1: `git mv crates/shared-kernel crates/domain/core/shared-kernel` (디렉토리 통째 이동)
- [ ] Step 2: 3 신규 BC crate 디렉토리 생성 + 빈 lib.rs + 최소 Cargo.toml + README
- [ ] Step 3: 루트 `Cargo.toml` workspace.members 4개 경로로 갱신
- [ ] Step 4: workspace.dependencies에 `async-trait = "0.1"` 추가
- [ ] Step 5: `cargo check --workspace` 확인 (로컬 불가 시 CI 위임)
- [ ] Step 6: Commit + push

```bash
git commit -m "refactor(workspace): restructure crates per spec § 8.1 — move shared-kernel + scaffold 3 Core BC crates"
```

---

## Tasks 2-7: shared-kernel 추가 값 객체 6개

각 task = 1 값 객체. 패턴은 Tasks 12-25와 동일. 스펙 참조는 spec § 5.1 (listing 컬럼 정의의 enum 값 들).

### Task 2: ListingType enum

**스펙:** spec § 5.1 line 185-186 — `factory`, `warehouse`, `office`, `knowledge_industry_center`, `industrial_land`, `logistics_center` (6값)

**File:** `crates/domain/core/shared-kernel/src/listing_type.rs`

**Pattern:** unit-like enum + `#[repr(...)]` 불필요. `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]` + `#[serde(rename_all = "snake_case")]`. `as_str()` 메서드 + `FromStr`.

- [ ] Tests (≥8): 각 6값 valid 파싱, snake_case roundtrip, 미지원 값 거부, Display
- [ ] Impl (stubs first per TDD)
- [ ] CI green

```bash
git commit -m "feat(shared-kernel): ListingType enum (6 industrial property types)"
```

### Task 3: TransactionType enum

**스펙:** spec § 5.1 line 187-188 — `sale`, `monthly_rent`, `jeonse` (3값). 거래 유형별 deposit/monthly_rent NULL 규칙 *문서화* (실제 검증은 SQL CHECK V003_01).

**File:** `crates/domain/core/shared-kernel/src/transaction_type.rs`

**핵심 메서드:**
```rust
pub fn requires_deposit(self) -> bool   // monthly_rent + jeonse → true
pub fn requires_monthly_rent(self) -> bool  // monthly_rent → true
```

- [ ] Tests (≥8): 3값 + 두 도우미 메서드 + Display + FromStr
- [ ] CI green

```bash
git commit -m "feat(shared-kernel): TransactionType enum (sale/monthly_rent/jeonse) + deposit/rent invariant helpers"
```

### Task 4: ListingStatus enum + 상태 머신

**스펙:** spec § 5.1 line 195-196 — `draft`, `pending_review`, `active`, `sold`, `expired`, `rejected` (6값). spec § 8.3 lines 873-895의 상태 전이 규칙.

**File:** `crates/domain/core/shared-kernel/src/listing_status.rs`

**핵심:** 상태 전이 *허용 그래프*를 메서드로 제공:
```rust
pub fn can_transition_to(self, target: Self) -> bool {
    use ListingStatus::*;
    matches!((self, target),
        (Draft, PendingReview)
        | (PendingReview, Active)
        | (PendingReview, Rejected)
        | (Active, Sold)
        | (Active, Expired)
        | (Rejected, Draft)  // 사용자 수정 후 재제출
    )
}
```

- [ ] Tests (≥10): 6값 + 7개 합법 전이 + 5개 위반 전이 + Display
- [ ] CI green

```bash
git commit -m "feat(shared-kernel): ListingStatus enum + state machine (can_transition_to)"
```

### Task 5: ContactVisibility enum

**스펙:** spec § 5.1 line 197-198 — `public`, `login_required`, `verified_only` (3값).

**File:** `crates/domain/core/shared-kernel/src/contact_visibility.rs`

- [ ] Tests (≥6): 3값 + Display + FromStr
- [ ] CI green

```bash
git commit -m "feat(shared-kernel): ContactVisibility enum (public/login_required/verified_only)"
```

### Task 6: ListingTitle 값 객체

**스펙:** spec § 5.1 line 193 — `title varchar(200) not null`. 빈 문자열 거부, ≤200자.

**File:** `crates/domain/core/shared-kernel/src/listing_title.rs`

**Pattern:** 동일 — RoadAddress와 같은 String wrapper. trim 후 비어있지 않음 + ≤200자.

- [ ] Tests (≥8): 정상, trim, 빈 거부, 200자 OK, 201자 거부, 한글 허용
- [ ] CI green

```bash
git commit -m "feat(shared-kernel): ListingTitle — non-empty bounded string (≤200 chars)"
```

### Task 7: Description 값 객체

**스펙:** spec § 5.1 line 194 — `description text not null default ''`. 빈 *허용* (default ''), 길이 상한 없음 (text type)이지만 application-level cap 5000자 권고.

**File:** `crates/domain/core/shared-kernel/src/description.rs`

**Pattern:** 빈 허용 (Option 아니라 빈 String wrapper). 길이 ≤5000.

- [ ] Tests (≥8): 정상, 빈 OK, 5000자 OK, 5001자 거부, 한글, multi-line
- [ ] CI green

```bash
git commit -m "feat(shared-kernel): Description — empty-allowed bounded text (≤5000 chars)"
```

---
