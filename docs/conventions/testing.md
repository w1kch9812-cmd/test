# 테스트 컨벤션

## 1. 분류

| 종류 | 도구 | 위치 | 목적 |
|------|------|------|------|
| 단위 | `cargo test`, Vitest | 같은 파일 (`#[cfg(test)] mod tests`) 또는 `__tests__/` | 함수/메서드 |
| 통합 | Testcontainers + sqlx::test | `tests/` (각 crate) | DB + 외부 API |
| 계약 | Pact (sub-project 5+) | `tests/contract/` | API 양쪽 약속 |
| 스냅샷 | insta (Rust), Vitest snapshot | 단위 테스트와 동거 | 응답/렌더 결과 |
| Property | proptest | 단위 테스트 | 불변 속성 |
| Mutation | cargo-mutants | CI 정기 | 테스트 품질 |
| E2E | Playwright | `tests/e2e/` | 전체 사용자 시나리오 |
| 부하 | k6 | `tests/load/` | 성능 회귀 |
| 카오스 | Chaos Mesh | `tests/chaos/` | 장애 회복 |
| 시각 회귀 | Lost Pixel (OSS) | Storybook + CI | UI 변경 |

## 2. 커버리지 임계값 (CI 차단)

| 영역 | 최소 커버리지 |
|------|------------|
| `crates/domain/*` | **90%** (도메인 핵심) |
| `crates/db` | 70% |
| 공짱 소유 비-Catalog 외부 어댑터 crate | 70% |
| `services/*` | 60% |
| `apps/*` 라우터/Server Action | 50% |
| `packages/ui-web` 컴포넌트 | 40% (시각 회귀로 보강) |

도구: `cargo-tarpaulin` (Rust), Vitest coverage (TS).

Catalog 소스 클라이언트, raw capture, 공용 공간 데이터 리더는 Platform Core 소유다.
공짱 테스트 커버리지 표는 그 crate를 다시 만들거나 관리 대상으로 해석하면 안 된다.

## 3. 네이밍 규칙

`<주체>_<can/cannot/returns/throws>_<조건>` — 한 줄이 검증 사실.

```rust
// Rust
#[test]
fn listing_can_be_published_when_owner_has_business_number() { ... }

#[test]
fn listing_cannot_be_published_when_status_is_draft() { ... }

#[test]
fn listing_publish_returns_error_when_already_sold() { ... }

#[test]
fn pnu_try_new_throws_when_input_is_not_19_digits() { ... }
```

```ts
// TS (Vitest)
test("listing can be published when owner has business number", () => { ... });
test("listing cannot be published when status is draft", () => { ... });
```

## 4. AAA 패턴

```rust
#[test]
fn ... () {
    // Arrange
    let owner = User::with_business_number("123-45-67890");
    let listing = Listing::draft(&owner);

    // Act
    let result = listing.submit_for_review();

    // Assert
    assert!(result.is_ok());
    assert_eq!(listing.status(), ListingStatus::Review);
}
```

## 5. mock 정책

| 종류 | 정책 |
|------|------|
| 외부 HTTP API | mockall + wiremock 또는 Testcontainers |
| DB | sqlx::test (실제 PG, 트랜잭션 rollback) |
| 시간 | mockable Clock (TimeProvider trait) |
| 랜덤 | seedable Rng |
| Zitadel | OIDC 모의 서버 (sub-project 3) |

→ "거의 항상 실제 DB"는 SSOT 원칙과 일치 (mock이 사본을 만들지 않게).

## 6. Rust 단위 테스트 예시

```rust
// crates/domain/core/listing/src/entity.rs
#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[fixture]
    fn business_user() -> User {
        User::with_business_number("123-45-67890")
    }

    #[rstest]
    fn listing_can_be_published_when_owner_has_business_number(business_user: User) {
        let listing = Listing::draft(&business_user);
        assert!(listing.can_be_published_by(&business_user));
    }
}
```

## 7. 통합 테스트 (sqlx::test)

```rust
// crates/db/tests/listing_repo.rs
#[sqlx::test]
async fn save_and_find_listing(pool: PgPool) {
    let repo = PgListingRepository::new(pool);
    let listing = Listing::draft_for_test();

    repo.save(&listing).await.expect("save");
    let found = repo.find(&listing.id()).await.expect("find").expect("exists");

    assert_eq!(found.id(), listing.id());
}
```

`#[sqlx::test]` = 자동 fresh DB + 마이그레이션 적용 + 트랜잭션 rollback.

## 8. E2E (Playwright)

```ts
// tests/e2e/listings.spec.ts
test("buyer can search and bookmark listings", async ({ page }) => {
  await page.goto("/listings");
  await page.fill("[data-testid=search-input]", "강남구 공장");
  await page.click("[data-testid=search-button]");

  await expect(page.locator("[data-testid=listing-card]")).toHaveCount(20);
  await page.click("[data-testid=listing-card]:first-child");
  await page.click("[data-testid=bookmark-button]");

  await expect(page.locator("[data-testid=bookmark-success]")).toBeVisible();
});
```

`data-testid` 사용 (CSS 클래스/텍스트로 선택 X — 변경 깨짐 방지).

## 9. 금지 패턴

- ❌ Sleep 기반 동기화 (`tokio::time::sleep` 단순 대기) — 이벤트 기반으로
- ❌ 환경 의존 (테스트 머신 시간, 로케일 등) — 명시적 fixture로
- ❌ 테스트 간 공유 state (각 테스트 격리)
- ❌ "skip" 또는 "ignore" 누적 — 깨진 테스트는 즉시 수정 또는 삭제
- ❌ 실제 외부 API 호출 (Zitadel, V-World, Naver Maps 등) — 항상 mock 또는 Testcontainers

## 10. 자동 강제

- pre-push: `cargo test --workspace` + `pnpm vitest run`
- CI: 커버리지 임계값 검증 (`cargo-tarpaulin --fail-under 90`)
- Pact: PR마다 계약 호환성 검증
- Mutation: 주간 cron (`cargo-mutants`)
