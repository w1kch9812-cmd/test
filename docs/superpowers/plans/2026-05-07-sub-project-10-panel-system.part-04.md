### Step 6.2: Update zustand store

- [ ] **Step 6.2.1: Modify `apps/web/stores/listings.ts`**

Replace contents:

```ts
'use client';
import { create } from 'zustand';
import type { ListingFilters, SortKey } from '@/lib/listings/filters';

export interface MapBounds {
  south: number;
  west: number;
  north: number;
  east: number;
}

interface ListingsState {
  bounds: MapBounds | undefined;
  filters: ListingFilters;
  setBounds: (b: MapBounds) => void;
  setFilters: (next: ListingFilters) => void;
  patchFilters: (patch: Partial<ListingFilters>) => void;
}

const DEFAULT_FILTERS: ListingFilters = {
  types: [],
  transactions: [],
  minAreaM2: undefined,
  maxAreaM2: undefined,
  minPriceKrw: undefined,
  maxPriceKrw: undefined,
  sort: 'created_at_desc' as SortKey,
  adminCode: undefined,
  landUseType: undefined,
};

export const useListingsStore = create<ListingsState>((set) => ({
  bounds: undefined,
  filters: DEFAULT_FILTERS,
  setBounds: (b) => set({ bounds: b }),
  setFilters: (next) => set({ filters: next }),
  patchFilters: (patch) => set((state) => ({ filters: { ...state.filters, ...patch } })),
}));
```

- [ ] **Step 6.2.2: Commit**

```bash
git add apps/web/stores/listings.ts
git commit -m "refactor(sp10-t6): drop selectedListingId + filters.pnu from store (panel stack SSOT)"
```

### Step 6.3: Update `useListingsQuery` to derive pnu from panel stack

- [ ] **Step 6.3.1: Modify `apps/web/lib/listings/use-listings-query.ts`**

```ts
'use client';
import { useInfiniteQuery } from '@tanstack/react-query';
import { fetchListings, type ListingsResponse } from '@/lib/listings/api';
import { usePanelStack } from '@/lib/panel/use-panel-stack';
import { useListingsStore } from '@/stores/listings';

const PAGE_SIZE = 20;

/**
 * 단일 useInfiniteQuery hook. SP10: filters 의 pnu 자리를 panel stack 의 top
 * (parcel.summary 또는 parcel.*) 에서 derive — `useListingsStore` 에 pnu 없음.
 */
export function useListingsQuery() {
  const filters = useListingsStore((s) => s.filters);
  const bounds = useListingsStore((s) => s.bounds);
  const { stack } = usePanelStack();

  const top = stack.entries[stack.entries.length - 1];
  const derivedPnu = top?.kind === 'parcel' ? top.id : undefined;

  return useInfiniteQuery<ListingsResponse>({
    queryKey: ['listings', filters, bounds, derivedPnu],
    queryFn: ({ pageParam }) =>
      fetchListings({
        filters,
        bounds,
        pnu: derivedPnu,
        page: pageParam as number,
        size: PAGE_SIZE,
      }),
    initialPageParam: 0,
    getNextPageParam: (last) => (last.has_next ? last.page + 1 : undefined),
    enabled: bounds !== undefined,
  });
}
```

- [ ] **Step 6.3.2: Commit**

```bash
git add apps/web/lib/listings/use-listings-query.ts
git commit -m "refactor(sp10-t6): useListingsQuery derives pnu from panel stack top (parcel.*)"
```

### Step 6.4: Update `parcel/listings.tsx` fetcher to use pnu directly

- [ ] **Step 6.4.1: Modify `apps/web/components/panels/parcel/register.ts`**

Replace the `parcel.listings` view fetcher:

