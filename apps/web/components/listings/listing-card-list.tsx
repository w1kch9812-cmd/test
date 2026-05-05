"use client";
import { useInfiniteQuery } from "@tanstack/react-query";
import { useTranslations } from "next-intl";
import { useEffect, useRef } from "react";
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

  const sentinelRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!sentinelRef.current) return;
    const obs = new IntersectionObserver((entries) => {
      if (entries[0]?.isIntersecting && query.hasNextPage && !query.isFetchingNextPage) {
        void query.fetchNextPage();
      }
    });
    obs.observe(sentinelRef.current);
    return () => obs.disconnect();
  }, [query]);

  if (query.isLoading) {
    return (
      <div className="flex flex-col gap-3 p-4">
        {(["sk-0", "sk-1", "sk-2", "sk-3", "sk-4", "sk-5"] as const).map((k) => (
          <div
            key={k}
            className="h-48 animate-pulse rounded-lg"
            style={{ background: "var(--color-muted)" }}
          />
        ))}
      </div>
    );
  }

  if (query.isError) {
    return (
      <div className="p-8 text-center text-sm" style={{ color: "var(--color-destructive)" }}>
        {t("errors.fetchFailed")}
      </div>
    );
  }

  const allListings = query.data?.pages.flatMap((p) => p.listings) ?? [];

  if (allListings.length === 0) {
    return (
      <div className="p-8 text-center text-sm" style={{ color: "var(--color-muted-fg)" }}>
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
        <div className="text-center text-xs" style={{ color: "var(--color-muted-fg)" }}>
          {t("loading")}
        </div>
      )}
    </div>
  );
}
