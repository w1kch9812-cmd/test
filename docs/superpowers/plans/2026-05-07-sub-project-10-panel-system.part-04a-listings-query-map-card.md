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