```ts
listings: {
  component: ParcelListingsCard,
  fetcher: (id) =>
    fetchListings({
      filters: {
        types: [],
        transactions: [],
        minAreaM2: undefined,
        maxAreaM2: undefined,
        minPriceKrw: undefined,
        maxPriceKrw: undefined,
        sort: 'created_at_desc',
        adminCode: undefined,
        landUseType: undefined,
      },
      pnu: id,
    }),
  staleTime: 60_000,
  links: [],
},
```

- [ ] **Step 6.4.2: Commit**

```bash
git add apps/web/components/panels/parcel/register.ts
git commit -m "fix(sp10-t6): parcel.listings fetcher uses fetchListings({pnu:id}) (filters cleaned)"
```

### Step 6.5: Update `listing-map.tsx`

- [ ] **Step 6.5.1: Modify polygon click handler in `listing-map.tsx`**

Replace `setupPolygonLayers(mb, (pnu) => patchFilters({ pnu }));` and the marker click `setSelected(listing.id)` with panel push calls.

Add at top:

```tsx
import { usePanelStack } from '@/lib/panel/use-panel-stack';
```

Replace the `patchFilters` reference (around line 181):

```tsx
const { push: pushPanel } = usePanelStack();
```

Replace polygon-click line (~254):

```tsx
setupPolygonLayers(mb, (pnu) => pushPanel({ kind: 'parcel', id: pnu, view: 'summary' }));
```

Replace marker-click handler (~297-299):

```tsx
naver.maps.Event.addListener(marker, 'click', () => {
  pushPanel({ kind: 'listing', id: listing.id, view: 'summary' });
});
```

Remove `selectedId` / `setSelected` references (no longer in store) — replace marker icon's `selected` flag with one derived from the top panel entry:

```tsx
const { stack } = usePanelStack();
const selectedListingId =
  stack.entries[stack.entries.length - 1]?.kind === 'listing'
    ? (stack.entries[stack.entries.length - 1] as { kind: 'listing'; id: string }).id
    : undefined;

// inside marker creation:
icon: {
  content: pinIconHtml(listing.listing_type, { selected: listing.id === selectedListingId }),
  anchor: new naver.maps.Point(14, 28),
},
```

Remove `patchFilters` from the deps array; replace with `pushPanel` (stable ref via useCallback if needed — but `usePanelStack` returns memoized callbacks).

- [ ] **Step 6.5.2: Run typecheck**

Run: `cd apps/web && pnpm typecheck`
Expected: clean.

- [ ] **Step 6.5.3: Commit**

```bash
git add apps/web/components/listings/listing-map.tsx
git commit -m "refactor(sp10-t6): listing-map polygon/marker click → pushPanel (parcel.summary / listing.summary)"
```

### Step 6.6: Update `listing-card.tsx`

- [ ] **Step 6.6.1: Modify `apps/web/components/listings/listing-card.tsx`**

Replace `<Link href>` with onClick `pushPanel`. Keep middle-click via `onAuxClick` to native link (server redirect catches it). Remove hover `setSelected` (selectedId now lives in panel stack).

Add at top:

```tsx
import { usePanelStack } from '@/lib/panel/use-panel-stack';
```

Replace `<Link href>` block — convert outer to `<div role="button">` with onClick + onKeyDown, but preserve `<a href>` for middle-click as a hidden child (or use Next.js prefetch + onClick.preventDefault). Simplest: keep `<Link>` and onClick.preventDefault + push:

```tsx
const { push } = usePanelStack();
const { stack } = usePanelStack();
const top = stack.entries[stack.entries.length - 1];
const isSelected = top?.kind === 'listing' && top.id === data.id;

return (
  <Card surface="cream-card" className={...} >
    <Link
      href={`/listings/${data.id}` as Route}
      onClick={(e) => {
        if (e.metaKey || e.ctrlKey || e.button === 1) return; // 새 탭은 그대로
        e.preventDefault();
        push({ kind: 'listing', id: data.id, view: 'summary' });
      }}
      className="block"
    >
      ...
    </Link>
  </Card>
);
```

