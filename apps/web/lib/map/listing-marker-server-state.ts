import type { ListingFilters, ListingType, TransactionType } from "@/lib/listings/filters";

export type ListingMarkerServerKeyInput = {
  filterHash: string;
  projectionVersion: number | undefined;
  anchorSnapshotId: string | undefined;
};

export type ListingMarkerFilterRequest = {
  types: ListingType[];
  transactions: TransactionType[];
  min_area_m2: number | undefined;
  max_area_m2: number | undefined;
  min_price_krw: number | undefined;
  max_price_krw: number | undefined;
};

export type ListingMarkerFilterRegistrationResponse = {
  filter_hash: string;
};

export type ListingMarkerCountResponse = {
  total_count: number;
  projection_version: number | null | undefined;
  anchor_snapshot_id: string | null | undefined;
};

export function buildListingMarkerServerKey(input: ListingMarkerServerKeyInput): string {
  return [
    "listing",
    input.filterHash,
    input.projectionVersion ?? "none",
    input.anchorSnapshotId ?? "none",
  ].join("|");
}

export function buildListingMarkerFilterRequest(
  filters: ListingFilters,
): ListingMarkerFilterRequest {
  return {
    types: [...filters.types],
    transactions: [...filters.transactions],
    min_area_m2: filters.minAreaM2,
    max_area_m2: filters.maxAreaM2,
    min_price_krw: filters.minPriceKrw,
    max_price_krw: filters.maxPriceKrw,
  };
}
