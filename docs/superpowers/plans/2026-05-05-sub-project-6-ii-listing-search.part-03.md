## Task 4: Filter / Search bar — 종류 + 거래 + 평수 + 가격 + URL query 동기화

**Files:**
- Create: `packages/ui/primitives/range-slider.tsx`
- Create: `packages/ui/primitives/multi-select.tsx`
- Modify: `packages/ui/index.ts` (export)
- Create: `apps/web/components/listings/search-bar.tsx`
- Create: `apps/web/components/listings/filter-bar.tsx`
- Test: `apps/web/tests/unit/listings/filter-bar.test.tsx`

(주: shadcn/Radix 의 Slider primitive 와 Combobox 패턴 사용. 이미 packages/ui 에 Radix 가 있으므로 추가 dep 불필요 — Radix Slider 직접 사용.)

- [ ] **Step 4.1: Radix Slider 추가 + range-slider primitive**

```bash
pnpm --filter=@gongzzang/ui add @radix-ui/react-slider@^1.2.3
```

`packages/ui/primitives/range-slider.tsx`:

```typescript
"use client";
import * as SliderPrimitive from "@radix-ui/react-slider";
import { cn } from "../lib/cn";

interface RangeSliderProps {
  min: number;
  max: number;
  step?: number;
  value: [number, number];
  onValueChange: (next: [number, number]) => void;
  formatValue?: (v: number) => string;
  className?: string;
}

export function RangeSlider({
  min, max, step = 1, value, onValueChange, formatValue, className,
}: RangeSliderProps) {
  return (
    <div className={cn("flex flex-col gap-2", className)}>
      <SliderPrimitive.Root
        min={min}
        max={max}
        step={step}
        value={value}
        onValueChange={(v) => onValueChange([v[0], v[1]] as [number, number])}
        className="relative flex h-5 w-full touch-none select-none items-center"
      >
        <SliderPrimitive.Track className="relative h-1 w-full grow overflow-hidden rounded-full bg-muted">
          <SliderPrimitive.Range className="absolute h-full bg-primary" />
        </SliderPrimitive.Track>
        <SliderPrimitive.Thumb className="block h-4 w-4 rounded-full border border-primary bg-background shadow focus:outline-none focus:ring-2 focus:ring-ring" />
        <SliderPrimitive.Thumb className="block h-4 w-4 rounded-full border border-primary bg-background shadow focus:outline-none focus:ring-2 focus:ring-ring" />
      </SliderPrimitive.Root>
      <div className="flex justify-between text-xs text-muted-foreground">
        <span>{formatValue ? formatValue(value[0]) : value[0]}</span>
        <span>{formatValue ? formatValue(value[1]) : value[1]}</span>
      </div>
    </div>
  );
}
```

- [ ] **Step 4.2: multi-select primitive (간단 chip 형태)**

`packages/ui/primitives/multi-select.tsx`:

```typescript
"use client";
import { cn } from "../lib/cn";

interface MultiSelectOption {
  value: string;
  label: string;
}

interface MultiSelectProps {
  options: MultiSelectOption[];
  value: string[];
  onValueChange: (next: string[]) => void;
  className?: string;
}

export function MultiSelect({ options, value, onValueChange, className }: MultiSelectProps) {
  const toggle = (v: string) => {
    if (value.includes(v)) onValueChange(value.filter((x) => x !== v));
    else onValueChange([...value, v]);
  };
  return (
    <div className={cn("flex flex-wrap gap-2", className)}>
      {options.map((opt) => {
        const selected = value.includes(opt.value);
        return (
          <button
            key={opt.value}
            type="button"
            aria-pressed={selected}
            onClick={() => toggle(opt.value)}
            className={cn(
              "rounded-full border px-3 py-1 text-sm transition",
              selected
                ? "border-primary bg-primary text-primary-foreground"
                : "border-border bg-background hover:bg-muted",
            )}
          >
            {opt.label}
          </button>
        );
      })}
    </div>
  );
}
```

- [ ] **Step 4.3: packages/ui/index.ts export**