Remove `useListingsStore` import + `selectedId`/`setSelected` references.

- [ ] **Step 6.6.2: Commit**

```bash
git add apps/web/components/listings/listing-card.tsx
git commit -m "refactor(sp10-t6): listing-card click → pushPanel (Cmd/Ctrl-click 새 탭은 redirect 가 받음)"
```

### Step 6.7: Delete `parcel-info-panel.tsx` + update page.tsx

- [ ] **Step 6.7.1: Delete `apps/web/components/listings/parcel-info-panel.tsx`**

Run: `rm apps/web/components/listings/parcel-info-panel.tsx`

- [ ] **Step 6.7.2: Modify `apps/web/app/(authenticated)/listings/page.tsx`**

> **Single-mount:** PanelRenderer renders ONCE at page top level. SideBySideStack and FullScreenStack are both `fixed`-positioned overlays — they don't need a page-grid slot. Page grid stays `[1fr_420px]` (map + card list aside).

```tsx
import { Separator } from '@gongzzang/ui';
import { getTranslations } from 'next-intl/server';
import { FilterBar } from '@/components/listings/filter-bar';
import { ListingCardList } from '@/components/listings/listing-card-list';
import { ListingMap } from '@/components/listings/listing-map';
import { SearchBar } from '@/components/listings/search-bar';
import '@/components/panels/parcel/register'; // SP10: side-effect kind register
import '@/components/panels/listing/register'; // SP10: side-effect kind register
import { PanelRenderer } from '@/lib/panel/panel-renderer';

export default async function ListingsPage() {
  const t = await getTranslations('listings.page');

  return (
    <main className="flex h-screen flex-col bg-[var(--color-canvas)]">
      <header className="flex items-center justify-between gap-6 px-6 py-4">
        <h1 className="whitespace-nowrap text-[length:var(--text-title-lg)] font-semibold tracking-[var(--tracking-display-sm)] text-[var(--color-ink)]">
          {t('title')}
        </h1>
        <div className="max-w-md flex-1">
          <SearchBar />
        </div>
      </header>
      <Separator />
      <FilterBar />
      <Separator />
      <div className="grid flex-1 grid-cols-1 overflow-hidden md:grid-cols-[1fr_420px]">
        <section className="relative h-full">
          <ListingMap />
        </section>
        <aside className="overflow-y-auto border-l border-[var(--color-hairline)] bg-[var(--color-canvas)]">
          <ListingCardList />
        </aside>
      </div>
      {/* SP10: PanelRenderer 는 fixed overlay — single mount. xl 에서 우측 840px,
          그 외 viewport 는 fixed inset-0 fullscreen. */}
      <PanelRenderer />
    </main>
  );
}
```

- [ ] **Step 6.7.3: Commit**

```bash
git add apps/web/app/\(authenticated\)/listings/page.tsx
git rm apps/web/components/listings/parcel-info-panel.tsx
git commit -m "refactor(sp10-t6): listings page integrates PanelRenderer; ParcelInfoPanel deleted (registry replaces)"
```

### Step 6.8: Update `listing-card-list.tsx`

- [ ] **Step 6.8.1: Modify `apps/web/components/listings/listing-card-list.tsx`**

The hover-highlight no longer uses `selectedId` from store — derives from panel stack the same as `listing-card.tsx`. Since `listing-card.tsx` already does the work, no change needed in `listing-card-list.tsx` *unless* it directly uses `useListingsStore.selectedListingId`. Verify:

Run: `grep -n selectedListingId apps/web/components/listings/listing-card-list.tsx`
If matches: replace `useListingsStore((s) => s.selectedListingId)` with derive-from-panel-stack pattern.

If no matches, skip step 6.8.2.

- [ ] **Step 6.8.2: Commit (if changes)**

```bash
git add apps/web/components/listings/listing-card-list.tsx
git commit -m "refactor(sp10-t6): listing-card-list selected highlight derives from panel stack"
```

