export type ListingType =
  | "factory"
  | "warehouse"
  | "office"
  | "knowledge_industry_center"
  | "industrial_land"
  | "logistics_center";

export type TransactionType = "sale" | "monthly_rent" | "jeonse";

export type SortKey = "created_at_desc" | "price_asc" | "price_desc" | "area_asc" | "area_desc";

export interface ListingFilters {
  types: ListingType[];
  transactions: TransactionType[];
  minAreaM2: number | undefined;
  maxAreaM2: number | undefined;
  minPriceKrw: number | undefined;
  maxPriceKrw: number | undefined;
  sort: SortKey;
}

const VALID_TYPES: ListingType[] = [
  "factory",
  "warehouse",
  "office",
  "knowledge_industry_center",
  "industrial_land",
  "logistics_center",
];

const VALID_TXNS: TransactionType[] = ["sale", "monthly_rent", "jeonse"];

const VALID_SORTS: SortKey[] = [
  "created_at_desc",
  "price_asc",
  "price_desc",
  "area_asc",
  "area_desc",
];

function parseList<T extends string>(raw: string | null, valid: readonly T[]): T[] {
  if (!raw) return [];
  return raw
    .split(",")
    .map((s) => s.trim())
    .filter((s): s is T => valid.includes(s as T));
}

function parseNumber(raw: string | null): number | undefined {
  if (raw === null || raw === "") return undefined;
  const n = Number(raw);
  return Number.isFinite(n) ? n : undefined;
}

export function parseFiltersFromSearchParams(sp: URLSearchParams): ListingFilters {
  const sortRaw = sp.get("sort");
  const sort: SortKey = VALID_SORTS.includes(sortRaw as SortKey)
    ? (sortRaw as SortKey)
    : "created_at_desc";

  return {
    types: parseList(sp.get("types"), VALID_TYPES),
    transactions: parseList(sp.get("transaction"), VALID_TXNS),
    minAreaM2: parseNumber(sp.get("min_area_m2")),
    maxAreaM2: parseNumber(sp.get("max_area_m2")),
    minPriceKrw: parseNumber(sp.get("min_price_krw")),
    maxPriceKrw: parseNumber(sp.get("max_price_krw")),
    sort,
  };
}

export function toSearchParams(f: ListingFilters): URLSearchParams {
  const sp = new URLSearchParams();
  if (f.types.length > 0) sp.set("types", f.types.join(","));
  if (f.transactions.length > 0) sp.set("transaction", f.transactions.join(","));
  if (f.minAreaM2 !== undefined) sp.set("min_area_m2", String(f.minAreaM2));
  if (f.maxAreaM2 !== undefined) sp.set("max_area_m2", String(f.maxAreaM2));
  if (f.minPriceKrw !== undefined) sp.set("min_price_krw", String(f.minPriceKrw));
  if (f.maxPriceKrw !== undefined) sp.set("max_price_krw", String(f.maxPriceKrw));
  if (f.sort !== "created_at_desc") sp.set("sort", f.sort);
  return sp;
}
