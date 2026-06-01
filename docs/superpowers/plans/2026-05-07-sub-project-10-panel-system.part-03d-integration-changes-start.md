## Task 6: 통합 변경 + lint rules + e2e + a11y + 회귀

**목표:** Spec § 11 통합 변경 적용, § 10.1 lint rules, § 10.2 e2e, § 10.3 extensibility 회귀, § 10.4 텔레메트리 검증.

**Files (modify):**
- `apps/web/app/(authenticated)/listings/page.tsx`
- `apps/web/components/listings/listing-map.tsx`
- `apps/web/components/listings/listing-card.tsx`
- `apps/web/components/listings/listing-card-list.tsx`
- `apps/web/lib/listings/use-listings-query.ts`
- `apps/web/stores/listings.ts`
- `apps/web/lib/listings/filters.ts`
- `apps/web/lib/listings/api.ts` (fetchListings 의 pnu query param 직접화)
- `apps/web/tests/unit/listings/filters.test.ts` (pnu 케이스 제거)
- `apps/web/biome.json`
- `lefthook.yml`

**Files (create):**
- `apps/web/tests/unit/panel-extensibility.test.ts`
- `apps/web/tests/e2e/panel-system.spec.ts`

**Files (delete):**
- `apps/web/components/listings/parcel-info-panel.tsx`

### Step 6.1: Refactor `fetchListings` to accept pnu directly

- [ ] **Step 6.1.1: Modify `apps/web/lib/listings/api.ts`**

Change `FetchListingsInput` and `fetchListings` so that `pnu` is a top-level param, not from filters:

```ts
// Replace the existing FetchListingsInput / fetchListings region with:
export interface FetchListingsInput {
  filters: ListingFilters;
  bounds?: { south: number; west: number; north: number; east: number };
  pnu?: string;
  page?: number;
  size?: number;
}

export async function fetchListings(input: FetchListingsInput): Promise<ListingsResponse> {
  const sp = toSearchParams(input.filters);
  if (input.bounds) {
    const { south, west, north, east } = input.bounds;
    sp.set('bounds', `${south},${west},${north},${east}`);
  }
  if (input.pnu) sp.set('pnu', input.pnu);
  if (input.page !== undefined) sp.set('page', String(input.page));
  if (input.size !== undefined) sp.set('size', String(input.size));

  const json = await api.get(`listings?${sp.toString()}`).json<unknown>();
  return ListingsResponseSchema.parse(json);
}
```

- [ ] **Step 6.1.2: Modify `apps/web/lib/listings/filters.ts`**

Remove `pnu` from `ListingFilters` and from `parseFiltersFromSearchParams` / `toSearchParams`:

```ts
// Edit ListingFilters interface — delete the pnu field:
export interface ListingFilters {
  types: ListingType[];
  transactions: TransactionType[];
  minAreaM2: number | undefined;
  maxAreaM2: number | undefined;
  minPriceKrw: number | undefined;
  maxPriceKrw: number | undefined;
  sort: SortKey;
  adminCode: string | undefined;
  landUseType: string | undefined;
}

// Edit parseFiltersFromSearchParams — remove the pnu line:
//   pnu: sp.get("pnu") ?? undefined,   ← delete

// Edit toSearchParams — remove the pnu branch:
//   if (f.pnu) sp.set("pnu", f.pnu);   ← delete
```

- [ ] **Step 6.1.3: Update `apps/web/tests/unit/listings/filters.test.ts`**

Read the file. Delete any test that checks `pnu` parsing/serialization, since the field is gone.

- [ ] **Step 6.1.4: Run unit tests**

Run: `cd apps/web && pnpm test`
Expected: pass.

- [ ] **Step 6.1.5: Commit**

```bash
git add apps/web/lib/listings/api.ts apps/web/lib/listings/filters.ts apps/web/tests/unit/listings/filters.test.ts
git commit -m "refactor(sp10-t6): fetchListings pnu top-level + remove filters.pnu (panel stack derives)"
```

