# Sub-project 2a-fixup: Spec Defects (5 tasks) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development. Steps use `- [ ]` syntax.

**Goal:** Plan 2a 진행 중 발견된 spec/value-object 결함 5건을 *Plan 2b 시작 전* 정리. 깨진 토대 위에 Aggregate를 쌓지 않기 위함.

**Architecture:** 5 task. 처음 3개는 V003 마이그레이션 (DB 레벨 invariant), 마지막 2개는 shared-kernel 값 객체 보강. 각 task = TDD + CI green + push.

**Defer policy:** 8개 follow-up 중 5개만 즉시 수정. 나머지 3개 (NTS 체크섬 외부 검증, BusinessNumber D₃D₄, KsicCode A-U)는 *외부 정책/사람 검증 의존*이라 production 배포 전까지만 처리되면 됨.

---

## Task A: V003_01 — listing transaction_type cross-field CHECK

**Files:**
- Create: `migrations/30001_listing_transaction_type_check.sql`
- Modify: `tests/migrations/test_v001_full.sh` — assert constraint exists

**Spec defect:** spec § 5.1 `listing` 테이블에 `transaction_type` × `deposit_krw`/`monthly_rent_krw` cross-field CHECK 누락. 결과:
- `transaction_type='sale'` + `deposit_krw=100` (NULL 이어야) — 통과
- `transaction_type='monthly_rent'` + `deposit_krw=NULL` — 통과 (전세금 없는 월세)
- `transaction_type='jeonse'` + `monthly_rent_krw=100` (NULL 이어야) — 통과

올바른 invariant:

| transaction_type | deposit_krw | monthly_rent_krw |
|---|---|---|
| `sale` | NULL | NULL |
| `monthly_rent` | NOT NULL | NOT NULL |
| `jeonse` | NOT NULL | NULL |

- [ ] Step 1: `30001_listing_transaction_type_check.sql` 작성

```sql
-- V003_01: listing transaction_type × deposit/monthly_rent cross-field CHECK
-- spec § 5.1 누락 invariant 보강 (sub-project 2a-fixup)

alter table listing
    add constraint listing_transaction_fields_chk
    check (
        (transaction_type = 'sale'
         and deposit_krw is null
         and monthly_rent_krw is null)
        or
        (transaction_type = 'monthly_rent'
         and deposit_krw is not null
         and monthly_rent_krw is not null)
        or
        (transaction_type = 'jeonse'
         and deposit_krw is not null
         and monthly_rent_krw is null)
    );
```

- [ ] Step 2: `test_v001_full.sh`에 constraint 존재 검증 추가:

```bash
# Listing transaction_type cross-field CHECK exists (V003_01)
if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_constraint where conrelid='listing'::regclass and conname='listing_transaction_fields_chk';" | grep -q '^1$'; then
  echo "FAIL: listing_transaction_fields_chk missing" >&2; exit 1
fi
```

- [ ] Step 3: spec § 5.1 SQL 본문에 CHECK 추가 (spec ↔ migration 정렬)

- [ ] Step 4: Commit + push + CI green

```bash
git add migrations/30001_*.sql tests/migrations/test_v001_full.sh docs/superpowers/specs/
git commit -m "fix(db): V003_01 — listing transaction_type cross-field CHECK (spec § 5.1 gap)"
git push origin main
```

---

## Task B: V003_02 — BVQ + LRQ optimistic locking version

**Files:**
- Create: `migrations/30002_queue_optimistic_locking.sql`
- Modify: `tests/migrations/test_v001_full.sh`
- Modify: `docs/superpowers/specs/2026-05-02-sub-project-2-db-core-domain-design.md` § 5.5

**Spec defect:** `business_verification_queue` (BVQ)와 `listing_review_queue` (LRQ)는 어드민이 동시에 편집하는 워크플로우 테이블인데 `version bigint` 누락. 두 어드민이 같은 행을 동시 검토하면 lost update.

`pipeline_schedule`은 `version bigint` 있음. BVQ/LRQ도 같은 패턴 적용.

- [ ] Step 1: `30002_queue_optimistic_locking.sql` 작성

```sql
-- V003_02: BVQ + LRQ optimistic locking — concurrent admin edit lost update 방어

alter table business_verification_queue
    add column version bigint not null default 1;

alter table listing_review_queue
    add column version bigint not null default 1;
```

- [ ] Step 2: test 보강 — 두 테이블 모두 `version` 컬럼 존재 + default 1 검증

- [ ] Step 3: spec § 5.5 BVQ/LRQ에 `version bigint not null default 1` 라인 추가

- [ ] Step 4: Commit + push + CI green

---

## Task C: V003_03 — featured_content time-bound CHECK

**Files:**
- Create: `migrations/30003_featured_content_time_bound.sql`
- Modify: `tests/migrations/test_v001_full.sh`
- Modify: spec § 5.5

**Spec defect:** `featured_content.starts_at`/`ends_at` 둘 다 NOT NULL이지만 `ends_at > starts_at` invariant 누락. 광고 시작이 종료보다 늦으면 *영원히 비활성*.

- [ ] Step 1: `30003_featured_content_time_bound.sql` 작성

```sql
-- V003_03: featured_content ends_at > starts_at invariant

alter table featured_content
    add constraint featured_content_time_bound_chk
    check (ends_at > starts_at);
```

- [ ] Step 2: test 보강 — constraint 존재 검증

- [ ] Step 3: spec § 5.5 SQL 본문에 CHECK 추가