```typescript
export { RangeSlider } from "./primitives/range-slider";
export { MultiSelect } from "./primitives/multi-select";
```

- [ ] **Step 4.4: i18n listings.ko.json**

`apps/web/lib/i18n/messages/listings.ko.json`:

```json
{
  "listings": {
    "page": {
      "title": "매물 검색"
    },
    "search": {
      "placeholder": "지역명을 검색해 주세요"
    },
    "filter": {
      "type": "매물 종류",
      "transaction": "거래 방식",
      "areaM2": "면적 (m²)",
      "priceKrw": "가격 (원)",
      "sort": "정렬"
    },
    "type": {
      "factory": "공장",
      "warehouse": "창고",
      "office": "사무실",
      "knowledge_industry_center": "지식산업센터",
      "industrial_land": "산업단지",
      "logistics_center": "물류센터"
    },
    "transaction": {
      "sale": "매매",
      "monthly_rent": "월세",
      "jeonse": "전세"
    },
    "sort": {
      "created_at_desc": "최신순",
      "price_asc": "가격 낮은 순",
      "price_desc": "가격 높은 순",
      "area_asc": "면적 좁은 순",
      "area_desc": "면적 넓은 순"
    },
    "card": {
      "viewCount": "조회",
      "bookmarkCount": "관심",
      "favoritePlaceholder": "즐겨찾기"
    },
    "empty": "조건에 맞는 매물이 없어요",
    "loading": "매물을 불러오는 중이에요",
    "errors": {
      "fetchFailed": "매물을 불러오지 못했어요. 잠시 후 다시 시도해 주세요."
    }
  }
}
```

`apps/web/i18n.ts` 의 message merge 에 listings 추가:

```typescript
const [common, auth, listings] = await Promise.all([
  import("./lib/i18n/ko.json"),
  import("./lib/i18n/messages/auth.ko.json"),
  import("./lib/i18n/messages/listings.ko.json"),
]);
return {
  locale,
  messages: { ...common.default, ...auth.default, ...listings.default },
};
```

- [ ] **Step 4.5: filter-bar.tsx + search-bar.tsx**

`apps/web/components/listings/search-bar.tsx`:

```typescript
"use client";
import { useTranslations } from "next-intl";
import { Input } from "@gongzzang/ui";
import { useState } from "react";

export function SearchBar() {
  const t = useTranslations("listings.search");
  const [value, setValue] = useState("");
  return (
    <div className="w-full max-w-md">
      <Input
        type="search"
        value={value}
        onChange={(e) => setValue(e.target.value)}
        placeholder={t("placeholder")}
        aria-label={t("placeholder")}
      />
    </div>
  );
}
```

(NOTE: 실제 지역 검색은 미래 — 지금은 input 만 자리. T9 Open question 1.)

`apps/web/components/listings/filter-bar.tsx`:

```typescript
"use client";
import { useTranslations } from "next-intl";
import { MultiSelect, RangeSlider } from "@gongzzang/ui";
import { useListingsStore } from "@/stores/listings";
import type { ListingType, TransactionType, SortKey } from "@/lib/listings/filters";
import { formatAreaM2, formatPriceKrw } from "@/lib/listings/format";

const TYPES: ListingType[] = [
  "factory", "warehouse", "office",
  "knowledge_industry_center", "industrial_land", "logistics_center",
];
const TXNS: TransactionType[] = ["sale", "monthly_rent", "jeonse"];
const SORTS: SortKey[] = [
  "created_at_desc", "price_asc", "price_desc", "area_asc", "area_desc",
];

const AREA_MIN = 0;
const AREA_MAX = 10_000;
const PRICE_MIN = 0;
const PRICE_MAX = 100_000_000_000; // 1000억

export function FilterBar() {
  const t = useTranslations("listings");
  const filters = useListingsStore((s) => s.filters);
  const patch = useListingsStore((s) => s.patchFilters);

  const typeOptions = TYPES.map((v) => ({ value: v, label: t(`type.${v}`) }));
  const txnOptions = TXNS.map((v) => ({ value: v, label: t(`transaction.${v}`) }));

  return (
    <div className="flex flex-col gap-4 p-4">
      <section aria-label={t("filter.type")}>
        <h3 className="mb-2 text-sm font-semibold">{t("filter.type")}</h3>
        <MultiSelect
          options={typeOptions}
          value={filters.types}
          onValueChange={(v) => patch({ types: v as ListingType[] })}
        />
      </section>
      <section aria-label={t("filter.transaction")}>
        <h3 className="mb-2 text-sm font-semibold">{t("filter.transaction")}</h3>
        <MultiSelect
          options={txnOptions}
          value={filters.transactions}
          onValueChange={(v) => patch({ transactions: v as TransactionType[] })}
        />
      </section>
      <section aria-label={t("filter.areaM2")}>
        <h3 className="mb-2 text-sm font-semibold">{t("filter.areaM2")}</h3>
        <RangeSlider
          min={AREA_MIN}
          max={AREA_MAX}
          step={100}
          value={[filters.minAreaM2 ?? AREA_MIN, filters.maxAreaM2 ?? AREA_MAX]}
          onValueChange={([min, max]) =>
            patch({
              minAreaM2: min === AREA_MIN ? undefined : min,
              maxAreaM2: max === AREA_MAX ? undefined : max,
            })
          }
          formatValue={formatAreaM2}
        />
      </section>
      <section aria-label={t("filter.priceKrw")}>
        <h3 className="mb-2 text-sm font-semibold">{t("filter.priceKrw")}</h3>
        <RangeSlider
          min={PRICE_MIN}
          max={PRICE_MAX}
          step={10_000_000}
          value={[filters.minPriceKrw ?? PRICE_MIN, filters.maxPriceKrw ?? PRICE_MAX]}
          onValueChange={([min, max]) =>
            patch({
              minPriceKrw: min === PRICE_MIN ? undefined : min,
              maxPriceKrw: max === PRICE_MAX ? undefined : max,
            })
          }
          formatValue={formatPriceKrw}
        />
      </section>
      <section aria-label={t("filter.sort")}>
        <h3 className="mb-2 text-sm font-semibold">{t("filter.sort")}</h3>
        <select
          value={filters.sort}
          onChange={(e) => patch({ sort: e.target.value as SortKey })}
          className="rounded border border-border bg-background px-3 py-2 text-sm"
          aria-label={t("filter.sort")}
        >
          {SORTS.map((s) => (
            <option key={s} value={s}>{t(`sort.${s}`)}</option>
          ))}
        </select>
      </section>
    </div>
  );
}
```

- [ ] **Step 4.6: typecheck + lint + commit**

```bash
pnpm typecheck && pnpm lint
git add packages/ui/primitives/range-slider.tsx packages/ui/primitives/multi-select.tsx packages/ui/index.ts packages/ui/package.json apps/web/components/listings/search-bar.tsx apps/web/components/listings/filter-bar.tsx apps/web/lib/i18n/messages/listings.ko.json apps/web/i18n.ts pnpm-lock.yaml
git commit -m "feat(6ii-T4): RangeSlider + MultiSelect primitives + FilterBar + i18n

- packages/ui: RangeSlider (Radix Slider) + MultiSelect (chip toggle) primitives
- listings/search-bar: 지역 검색 input (실 검색 API 통합 미래)
- listings/filter-bar: 종류/거래 multi + 평수/가격 range + 정렬 select + i18n
- listings.ko.json: 6 type + 3 transaction + 5 sort + filter labels"
```

---

## Task 5: Listing Card + Card List — 무한 스크롤 + skeleton + 핀↔카드 highlight

**Files:**
- Create: `apps/web/components/listings/listing-card.tsx`
- Create: `apps/web/components/listings/listing-card-list.tsx`

- [ ] **Step 5.1: listing-card.tsx**

`apps/web/components/listings/listing-card.tsx`:

```typescript
"use client";
import { useTranslations } from "next-intl";
import Link from "next/link";
import { Card, CardContent } from "@gongzzang/ui";
import { Heart } from "lucide-react";
import type { ListingCard as ListingCardData } from "@/lib/listings/api";
import { formatAreaPyeong, formatPriceKrw } from "@/lib/listings/format";
import { getPinColor } from "@/lib/listings/pin-color";
import { useListingsStore } from "@/stores/listings";

interface ListingCardProps {
  data: ListingCardData;
}

export function ListingCard({ data }: ListingCardProps) {
  const t = useTranslations("listings");
  const selectedId = useListingsStore((s) => s.selectedListingId);
  const setSelected = useListingsStore((s) => s.setSelectedListingId);
  const isSelected = selectedId === data.id;

  return (
    <Card
      className={`overflow-hidden transition ${
        isSelected ? "ring-2 ring-primary" : "hover:bg-muted/50"
      }`}
      onMouseEnter={() => setSelected(data.id)}
      onMouseLeave={() => setSelected(null)}
    >
      <Link href={`/listings/${data.id}`} className="block">
        <div
          className="aspect-[4/3] w-full bg-muted"
          style={{
            backgroundColor: data.thumbnail_url ? undefined : `${getPinColor(data.listing_type)}22`,
          }}
        >
          {data.thumbnail_url ? (
            <img src={data.thumbnail_url} alt={data.title} className="h-full w-full object-cover" />
          ) : (
            <div className="flex h-full items-center justify-center text-muted-foreground text-sm">
              {t(`type.${data.listing_type}`)}
            </div>
          )}
        </div>
        <CardContent className="p-4">
          <div className="mb-2 flex items-center gap-2">
            <span
              className="rounded-full px-2 py-0.5 text-xs font-medium text-white"
              style={{ backgroundColor: getPinColor(data.listing_type) }}
            >
              {t(`type.${data.listing_type}`)}
            </span>
            <span className="text-xs text-muted-foreground">
              {t(`transaction.${data.transaction_type}`)}
            </span>
          </div>
          <h3 className="mb-1 line-clamp-1 text-base font-semibold">{data.title}</h3>
          <div className="mb-2 text-sm text-muted-foreground">
            {formatAreaPyeong(data.area_m2)}
          </div>
          <div className="text-lg font-bold">{formatPriceKrw(data.price_krw)}</div>
          <div className="mt-2 flex items-center gap-3 text-xs text-muted-foreground">
            <span aria-label={t("card.viewCount")}>👁 {data.view_count}</span>
            <button
              type="button"
              aria-label={t("card.favoritePlaceholder")}
              className="flex items-center gap-1 hover:text-primary"
              onClick={(e) => {
                e.preventDefault();
                // SP6-iii 가 즐겨찾기 toggle 구현
              }}
            >
              <Heart className="h-3 w-3" /> {data.bookmark_count}
            </button>
          </div>
        </CardContent>
      </Link>
    </Card>
  );
}
```

- [ ] **Step 5.2: listing-card-list.tsx (무한 스크롤)**

`apps/web/components/listings/listing-card-list.tsx`:

```typescript
"use client";
import { useEffect, useRef } from "react";
import { useInfiniteQuery } from "@tanstack/react-query";
import { useTranslations } from "next-intl";
import { ListingCard } from "@/components/listings/listing-card";
import { fetchListings, type ListingsResponse } from "@/lib/listings/api";
import { useListingsStore } from "@/stores/listings";

const PAGE_SIZE = 20;

export function ListingCardList() {
  const t = useTranslations("listings");
  const filters = useListingsStore((s) => s.filters);
  const bounds = useListingsStore((s) => s.bounds);

  const query = useInfiniteQuery<ListingsResponse>({
    queryKey: ["listings", filters, bounds],
    queryFn: ({ pageParam }) =>
      fetchListings({
        filters,
        bounds,
        page: pageParam as number,
        size: PAGE_SIZE,
      }),
    initialPageParam: 0,
    getNextPageParam: (last) => (last.has_next ? last.page + 1 : undefined),
    enabled: bounds !== undefined,
  });

  // 무한 스크롤 sentinel
  const sentinelRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!sentinelRef.current) return;
    const obs = new IntersectionObserver((entries) => {
      if (entries[0]?.isIntersecting && query.hasNextPage && !query.isFetchingNextPage) {
        query.fetchNextPage();
      }
    });
    obs.observe(sentinelRef.current);
    return () => obs.disconnect();
  }, [query]);

  if (query.isLoading) {
    return (
      <div className="flex flex-col gap-3 p-4">
        {Array.from({ length: 6 }).map((_, i) => (
          <div key={i} className="h-48 animate-pulse rounded-lg bg-muted" />
        ))}
      </div>
    );
  }

  if (query.isError) {
    return (
      <div className="p-8 text-center text-sm text-destructive">
        {t("errors.fetchFailed")}
      </div>
    );
  }

  const allListings = query.data?.pages.flatMap((p) => p.listings) ?? [];

  if (allListings.length === 0) {
    return (
      <div className="p-8 text-center text-sm text-muted-foreground">
        {t("empty")}
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-3 p-4">
      {allListings.map((listing) => (
        <ListingCard key={listing.id} data={listing} />
      ))}
      <div ref={sentinelRef} className="h-8" />
      {query.isFetchingNextPage && (
        <div className="text-center text-xs text-muted-foreground">{t("loading")}</div>
      )}
    </div>
  );
}
```

- [ ] **Step 5.3: typecheck**

```bash
pnpm --filter=@gongzzang/web typecheck
```

Expected: PASS.

- [ ] **Step 5.4: Commit**

```bash
git add apps/web/components/listings/listing-card.tsx apps/web/components/listings/listing-card-list.tsx
git commit -m "feat(6ii-T5): ListingCard + ListingCardList (무한 스크롤 + skeleton + 핀↔카드 highlight)

- listing-card: 사진 (또는 종류 placeholder) + type badge + 제목 + 평수 + 가격 + view/bookmark count + 즐겨찾기 자리 (SP6-iii) + hover → 핀 highlight
- listing-card-list: TanStack Query useInfiniteQuery + IntersectionObserver sentinel + skeleton (6 카드) + empty/error 상태 + i18n"
```

---

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

## Task 7: Pretendard self-host + dark mode + CSP cdn 제거

**Files:**
- Create: `apps/web/public/fonts/Pretendard-Regular.woff2`
- Create: `apps/web/public/fonts/Pretendard-Medium.woff2`
- Create: `apps/web/public/fonts/Pretendard-Bold.woff2`
- Create: `apps/web/public/fonts/Pretendard-Heavy.woff2`
- Modify: `apps/web/app/layout.tsx`
- Modify: `packages/ui/tokens/typography.css`
- Modify: `apps/web/proxy.ts` (CSP)

- [ ] **Step 7.1: Pretendard variable woff2 다운로드 (4 가중치)**

```bash
mkdir -p apps/web/public/fonts
cd apps/web/public/fonts
# Pretendard variable subset web font
curl -L -o Pretendard-Regular.woff2 https://github.com/orioncactus/pretendard/raw/v1.3.9/packages/pretendard/dist/web/static/woff2/Pretendard-Regular.woff2
curl -L -o Pretendard-Medium.woff2 https://github.com/orioncactus/pretendard/raw/v1.3.9/packages/pretendard/dist/web/static/woff2/Pretendard-Medium.woff2
curl -L -o Pretendard-Bold.woff2 https://github.com/orioncactus/pretendard/raw/v1.3.9/packages/pretendard/dist/web/static/woff2/Pretendard-Bold.woff2
curl -L -o Pretendard-ExtraBold.woff2 https://github.com/orioncactus/pretendard/raw/v1.3.9/packages/pretendard/dist/web/static/woff2/Pretendard-ExtraBold.woff2
ls -la
```

(NOTE: 4 file ≈ 800 KB 합. license = OFL 1.1 Pretendard 의 라이선스. README 의 attribution 추가 권장.)

