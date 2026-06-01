# Sub-project 6-ii Listing Search - Part 03B: Listing Card and List

Parent index: [Sub-project 6-ii Listing Search - Part 03](./2026-05-05-sub-project-6-ii-listing-search.part-03.md).
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
