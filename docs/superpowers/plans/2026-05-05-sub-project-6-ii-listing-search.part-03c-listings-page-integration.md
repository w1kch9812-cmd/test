# Sub-project 6-ii Listing Search - Part 03C: Listings Page Integration

Parent index: [Sub-project 6-ii Listing Search - Part 03](./2026-05-05-sub-project-6-ii-listing-search.part-03.md).
## Task 6: `/(authenticated)/listings/page.tsx` 통합 + i18n

**Files:**
- Create: `apps/web/app/(authenticated)/listings/page.tsx`
- Create: `apps/web/app/(authenticated)/listings/loading.tsx`

- [ ] **Step 6.1: page.tsx**

`apps/web/app/(authenticated)/listings/page.tsx`:

```typescript
import { getTranslations } from "next-intl/server";
import { ListingMap } from "@/components/listings/listing-map";
import { ListingCardList } from "@/components/listings/listing-card-list";
import { FilterBar } from "@/components/listings/filter-bar";
import { SearchBar } from "@/components/listings/search-bar";

export default async function ListingsPage() {
  const t = await getTranslations("listings.page");

  return (
    <main className="flex h-screen flex-col">
      <header className="flex items-center justify-between border-b border-border p-4">
        <h1 className="text-xl font-bold">{t("title")}</h1>
        <SearchBar />
      </header>
      <div className="grid flex-1 grid-cols-1 overflow-hidden md:grid-cols-[280px_1fr_400px]">
        <aside className="overflow-y-auto border-r border-border md:block hidden">
          <FilterBar />
        </aside>
        <section className="relative h-full" aria-label={t("title")}>
          <ListingMap listings={[]} />
        </section>
        <aside className="overflow-y-auto border-l border-border">
          <ListingCardList />
        </aside>
      </div>
    </main>
  );
}
```

(NOTE: `<ListingMap listings={[]} />` — 실 listings 는 ListingCardList 의 useInfiniteQuery 가 fetch. ListingMap 도 같은 query 의 `data?.pages.flatMap` 을 받아야 — 이건 client side 에서 useListingsStore 통해 같이 sync. T6 의 이 통합은 단순화 — ListingMap 이 자체적으로 useInfiniteQuery 호출하거나 또는 listings prop 으로 받음. 후자가 더 단순.)

**개선** — ListingMap 도 listings prop 받기:

```typescript
"use client";
import { useInfiniteQuery } from "@tanstack/react-query";
import { ListingMap } from "@/components/listings/listing-map";
import { ListingCardList } from "@/components/listings/listing-card-list";
import { fetchListings, type ListingsResponse } from "@/lib/listings/api";
import { useListingsStore } from "@/stores/listings";

export function ListingsContent() {
  const filters = useListingsStore((s) => s.filters);
  const bounds = useListingsStore((s) => s.bounds);
  const query = useInfiniteQuery<ListingsResponse>({
    queryKey: ["listings", filters, bounds],
    queryFn: ({ pageParam }) =>
      fetchListings({ filters, bounds, page: pageParam as number, size: 20 }),
    initialPageParam: 0,
    getNextPageParam: (last) => (last.has_next ? last.page + 1 : undefined),
    enabled: bounds !== undefined,
  });
  const allListings = query.data?.pages.flatMap((p) => p.listings) ?? [];

  return (
    <>
      <section className="relative h-full">
        <ListingMap listings={allListings} />
      </section>
      <aside className="overflow-y-auto border-l border-border">
        <ListingCardList query={query} />
      </aside>
    </>
  );
}
```

이 통합 component 를 만들고 page.tsx 에서 사용. ListingCardList 는 query 를 prop 으로 받도록 변경 (또는 hook 분리).

**SSS 결정**: query 를 별도 hook (`useListingsQuery`) 으로 분리 — ListingMap, ListingCardList 둘 다 사용. 단일 query, 캐시 공유.

`apps/web/lib/listings/use-listings-query.ts`:

```typescript
"use client";
import { useInfiniteQuery } from "@tanstack/react-query";
import { fetchListings, type ListingsResponse } from "@/lib/listings/api";
import { useListingsStore } from "@/stores/listings";

export function useListingsQuery() {
  const filters = useListingsStore((s) => s.filters);
  const bounds = useListingsStore((s) => s.bounds);
  return useInfiniteQuery<ListingsResponse>({
    queryKey: ["listings", filters, bounds],
    queryFn: ({ pageParam }) =>
      fetchListings({ filters, bounds, page: pageParam as number, size: 20 }),
    initialPageParam: 0,
    getNextPageParam: (last) => (last.has_next ? last.page + 1 : undefined),
    enabled: bounds !== undefined,
  });
}
```

ListingMap + ListingCardList 둘 다 `const query = useListingsQuery()` 호출 — TanStack Query 는 동일 queryKey 를 cache 공유.

ListingMap 의 props 변경:

```typescript
export function ListingMap() {
  const query = useListingsQuery();
  const listings = query.data?.pages.flatMap((p) => p.listings) ?? [];
  // ... 기존 동작
}
```

ListingCardList 도 `const query = useListingsQuery()`. (Step 5.2 의 query 를 hook 호출로 변경.)

이 변경 위해 Step 5.2 의 listing-card-list.tsx 를 hook 호출로 수정:

```typescript
import { useListingsQuery } from "@/lib/listings/use-listings-query";

export function ListingCardList() {
  const query = useListingsQuery();
  // ... 기존 (query 직접 호출 부분만 변경)
}
```

ListingMap 도 동일.

- [ ] **Step 6.2: page.tsx 단순화**

```typescript
import { getTranslations } from "next-intl/server";
import { ListingMap } from "@/components/listings/listing-map";
import { ListingCardList } from "@/components/listings/listing-card-list";
import { FilterBar } from "@/components/listings/filter-bar";
import { SearchBar } from "@/components/listings/search-bar";

export default async function ListingsPage() {
  const t = await getTranslations("listings.page");
  return (
    <main className="flex h-screen flex-col">
      <header className="flex items-center justify-between border-b border-border p-4">
        <h1 className="text-xl font-bold">{t("title")}</h1>
        <SearchBar />
      </header>
      <div className="grid flex-1 grid-cols-1 overflow-hidden md:grid-cols-[280px_1fr_400px]">
        <aside className="overflow-y-auto border-r border-border md:block hidden">
          <FilterBar />
        </aside>
        <section className="relative h-full">
          <ListingMap />
        </section>
        <aside className="overflow-y-auto border-l border-border">
          <ListingCardList />
        </aside>
      </div>
    </main>
  );
}
```

- [ ] **Step 6.3: loading.tsx**

`apps/web/app/(authenticated)/listings/loading.tsx`:

```typescript
import { getTranslations } from "next-intl/server";

export default async function ListingsLoading() {
  const t = await getTranslations("listings");
  return (
    <main className="flex h-screen items-center justify-center">
      <div className="text-sm text-muted-foreground">{t("loading")}</div>
    </main>
  );
}
```

- [ ] **Step 6.4: typecheck + dev 로컬 시연**

```bash
pnpm typecheck
# 별도 터미널: pnpm --filter=@gongzzang/web dev
# http://localhost:3000/listings 접속 (로그인 후)
```

- [ ] **Step 6.5: Commit**

```bash
git add apps/web/app/\(authenticated\)/listings/ apps/web/lib/listings/use-listings-query.ts apps/web/components/listings/listing-map.tsx apps/web/components/listings/listing-card-list.tsx
git commit -m "feat(6ii-T6): /(authenticated)/listings 통합 + useListingsQuery hook

- /listings/page.tsx: 3-column 그리드 (필터/지도/카드 list) + 헤더 (제목 + 검색바)
- useListingsQuery: 단일 useInfiniteQuery hook (ListingMap + ListingCardList 캐시 공유)
- loading.tsx: Suspense fallback (i18n)"
```

---
