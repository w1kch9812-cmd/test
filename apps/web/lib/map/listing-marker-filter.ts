import type { ListingFilters } from "@/lib/listings/filters";

type MapboxFilterExpression = unknown[];

export function buildListingMarkerLayerFilter(
  filters: ListingFilters,
  tombstoneIds: Iterable<string> = [],
): MapboxFilterExpression {
  const clauses: MapboxFilterExpression = ["all"];
  const hiddenMarkerIds = [...tombstoneIds];

  if (hiddenMarkerIds.length > 0) {
    clauses.push(["!", ["in", ["get", "id"], ["literal", hiddenMarkerIds]]]);
  }

  if (filters.types.length > 0) {
    clauses.push(["in", ["get", "listing_type"], ["literal", filters.types]]);
  }
  if (filters.transactions.length > 0) {
    clauses.push(["in", ["get", "transaction_type"], ["literal", filters.transactions]]);
  }
  if (filters.minAreaM2 !== undefined) {
    clauses.push([">=", ["to-number", ["get", "area_m2"]], filters.minAreaM2]);
  }
  if (filters.maxAreaM2 !== undefined) {
    clauses.push(["<=", ["to-number", ["get", "area_m2"]], filters.maxAreaM2]);
  }
  if (filters.minPriceKrw !== undefined) {
    clauses.push([">=", ["to-number", ["get", "price_krw"]], filters.minPriceKrw]);
  }
  if (filters.maxPriceKrw !== undefined) {
    clauses.push(["<=", ["to-number", ["get", "price_krw"]], filters.maxPriceKrw]);
  }

  return clauses;
}
