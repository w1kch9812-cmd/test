"use client";
import { useInfiniteQuery } from "@tanstack/react-query";
import { fetchListings, type ListingsResponse } from "@/lib/listings/api";
import { useListingsStore } from "@/stores/listings";

const PAGE_SIZE = 20;

/**
 * 단일 useInfiniteQuery hook — ListingMap + ListingCardList 가 동일 queryKey 로 공유.
 * filters / bounds 변경 시 자동 refetch (Zustand 의 patchFilters 가 새 object ref 생성).
 */
export function useListingsQuery() {
  const filters = useListingsStore((s) => s.filters);
  const bounds = useListingsStore((s) => s.bounds);

  return useInfiniteQuery<ListingsResponse>({
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
}
