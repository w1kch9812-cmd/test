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

