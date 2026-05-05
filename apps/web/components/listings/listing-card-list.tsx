"use client";
import { Skeleton } from "@gongzzang/ui";
import { useTranslations } from "next-intl";
import { useEffect, useRef } from "react";
import { ListingCard } from "@/components/listings/listing-card";
import { useListingsQuery } from "@/lib/listings/use-listings-query";

const SKELETON_KEYS = ["sk-0", "sk-1", "sk-2", "sk-3", "sk-4", "sk-5"] as const;

export function ListingCardList() {
  const t = useTranslations("listings");
  const query = useListingsQuery();

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
      <div className="flex flex-col gap-4 p-5">
        {SKELETON_KEYS.map((k) => (
          <Skeleton key={k} className="h-72 w-full" />
        ))}
      </div>
    );
  }

  if (query.isError) {
    return (
      <div className="p-8 text-center text-[length:var(--text-body-sm)] text-[var(--color-error)]">
        {t("errors.fetchFailed")}
      </div>
    );
  }

  const allListings = query.data?.pages.flatMap((p) => p.listings) ?? [];

  if (allListings.length === 0) {
    return (
      <div className="p-8 text-center text-[length:var(--text-body-sm)] text-[var(--color-muted)]">
        {t("empty")}
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-4 p-5">
      {allListings.map((listing) => (
        <ListingCard key={listing.id} data={listing} />
      ))}
      <div ref={sentinelRef} className="h-8" />
      {query.isFetchingNextPage && (
        <div className="text-center text-[length:var(--text-caption)] text-[var(--color-muted)]">
          {t("loading")}
        </div>
      )}
    </div>
  );
}