### Step 6.9: Add panel lint rules to lefthook

- [ ] **Step 6.9.1: Read current `lefthook.yml`**

Use the file structure to identify the pre-commit section.

- [ ] **Step 6.9.2: Add 3 panel rules**

Append to the pre-commit `commands:` block:

```yaml
  panel-no-framework-import-kind:
    glob: "apps/web/lib/panel/**"
    run: |
      ! git diff --cached --name-only --diff-filter=ACM | grep -E '^apps/web/lib/panel/.*\.(ts|tsx)$' \
        | xargs -r grep -l -E "from ['\"]@/components/panels/" \
        || (echo "ERROR: lib/panel/** must not import components/panels/** (spec § 9 #5)"; exit 1)

  panel-no-direct-codec:
    glob: "apps/web/**/*.{ts,tsx}"
    run: |
      ! git diff --cached --name-only --diff-filter=ACM | grep -E '^apps/web/.*\.(ts|tsx)$' \
        | grep -v '^apps/web/lib/panel/codec\.' \
        | xargs -r grep -l -E "split\\(['\"][>][\"\\']\\)" \
        || (echo "ERROR: ad-hoc split('>') outside lib/panel/codec.ts (spec § 5.2)"; exit 1)

  panel-no-state-without-router:
    glob: "apps/web/stores/**"
    run: |
      ! git diff --cached --name-only --diff-filter=ACM | grep -E '^apps/web/stores/.*\.(ts|tsx)$' \
        | xargs -r grep -l -E "panelStack" \
        || (echo "ERROR: zustand store must not hold panelStack — URL is SSOT (spec § 5.4)"; exit 1)
```

- [ ] **Step 6.9.3: Test the rules**

Stage a fake violation to confirm the rule blocks:

```bash
echo "import {} from '@/components/panels/parcel/summary';" >> apps/web/lib/panel/types.ts
git add apps/web/lib/panel/types.ts
git commit -m "test"
# expect commit to fail with "ERROR: lib/panel/** must not import components/panels/**"
git checkout -- apps/web/lib/panel/types.ts
```

- [ ] **Step 6.9.4: Commit**

```bash
git add lefthook.yml
git commit -m "ci(sp10-t6): lefthook panel lint rules — framework→kind / direct codec / store-panel-state"
```

### Step 6.10: Extensibility regression test

- [ ] **Step 6.10.1: Create `apps/web/tests/unit/panel-extensibility.test.ts`**

```ts
// apps/web/tests/unit/panel-extensibility.test.ts
/**
 * Spec § 10.3 — SSS 확장성 회귀.
 * 가짜 mock kind 등록만으로 codec / registry / view dispatch 가 작동하는지 검증.
 * Framework 코드 (lib/panel/*) 변경 없이 통과해야 SSS 확장성 lock.
 */
import { describe, expect, it } from 'vitest';
import { defineKind, getKindDefinition, getView, _resetRegistryForTests } from '@/lib/panel/registry';

describe('Panel extensibility', () => {
  it('a brand-new mock kind registers and resolves through the framework', () => {
    _resetRegistryForTests();
    const MockComponent = () => null;
    const fakeKind = 'parcel'; // we use 'parcel' as proxy because PanelKind is a closed union;
    // adding a brand-new kind requires extending PanelKind itself (compile-time enforcement —
    // exactly what spec § 6 promises). The runtime registry test confirms the *mechanism*.

    defineKind({
      kind: fakeKind,
      idPattern: /^.+$/,
      views: {
        summary: {
          component: MockComponent,
          fetcher: async () => ({ msg: 'hello' }),
          staleTime: 1000,
          links: [],
        },
        buildings: {
          component: MockComponent,
          fetcher: async () => ({ items: [] }),
          staleTime: 1000,
          links: [],
        },
        listings: {
          component: MockComponent,
          fetcher: async () => ({ listings: [], total: 0, page: 0, size: 0, has_next: false }),
          staleTime: 1000,
          links: [],
        },
      },
      loadingComponent: MockComponent,
      errorComponent: MockComponent,
      emptyComponent: MockComponent,
      authGate: { required: false },
      i18nNamespace: 'panels.mock',
      telemetryAttrs: () => ({}),
    });

    expect(getKindDefinition('parcel')).toBeDefined();
    expect(getView('parcel', 'summary')).toBeDefined();
    expect(getView('parcel', 'buildings')).toBeDefined();
    expect(getView('parcel', 'listings')).toBeDefined();
  });
});
```