- [ ] **Step 7.2: app/layout.tsx 의 next/font/local**

`apps/web/app/layout.tsx` 수정:

```typescript
import localFont from "next/font/local";

const pretendard = localFont({
  src: [
    { path: "../public/fonts/Pretendard-Regular.woff2", weight: "400", style: "normal" },
    { path: "../public/fonts/Pretendard-Medium.woff2", weight: "500", style: "normal" },
    { path: "../public/fonts/Pretendard-Bold.woff2", weight: "700", style: "normal" },
    { path: "../public/fonts/Pretendard-ExtraBold.woff2", weight: "800", style: "normal" },
  ],
  variable: "--font-pretendard",
  display: "swap",
});

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="ko" className={pretendard.variable}>
      <body className="font-sans">
        {/* ... existing providers ... */}
      </body>
    </html>
  );
}
```

(NOTE: 기존 layout.tsx 의 정확한 내용은 Read 후 정확히 변경. providers wrapper, NextIntlClientProvider 등 유지.)

- [ ] **Step 7.3: tokens/typography.css 정리**

`packages/ui/tokens/typography.css` 의 `@import url('https://cdn.jsdelivr.net/...')` 줄 제거. font-family 만 유지:

```css
:root {
  --font-sans: var(--font-pretendard), -apple-system, BlinkMacSystemFont, "Segoe UI",
    "Helvetica Neue", "Apple SD Gothic Neo", sans-serif;
}
```

`tailwind.config.ts` (또는 inline) 의 fontFamily.sans 가 `var(--font-sans)` 사용하도록 — 이미 그럴 수도 있음 (Read 로 확인).

- [ ] **Step 7.4: proxy.ts CSP 의 cdn.jsdelivr 제거**

`apps/web/proxy.ts` 의 CSP `style-src` 정리:

```typescript
const cspHeader = [
  `default-src 'self'`,
  `script-src 'self' 'nonce-${nonce}' 'strict-dynamic'`,
  `style-src 'self' 'unsafe-inline'`,           // cdn.jsdelivr 제거됨
  `img-src 'self' data: blob:`,
  `font-src 'self' data:`,                      // self-host 만 허용
  `connect-src 'self' ${env.NEXT_PUBLIC_API_BASE_URL} ${env.ZITADEL_ISSUER}`,
  `frame-ancestors 'none'`,
  `base-uri 'self'`,
  `form-action 'self' ${env.ZITADEL_ISSUER}`,
].join("; ");
```

(NOTE: 기존 cdn.jsdelivr.net allow 는 SP6-foundation 시점 자리. self-host 전환 후 삭제. — 단 기존 코드에 이미 추가됐는지 Read 로 확인. 없을 수도 있음.)

- [ ] **Step 7.5: 로컬 검증**

```bash
pnpm --filter=@gongzzang/web dev
# 브라우저: http://localhost:3000/listings
# DevTools → Network → fonts/* 의 200 + 자체 도메인 확인
# DevTools → Console 에 "Refused to execute" / "violates CSP" 경고 없어야
```

- [ ] **Step 7.6: bundle size**

```bash
pnpm --filter=@gongzzang/web test:bundle
```

Expected: under threshold (Pretendard 800 KB → next/font 가 subset 자동 적용 → 실제 < 200KB 추가).

- [ ] **Step 7.7: Commit**

```bash
git add apps/web/public/fonts/ apps/web/app/layout.tsx packages/ui/tokens/typography.css apps/web/proxy.ts
git commit -m "feat(6ii-T7): Pretendard self-host (next/font/local) + CSP cdn.jsdelivr 제거

- public/fonts/Pretendard-{Regular,Medium,Bold,ExtraBold}.woff2 (OFL 1.1, attribution in README)
- app/layout.tsx: localFont (4 weights, swap display, --font-pretendard variable)
- tokens/typography.css: cdn.jsdelivr import 제거, --font-sans = var(--font-pretendard) chain
- proxy.ts CSP: style-src 의 cdn.jsdelivr.net 제거 (self-host 전환 완료)"
```

---

