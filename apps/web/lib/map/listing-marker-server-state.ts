import type { ListingFilters, ListingType, TransactionType } from "@/lib/listings/filters";
import { API } from "@/lib/routes";

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

export type ListingMarkerServerState = {
  filterHash: string;
  totalCount: number | undefined;
  projectionVersion: number | undefined;
  anchorSnapshotId: string | undefined;
  requestKey: string;
};

type Fetcher = typeof fetch;

type LoadListingMarkerServerStateOptions = {
  fetcher?: Fetcher;
  origin?: string;
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

export function buildListingMarkerCountUrl(filterHash: string, origin: string): string {
  const countUrl = new URL(API.proxy.listingMarkerCounts, origin);
  countUrl.searchParams.set("filter_hash", filterHash);
  return countUrl.toString();
}

export async function loadListingMarkerServerState(
  filters: ListingFilters,
  signal: AbortSignal,
  options: LoadListingMarkerServerStateOptions = {},
): Promise<ListingMarkerServerState> {
  const fetcher = options.fetcher ?? fetch;
  const origin = options.origin ?? window.location.origin;
  const registered = await registerListingMarkerFilter(filters, signal, fetcher);
  const count = await fetchListingMarkerCount(registered.filter_hash, signal, fetcher, origin);
  return toListingMarkerServerState(registered.filter_hash, count);
}

async function registerListingMarkerFilter(
  filters: ListingFilters,
  signal: AbortSignal,
  fetcher: Fetcher,
): Promise<ListingMarkerFilterRegistrationResponse> {
  const response = await fetcher(API.proxy.listingMarkerFilters, {
    method: "POST",
    headers: {
      accept: "application/json",
      "content-type": "application/json",
    },
    body: JSON.stringify(buildListingMarkerFilterRequest(filters)),
    signal,
  });
  if (!response.ok) {
    throw new Error(`listing marker filter registration failed: ${response.status}`);
  }
  return (await response.json()) as ListingMarkerFilterRegistrationResponse;
}

async function fetchListingMarkerCount(
  filterHash: string,
  signal: AbortSignal,
  fetcher: Fetcher,
  origin: string,
): Promise<ListingMarkerCountResponse> {
  const response = await fetcher(buildListingMarkerCountUrl(filterHash, origin), {
    headers: { accept: "application/json" },
    signal,
  });
  if (!response.ok) {
    throw new Error(`listing marker count fetch failed: ${response.status}`);
  }
  return (await response.json()) as ListingMarkerCountResponse;
}

function toListingMarkerServerState(
  filterHash: string,
  count: ListingMarkerCountResponse,
): ListingMarkerServerState {
  const projectionVersion = count.projection_version ?? undefined;
  const anchorSnapshotId = count.anchor_snapshot_id ?? undefined;
  return {
    filterHash,
    totalCount: count.total_count,
    projectionVersion,
    anchorSnapshotId,
    requestKey: buildListingMarkerServerKey({
      filterHash,
      projectionVersion,
      anchorSnapshotId,
    }),
  };
}
