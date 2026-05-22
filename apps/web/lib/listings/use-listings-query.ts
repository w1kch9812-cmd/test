"use client";
import { useInfiniteQuery } from "@tanstack/react-query";
import { fetchListings, type ListingsResponse } from "@/lib/listings/api";
import { usePanelStack } from "@/lib/panel/use-panel-stack";
import { useListingsStore } from "@/stores/listings";

const PAGE_SIZE = 20;

/**
 * 단일 useInfiniteQuery hook. SP10: filters 의 pnu 자리를 panel stack 의 top
 * (parcel.summary 또는 parcel.*) 에서 derive — `useListingsStore` 에 pnu 없음.
 */
export function useListingsQuery() {
  const filters = useListingsStore((s) => s.filters);
  const { stack } = usePanelStack();

  const top = stack.entries[stack.entries.length - 1];
  const derivedPnu = top?.kind === "parcel" ? top.id : undefined;

  return useInfiniteQuery<ListingsResponse>({
    queryKey: ["listings", filters, derivedPnu],
    queryFn: ({ pageParam }) =>
      fetchListings({
        filters,
        pnu: derivedPnu,
        page: pageParam as number,
        size: PAGE_SIZE,
      }),
    initialPageParam: 0,
    getNextPageParam: (last) => (last.has_next ? last.page + 1 : undefined),
  });
}