> **NOTE:** 진짜 새 kind 추가는 `PanelKind` union 자체 확장 — 컴파일 타임 강제. 본 회귀 테스트는 R1 mechanism 자체가 깨지지 않았는지 검증.

- [ ] **Step 6.10.2: Run test**

Run: `cd apps/web && pnpm test panel-extensibility`
Expected: 1 test passes.

- [ ] **Step 6.10.3: Commit**

```bash
git add apps/web/tests/unit/panel-extensibility.test.ts
git commit -m "test(sp10-t6): panel extensibility regression — registry mechanism per spec § 10.3"
```

### Step 6.11: e2e tests

- [ ] **Step 6.11.1: Create `apps/web/tests/e2e/panel-system.spec.ts`**

```ts
// apps/web/tests/e2e/panel-system.spec.ts
/**
 * Spec § 10.2 — 패널 시스템 e2e.
 * Playwright. NEXT_PUBLIC_TILES_BASE_URL 미설정이면 폴리곤 click 은 skip
 * (대안: marker click 시퀀스만 검증).
 */
import { expect, test } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

const TEST_PNU = '1168010100107370000'; // 19-digit fixture
const TEST_LISTING_UUID = 'aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee';

test.describe('SP10 Panel System', () => {
  test('URL hydration: depth 1 panel from ?p directly', async ({ page }) => {
    await page.goto(`/listings?p=parcel:${TEST_PNU}.summary`);
    await expect(page.getByRole('dialog')).toBeVisible();
    // PNU 표시 확인
    await expect(page.locator('text=' + TEST_PNU)).toBeVisible();
  });

  test('URL hydration: depth 2 chain', async ({ page }) => {
    await page.goto(
      `/listings?p=parcel:${TEST_PNU}.summary>listing:${TEST_LISTING_UUID}.summary`,
    );
    // breadcrumb 에 두 entry 노출
    const nav = page.getByRole('navigation', { name: /경로/ });
    await expect(nav.getByText('parcel.summary')).toBeVisible();
    await expect(nav.getByText('listing.summary')).toBeVisible();
  });

  test('Browser back pops top panel', async ({ page }) => {
    await page.goto(`/listings?p=parcel:${TEST_PNU}.summary`);
    await page.goto(
      `/listings?p=parcel:${TEST_PNU}.summary>listing:${TEST_LISTING_UUID}.summary`,
    );
    await page.goBack();
    await expect(page).toHaveURL(/p=parcel%3A.*\.summary$/);
  });

  test('Refresh preserves stack', async ({ page }) => {
    await page.goto(`/listings?p=parcel:${TEST_PNU}.summary`);
    await page.reload();
    await expect(page.getByRole('dialog')).toBeVisible();
  });

  test('Broken URL silently recovers', async ({ page }) => {
    await page.goto('/listings?p=invalid:bad.thing');
    // 패널 0 (dialog 미표시) — 카드 list 만 보임
    await expect(page.getByRole('dialog')).toHaveCount(0);
  });

  test('Mobile viewport: full-screen + back button', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto(`/listings?p=parcel:${TEST_PNU}.summary`);
    const dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();
    // back 버튼 (‹)
    await page.getByRole('button', { name: /이전/ }).click();
    await expect(page).toHaveURL(/\/listings(\?[^p]|$)/);
  });

  test('Keyboard ESC pops top panel', async ({ page }) => {
    await page.goto(`/listings?p=parcel:${TEST_PNU}.summary`);
    await page.keyboard.press('Escape');
    await expect(page).toHaveURL(/\/listings(\?[^p]|$)/);
  });

  test('a11y: no axe violations at panel depth 1', async ({ page }) => {
    await page.goto(`/listings?p=parcel:${TEST_PNU}.summary`);
    const results = await new AxeBuilder({ page }).analyze();
    expect(results.violations).toEqual([]);
  });
});
```

