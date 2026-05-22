## Task 8: E2E + a11y + mobile responsive + bundle

**Files:**
- Create: `apps/web/tests/e2e/listings.spec.ts`
- Modify: `apps/web/playwright.config.ts` (필요시 viewport)

- [ ] **Step 8.1: e2e listings.spec.ts**

`apps/web/tests/e2e/listings.spec.ts`:

```typescript
import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

const ZITADEL_REAL = process.env.ZITADEL_E2E_REAL === "true";

test.describe("listings search", () => {
  test("/(authenticated)/listings 미인증 → /login redirect", async ({ page }) => {
    await page.goto("/listings");
    await page.waitForURL(/\/login/, { timeout: 10000 });
    expect(page.url()).toContain("returnTo=%2Flistings");
  });

  test("a11y on /listings (인증 우회 — Zitadel 없을 때 skip)", async ({ page }) => {
    test.skip(!ZITADEL_REAL, "real Zitadel container required");
    // 로그인 흐름 (auth.spec.ts 의 login flow 와 같음, 단순화)
    await page.goto("/login");
    await page.click('button[type="submit"]');
    await page.waitForURL(/localhost:8443/);
    await page.fill('input[name="loginName"]', "admin@zitadel.localhost");
    await page.click('button[type="submit"]');
    await page.fill('input[name="password"]', "Admin123!");
    await page.click('button[type="submit"]');
    await page.waitForURL(/\/profile|\/listings/, { timeout: 30000 });
    await page.goto("/listings");

    const accessibility = await new AxeBuilder({ page }).analyze();
    expect(accessibility.violations).toEqual([]);
  });

  test("필터 변경 → URL query 동기화", async ({ page }) => {
    test.skip(!ZITADEL_REAL, "real Zitadel container required");
    await page.goto("/listings");
    // 종류 chip click
    await page.getByRole("button", { name: "공장", exact: true }).click();
    await expect(page).toHaveURL(/types=factory/);
  });
});
```

- [ ] **Step 8.2: 로컬 e2e**

```bash
docker compose -f infra/zitadel/docker-compose.yml up -d
pnpm --filter=@gongzzang/web test:e2e
```

Expected: 1 PASS (미인증 redirect), 2 skip (ZITADEL_E2E_REAL 미설정).

- [ ] **Step 8.3: bundle 검증**

```bash
pnpm --filter=@gongzzang/web test:bundle
```

Expected: PASS.

- [ ] **Step 8.4: Commit**

```bash
git add apps/web/tests/e2e/listings.spec.ts
git commit -m "test(6ii-T8): listings e2e + a11y (Zitadel-dep test 는 ZITADEL_E2E_REAL flag)

- 미인증 → /login redirect (CI 자동 실행)
- a11y axe 검증 (Zitadel 의존 — SP6-iam-infra ephemeral env 에서 ZITADEL_E2E_REAL=true)
- 필터 chip 클릭 → URL query 동기화 (SP6-iam-infra 에서 검증)"
```

---

## Task 9: docs + ADR

**Files:**
- Create: `docs/frontend/listings-search.md`
- Create: `docs/adr/0006-listing-search-naver-maps.md`

- [ ] **Step 9.1: ADR 0006**

`docs/adr/0006-listing-search-naver-maps.md`:

```markdown
# ADR-0006: Listing 검색 화면의 지도 vendor — Naver Maps

| | |
|---|---|
| 작성일 | 2026-05-05 |
| 상태 | Accepted |
| 결정자 | 사용자 |
| 컨텍스트 | SP6-ii (매물 검색 화면) — 지도 SDK 선택 |

## 결정

**Naver Maps JavaScript SDK** 를 SP6-ii 의 지도 vendor 로 채택.

## 대안 비교

| 기준 | Naver Maps | 카카오맵 | Google Maps |
|---|---|---|---|
| 한국 산업단지 정확도 | ◎ | ◎ | △ (해외 base) |
| 무료 quota (dev) | 10만/월 | 30만/월 | 28000/월 |
| 부동산 표준 | ◎ (네이버 부동산) | ○ | △ |
| 공시지가 / 산업단지 layer | 별도 | 별도 | X |
| API key 발급 | NCP 가입 필요 | 카카오 Dev 가입 | Google Cloud |
| 한국어 UI / docs | ◎ | ◎ | ○ |

## 결정 근거

1. 네이버 부동산 = 한국 부동산 표준 — 사용자 친숙도
2. Naver Maps SDK 의 산업단지 표시 정확도 (KSURE / GIS layer 기반)
3. 향후 V-World / 공시지가 layer 통합 시 Naver geo coding 호환성 (PNU 매핑)

## 결과

- `apps/web/lib/naver-maps.ts` lazy SDK loader
- `NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID` env 추가 (zod validated)
- 6 매물 종류 unique pin color
- 클러스터링 (`submodules=clustering`) 사용

## 미래 결정 자리

- Naver Maps 무료 quota 초과 시 카카오맵 fallback (SP7-i 의 quota alert + SP6-data-sync 가 batch 호출 분리)
- 해외 매물 추가 시 Google Maps multi-vendor 검토
```

- [ ] **Step 9.2: docs/frontend/listings-search.md**

`docs/frontend/listings-search.md`:

```markdown
# Listings 검색 화면 — 운영 가이드

> SP6-ii. 디버깅 / 데이터 source / Naver Maps quota / 자주 발생하는 이슈.

## 1. 화면 흐름

```
/login (SP6-i) → /listings (SP6-ii)
                  ↓
                  proxy.ts auth gate (sid → access_token Bearer)
                  ↓
                  GET /api/proxy/listings?bounds=&types=&page=
                  ↓
                  backend GET /listings → ListingRepository::find_card_summaries_in_bbox
                  ↓
                  PostGIS ST_Within(geom_point, ST_MakeEnvelope) + filter + page
```

## 2. 환경 변수

```
NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID=<NCP Maps Client ID>
```

NCP 가입 후 Maps 등록. Free tier: 10만 호출/월. 초과 시 SP7-i 의 alert.

## 3. 자주 발생하는 이슈

| 증상 | 원인 후보 | 확인 |
|---|---|---|
| 지도 안 뜸 | NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID 미설정 | DevTools Console 의 `Naver Maps SDK failed` |
| 매물 0건 | DB 에 매물 없음 / 모든 매물 status != active | psql `SELECT count(*) FROM listing WHERE status='active'` |
| 필터 무시 됨 | URL query 와 store 동기화 깨짐 | DevTools Network 의 `?types=` 확인 |
| 무한 스크롤 안 됨 | IntersectionObserver sentinel 가 viewport 밖 | DevTools Elements 의 sentinel div 확인 |

## 4. 데이터 source

- DB `listing` 테이블 (V001) — `status='active'` 만 표시
- 외부 API (V-World / data.go.kr) sync 는 SP6-data-sync 가 책임
- 사진 (`thumbnail_url`) 은 SP6-iii 의 listing-photo 테이블 join 으로 채움

## 5. 주요 컴포넌트

| File | 역할 |
|---|---|
| `app/(authenticated)/listings/page.tsx` | 통합 layout (3-column grid) |
| `components/listings/listing-map.tsx` | Naver Maps + 핀 |
| `components/listings/listing-card-list.tsx` | 카드 + 무한 스크롤 |
| `lib/listings/use-listings-query.ts` | 단일 useInfiniteQuery hook |
| `stores/listings.ts` | Zustand: bounds + filters + selectedListingId |

## 6. 미래 sub-project 자리

- SP6-iii: 매물 상세 (`/listings/:id`) + 즐겨찾기 toggle + 사진 갤러리
- SP6-iv: 매물 등록 (broker)
- SP6-data-sync: V-World / data.go.kr / 공공 API → DB 자동 sync
- SP6-search-region: 지역 검색 (Naver/카카오 주소 검색 API 통합)

## 7. Spec / Plan / ADR

- Spec: `docs/superpowers/specs/2026-05-05-sub-project-6-ii-listing-search-design.md`
- Plan: `docs/superpowers/plans/2026-05-05-sub-project-6-ii-listing-search.md`
- ADR-0006: `docs/adr/0006-listing-search-naver-maps.md`
```