- [ ] Step 4: Commit + push + CI green

---

## Task D: BusinessNumber `000xxxxxxx` 예약 prefix 거부

**Files:**
- Modify: `crates/shared-kernel/src/business_number.rs`

**Spec defect:** 국세청 사업자번호 할당 규칙상 첫 3자리는 *세무서 코드*로 101+ 범위. `000xxxxxxx`/`00xxxxxxxx`/`0xxxxxxxxx` 등 `0`으로 시작하는 입력은 구조적으로 무효. 현재 `BusinessNumber::try_new`는 체크섬만 보므로 `0000000000` (체크섬 0)이 통과.

- [ ] Step 1: 실패 테스트 추가

```rust
#[test]
fn rejects_all_zeros() {
    let err = BusinessNumber::try_new("0000000000").unwrap_err();
    assert!(matches!(err, BusinessNumberError::ReservedPrefix));
}

#[test]
fn rejects_zero_prefix_001() {
    let err = BusinessNumber::try_new("0011234567").unwrap_err();
    assert!(matches!(err, BusinessNumberError::ReservedPrefix));
}
```

- [ ] Step 2: `BusinessNumberError`에 `ReservedPrefix` variant 추가:

```rust
/// 첫 3자리가 `000` (예약/미할당). 국세청 세무서 코드는 `101+`.
#[error("business number reserved prefix (first 3 digits must be ≥ 101)")]
ReservedPrefix,
```

- [ ] Step 3: `try_new`에 prefix 검증 추가 (체크섬 *전*에):

```rust
// Reserved prefix check (NTS allocation: tax office codes start at 101)
let prefix: u32 = cleaned[..3].parse().unwrap_or(0);
if prefix < 101 {
    return Err(BusinessNumberError::ReservedPrefix);
}
```

> **주의:** `parse().unwrap_or(0)` — workspace lints `unwrap_used = "deny"`. 함수에 `#[allow(clippy::unwrap_used)]` + `# Panics` 추가하거나, 체크섬 직전이라 이미 `cleaned`은 `is_ascii_digit`로 검증된 10자리 → infallible. 더 깔끔하게:

```rust
let first_three: u32 = cleaned[..3]
    .chars()
    .fold(0u32, |acc, c| acc * 10 + u32::from(c as u8 - b'0'));
if first_three < 101 {
    return Err(BusinessNumberError::ReservedPrefix);
}
```

- [ ] Step 4: Commit + push + CI green

---

## Task E: PhoneKr `82xxx` ambiguity 명확화

**Files:**
- Modify: `crates/shared-kernel/src/phone_kr.rs`

**Spec defect:** 현재 정규화 로직:
```rust
if let Some(rest) = digits.strip_prefix("82") {
    digits = format!("0{rest}");
}
```

이게 다음 케이스에서 silent rewrite:
- `8212345678` (10자리) → `012345678` (9자리, 서울 landline) — 모호
- `82012345678` (11자리) → `0012345678` — `00` 시작은 한국 다이얼링 패턴 아님

올바른 동작: `82` prefix는 *국가 코드*로만 인식해야 함. 입력이 `+82-` 또는 명시적 국제 형식일 때만 strip.

- [ ] Step 1: 실패 테스트 추가

```rust
#[test]
fn accepts_explicit_plus_82() {
    let p = PhoneKr::try_new("+82-10-1234-5678").expect("explicit +82");
    assert_eq!(p.as_str(), "01012345678");
}

#[test]
fn rejects_ambiguous_82_without_plus() {
    // "8212345678" — without +, treat as raw digits with leading 8 (invalid Korean)
    let err = PhoneKr::try_new("8212345678").unwrap_err();
    assert!(matches!(err, PhoneKrError::MustStartWithZero { .. }));
}

#[test]
fn rejects_82_followed_by_zero() {
    // "+82-0-10-1234-5678" — explicit +82 then leading 0 in domestic — should not silently produce "00..."
    let err = PhoneKr::try_new("+82-0-10-1234-5678").unwrap_err();
    assert!(matches!(err, PhoneKrError::InvalidLength { .. } | PhoneKrError::MustStartWithZero { .. }));
}
```

- [ ] Step 2: 로직 변경 — `+`/`82` 명시적 prefix만 strip

```rust
// 명시적 +82 prefix만 처리 (raw "82..." 시작은 ambiguous → 그대로)
let normalized = s.trim();
let has_plus_82 = normalized.starts_with("+82");
let mut digits: String = normalized.chars().filter(char::is_ascii_digit).collect();

if has_plus_82 {
    if let Some(rest) = digits.strip_prefix("82") {
        // +82-0-... 처럼 strip 후 leading 0이 또 있으면 사용자 실수 → 그대로 검증 단계로
        digits = format!("0{rest}");
    }
}
// else: raw digits — strip_prefix("82") 안 함

if !(9..=11).contains(&digits.len()) { ... }
if !digits.starts_with('0') { ... }
```

- [ ] Step 3: 기존 통과 테스트 검토 — `parse_with_82_prefix_no_plus`는 *기대 동작이 바뀜*. 새 동작에서 `82-10-1234-5678` (no +)는 거부. 기존 테스트 삭제 또는 *거부 테스트로 변경*.

- [ ] Step 4: Commit + push + CI green

---

## 완료 기준

5개 task 모두 통과 + CI green + spec § 5.1, § 5.5 갱신 = Plan 2a 진짜 종료.

이후 Plan 2b 시작.