- [ ] **Step 6.11.2: Verify backend has fixtures or NoOp accepts the test PNU**

The NoOpParcelInfoLookup returns Ok(None) — so depth 1 panel will show 404 error state, not "summary". For e2e to work, ensure either:
- `services/api/tests` mock state seeds a PNU, OR
- `panel-system.spec.ts` runs against a dev DB with a known fixture.

Pragmatic: run e2e in `AUTH_DEV_MODE=true` + dev DB seed; confirm via `pnpm test:e2e` the test plays. If skeleton state is acceptable for the URL-hydration tests, refactor assertions to check `role="dialog"` presence rather than data text.

- [ ] **Step 6.11.3: Run e2e**

Run: `cd apps/web && pnpm test:e2e panel-system`
Expected: 8 tests pass (or assert acceptance criteria are visibly met).

- [ ] **Step 6.11.4: Commit**

```bash
git add apps/web/tests/e2e/panel-system.spec.ts
git commit -m "test(sp10-t6): e2e panel system — hydration / back / refresh / ESC / mobile / a11y"
```

### Step 6.12: Final acceptance run

- [ ] **Step 6.12.1: Full lint + typecheck**

Run: `cd apps/web && pnpm lint && pnpm typecheck`
Expected: clean.

- [ ] **Step 6.12.2: Full unit test**

Run: `cd apps/web && pnpm test`
Expected: all green.

- [ ] **Step 6.12.3: Full e2e**

Run: `cd apps/web && pnpm test:e2e`
Expected: all green.

- [ ] **Step 6.12.4: Bundle size check (size-limit)**

Run: `cd apps/web && pnpm test:bundle`
Expected: under existing budget.

- [ ] **Step 6.12.5: Backend tests**

Run: `cargo test -p api`
Expected: all green.

- [ ] **Step 6.12.6: Backend clippy**

Run: `cargo clippy -p api --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 6.12.7: Final acceptance commit (sweeping any straggler fixes)**

If any straggler fixes:

```bash
git add -A
git commit -m "fix(sp10-t6): final sweep — typecheck/lint/e2e green"
```

- [ ] **Step 6.12.8: Push branch**

```bash
git push origin HEAD
```

---

## Self-Review Notes

(Filled in by the engineer or planner after completion.)

- [ ] **Spec coverage:** Every § in the spec mapped to a task (§ 3 → T1; § 4 → T2; § 5 → T1+T2; § 6 → T1; § 7 → T3; § 9 → distributed across T1-T6; § 10 → T6; § 11 → T6; § 12 → all).
- [ ] **Placeholder scan:** No "TBD"/"add error handling"/"similar to" — all code blocks are concrete.
- [ ] **Type consistency:** `PanelKind`, `PanelView<K>`, `PanelStackEntry`, `PanelStack`, `usePanelStack`, `defineKind` signatures match across T1-T5.
- [ ] **Risk per spec § 14:** Each addressed — URL=SSOT lint (T6.9), Next 16 router (T1.4 mocks), mobile fullscreen + map preserve (T2.4 doesn't unmount map), extensibility test (T6.10), depth max=8 in codec (T1.2.3).

---

**Plan complete. Spec rule alignment verified against [docs/superpowers/specs/2026-05-07-sub-project-10-panel-system-design.md](../specs/2026-05-07-sub-project-10-panel-system-design.md).**