- [ ] **Step 9.3: markdownlint + commit**

```bash
pnpm markdownlint-cli2 docs/frontend/listings-search.md docs/adr/0006-listing-search-naver-maps.md
git add docs/frontend/listings-search.md docs/adr/0006-listing-search-naver-maps.md
git commit -m "docs(6ii-T9): listings-search.md 운영 가이드 + ADR-0006 (Naver Maps 결정)

- frontend/listings-search.md: 디버깅 / 환경 변수 / 자주 발생 이슈 / 데이터 source / 미래 SP 자리
- adr/0006: Naver vs 카카오 vs Google 비교 + 한국 부동산 표준 + 무료 quota 결정 근거"
```

---

## 최종 검증 (T8 완료 후)

- [ ] **Step F.1: Push + 4 CI workflow 그린 확인**

```bash
git push origin main
gh run list --branch main --limit 5 --json status,conclusion,name
```

Expected: 4/4 success.

- [ ] **Step F.2: 사용자 manual 검증 (DB 의 매물 등록 + 화면 확인)**

```bash
# 1. Zitadel + Redis dev container
docker compose -f infra/zitadel/docker-compose.yml up -d

# 2. backend 시작
cargo run -p api

# 3. 가짜 매물 1개 SQL 직접 insert (또는 psql 으로)
psql $DATABASE_URL -c "
INSERT INTO listing (id, owner_id, parcel_pnu, listing_type, transaction_type,
                     price_krw, area_m2, title, status, geom_point, contact_visibility)
VALUES (
  'lst_test01HXY...',
  '<existing user.id>',
  '4111017200103580000',
  'factory', 'sale', 8000000000, 3960.0,
  '평택 첨단산업단지 공장',
  'active',
  ST_SetSRID(ST_MakePoint(127.0876, 37.0779), 4326),
  'login_required'
);
"

# 4. frontend dev
pnpm --filter=@gongzzang/web dev
```

브라우저 → `/login` → admin → `/listings` → 평택 매물 핀 + 카드 표시 확인.

- [ ] **Step F.3: SP6-ii 완료 보고 + 다음 sub-project 의향**

다음 후보:
- SP6-iii: 매물 상세 + 즐겨찾기
- SP6-iv: broker 매물 등록
- SP6-data-sync: 외부 API → DB sync
- SP6-iam-infra: Zitadel Pulumi 화 + production HTTPS 검증

---

## Spec coverage 자가 점검

| Spec § | 요구사항 | 구현 task |
|---|---|---|
| 2.1 Frontend 화면 | `/listings` page + 지도 + 카드 + 필터 + 무한 스크롤 + skeleton | T6 (page) + T3 (map) + T5 (card) + T4 (filter) |
| 2.1 Backend API | `GET /listings` + bounds + filter + page + sort | T1 |
| 2.1 디자인 | Pretendard self-host + Card/Range/MultiSelect + dark mode | T7 + T4 |
| 2.1 Naver Maps | SDK + 핀 + 클러스터 + bounds 이벤트 | T3 |
| 2.1 Mobile responsive | 작은 화면 toggle | T6 (md:grid-cols-) + T8 (e2e) |
| 4 API contract | RFC 7807 + zod | T1 (backend) + T2 (frontend zod) |
| 6 Task 분해 | T1-T9 | 전체 |
| 7 SSS 7기둥 | 일관성/자동강제/추적성/안전성/가시성/SSOT/명확성 | T1-T9 분산 |
| 8 Testing | unit + integration + e2e + a11y + mobile + bundle | T2-T8 |
| 9 디자인 시스템 | Pretendard self-host + dark mode + range/multi primitive | T7 + T4 |
| 10 Open questions | 5 종 (사진 / 지역 검색 / debounce / 클러스터 / 무한 스크롤) | T1/T3/T4/T5 시점 결정 |

**미반영 = 0**.
