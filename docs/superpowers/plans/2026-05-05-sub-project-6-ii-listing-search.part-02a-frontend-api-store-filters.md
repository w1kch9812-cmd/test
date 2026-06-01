# Sub-project 6-ii Listing Search - Part 02A: Frontend API, Store, Filters, and Format Helpers

Parent index: [Sub-project 6-ii Listing Search - Part 02](./2026-05-05-sub-project-6-ii-listing-search.part-02.md).
## Task 2: Frontend api.ts + Zustand store + filters URL 동기화 + format helpers

**Files:**
- Create: `apps/web/lib/listings/api.ts`
- Create: `apps/web/lib/listings/filters.ts`
- Create: `apps/web/lib/listings/format.ts`
- Create: `apps/web/lib/listings/pin-color.ts`
- Create: `apps/web/stores/listings.ts`
- Test: `apps/web/tests/unit/listings/format.test.ts`
- Test: `apps/web/tests/unit/listings/filters.test.ts`
- Test: `apps/web/tests/unit/listings/pin-color.test.ts`

- [ ] **Step 2.1: format.ts — failing tests**

`apps/web/tests/unit/listings/format.test.ts`:

```typescript
// @vitest-environment node
import { describe, it, expect } from "vitest";
import { formatPriceKrw, formatAreaPyeong, formatAreaM2, m2ToPyeong } from "@/lib/listings/format";

describe("formatPriceKrw — 한국 가격 표기", () => {
  it("1조 이상", () => {
    expect(formatPriceKrw(1_500_000_000_000)).toBe("1조 5,000억원");
  });
  it("억 + 만원", () => {
    expect(formatPriceKrw(8_500_000_000)).toBe("85억원");
    expect(formatPriceKrw(123_450_000)).toBe("1억 2,345만원");
  });
  it("만원 단위", () => {
    expect(formatPriceKrw(50_000_000)).toBe("5,000만원");
  });
  it("원 단위", () => {
    expect(formatPriceKrw(800_000)).toBe("800,000원");
  });
  it("0", () => {
    expect(formatPriceKrw(0)).toBe("0원");
  });
});

describe("m2ToPyeong + formatAreaPyeong", () => {
  it("1평 = 3.305 m²", () => {
    expect(m2ToPyeong(3.305)).toBeCloseTo(1.0, 1);
  });
  it("formatAreaPyeong 소수점 1자리", () => {
    expect(formatAreaPyeong(330.5)).toBe("100.0평");
    expect(formatAreaPyeong(33.05)).toBe("10.0평");
  });
});

describe("formatAreaM2", () => {
  it("정수 + 천단위 콤마", () => {
    expect(formatAreaM2(3960.5)).toBe("3,961㎡");
  });
});
```

- [ ] **Step 2.2: Run test — FAIL**

```bash
pnpm --filter=@gongzzang/web test -- tests/unit/listings/format.test.ts
```

Expected: FAIL — module not found.

- [ ] **Step 2.3: format.ts 구현**

`apps/web/lib/listings/format.ts`:

```typescript
const TRILLION = 1_000_000_000_000n;
const HUNDRED_MILLION = 100_000_000n;
const TEN_THOUSAND = 10_000n;
const PYEONG_PER_M2 = 0.3025; // 1 평 = 3.305 m² → 1 m² ≈ 0.3025 평

/**
 * 한국 가격 표기 (1조 5,000억원 / 85억원 / 1억 2,345만원 / 5,000만원 / 800,000원).
 */
export function formatPriceKrw(value: number): string {
  if (value === 0) return "0원";
  const big = BigInt(Math.round(value));
  const trillions = big / TRILLION;
  const remainderAfterTrillions = big % TRILLION;
  const hundredMillions = remainderAfterTrillions / HUNDRED_MILLION;
  const remainderAfterHM = remainderAfterTrillions % HUNDRED_MILLION;
  const tenThousands = remainderAfterHM / TEN_THOUSAND;

  const parts: string[] = [];
  if (trillions > 0n) parts.push(`${trillions}조`);
  if (hundredMillions > 0n) {
    if (trillions > 0n) {
      parts.push(`${formatThousands(hundredMillions)}억원`);
      return parts.join(" ");
    }
    parts.push(`${formatThousands(hundredMillions)}억`);
    if (tenThousands > 0n) parts.push(`${formatThousands(tenThousands)}만원`);
    else parts[parts.length - 1] = `${parts[parts.length - 1]}원`;
    return parts.join(" ");
  }
  if (tenThousands > 0n) return `${formatThousands(tenThousands)}만원`;
  return `${formatThousands(big)}원`;
}

function formatThousands(n: bigint): string {
  return n.toLocaleString("ko-KR");
}

/** m² → 평 변환. */
export function m2ToPyeong(m2: number): number {
  return m2 * PYEONG_PER_M2;
}

/** "100.0평" 형식. */
export function formatAreaPyeong(m2: number): string {
  return `${m2ToPyeong(m2).toFixed(1)}평`;
}

/** "3,961㎡" 형식 (정수 + 천단위 콤마). */
export function formatAreaM2(m2: number): string {
  return `${Math.round(m2).toLocaleString("ko-KR")}㎡`;
}
```

- [ ] **Step 2.4: Run test — PASS**

```bash
pnpm --filter=@gongzzang/web test -- tests/unit/listings/format.test.ts
```

Expected: PASS (5+ tests).

- [ ] **Step 2.5: pin-color.ts — test + impl**

`apps/web/tests/unit/listings/pin-color.test.ts`:

```typescript
import { describe, it, expect } from "vitest";
import { getPinColor, LISTING_TYPE_COLORS } from "@/lib/listings/pin-color";

describe("getPinColor", () => {
  it("6 종 매물 모두 hex color 반환", () => {
    expect(getPinColor("factory")).toMatch(/^#[0-9a-f]{6}$/i);
    expect(getPinColor("warehouse")).toMatch(/^#[0-9a-f]{6}$/i);
    expect(getPinColor("office")).toMatch(/^#[0-9a-f]{6}$/i);
    expect(getPinColor("knowledge_industry_center")).toMatch(/^#[0-9a-f]{6}$/i);
    expect(getPinColor("industrial_land")).toMatch(/^#[0-9a-f]{6}$/i);
    expect(getPinColor("logistics_center")).toMatch(/^#[0-9a-f]{6}$/i);
  });
  it("6 종 모두 unique color", () => {
    const colors = new Set(Object.values(LISTING_TYPE_COLORS));
    expect(colors.size).toBe(6);
  });
});
```

`apps/web/lib/listings/pin-color.ts`:

```typescript
export const LISTING_TYPE_COLORS = {
  factory: "#dc2626",                    // red-600 (공장)
  warehouse: "#2563eb",                  // blue-600 (창고)
  office: "#059669",                     // emerald-600 (사무실)
  knowledge_industry_center: "#7c3aed",  // violet-600 (지식산업센터)
  industrial_land: "#ea580c",            // orange-600 (산업단지/토지)
  logistics_center: "#0891b2",           // cyan-600 (물류센터)
} as const;

export type ListingTypeKey = keyof typeof LISTING_TYPE_COLORS;

export function getPinColor(listingType: string): string {
  return LISTING_TYPE_COLORS[listingType as ListingTypeKey] ?? "#6b7280"; // gray-500 fallback
}
```

- [ ] **Step 2.6: filters.ts — test + impl**

`apps/web/tests/unit/listings/filters.test.ts`:

```typescript
// @vitest-environment node
import { describe, it, expect } from "vitest";
import {
  parseFiltersFromSearchParams,
  toSearchParams,
  type ListingFilters,
} from "@/lib/listings/filters";

describe("parseFiltersFromSearchParams", () => {
  it("default filter (모두 빈 값)", () => {
    const f = parseFiltersFromSearchParams(new URLSearchParams());
    expect(f.types).toEqual([]);
    expect(f.transactions).toEqual([]);
    expect(f.minAreaM2).toBeUndefined();
    expect(f.sort).toBe("created_at_desc");
  });
  it("comma-separated types", () => {
    const f = parseFiltersFromSearchParams(new URLSearchParams("types=factory,warehouse"));
    expect(f.types).toEqual(["factory", "warehouse"]);
  });
  it("range parsing", () => {
    const f = parseFiltersFromSearchParams(
      new URLSearchParams("min_area_m2=100&max_area_m2=2000&min_price_krw=0&max_price_krw=5000000000"),
    );
    expect(f.minAreaM2).toBe(100);
    expect(f.maxAreaM2).toBe(2000);
    expect(f.minPriceKrw).toBe(0);
    expect(f.maxPriceKrw).toBe(5_000_000_000);
  });
});

describe("toSearchParams (round trip)", () => {
  it("filter → URLSearchParams → 동일 filter", () => {
    const f: ListingFilters = {
      types: ["factory", "office"],
      transactions: ["sale"],
      minAreaM2: 200,
      maxAreaM2: undefined,
      minPriceKrw: undefined,
      maxPriceKrw: undefined,
      sort: "price_asc",
    };
    const sp = toSearchParams(f);
    const back = parseFiltersFromSearchParams(sp);
    expect(back.types).toEqual(f.types);
    expect(back.transactions).toEqual(f.transactions);
    expect(back.minAreaM2).toBe(200);
    expect(back.sort).toBe("price_asc");
  });
});
```

`apps/web/lib/listings/filters.ts`:

```typescript
export type ListingType =
  | "factory"
  | "warehouse"
  | "office"
  | "knowledge_industry_center"
  | "industrial_land"
  | "logistics_center";

export type TransactionType = "sale" | "monthly_rent" | "jeonse";

export type SortKey =
  | "created_at_desc"
  | "price_asc"
  | "price_desc"
  | "area_asc"
  | "area_desc";

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
```

- [ ] **Step 2.7: Run filter tests — PASS**

```bash
pnpm --filter=@gongzzang/web test -- tests/unit/listings/
```

Expected: PASS.

- [ ] **Step 2.8: api.ts — zod schema + ky 호출**

`apps/web/lib/listings/api.ts`:

```typescript
import { z } from "zod";
import { api } from "@/lib/api";
import type { ListingFilters } from "@/lib/listings/filters";
import { toSearchParams } from "@/lib/listings/filters";

export const ListingCardSchema = z.object({
  id: z.string(),
  title: z.string(),
  listing_type: z.enum([
    "factory",
    "warehouse",
    "office",
    "knowledge_industry_center",
    "industrial_land",
    "logistics_center",
  ]),
  transaction_type: z.enum(["sale", "monthly_rent", "jeonse"]),
  price_krw: z.number().int(),
  deposit_krw: z.number().int().nullable(),
  monthly_rent_krw: z.number().int().nullable(),
  area_m2: z.number(),
  lat: z.number(),
  lng: z.number(),
  thumbnail_url: z.string().nullable(),
  view_count: z.number().int(),
  bookmark_count: z.number().int(),
  created_at: z.string(), // ISO 8601
});

export type ListingCard = z.infer<typeof ListingCardSchema>;

export const ListingsResponseSchema = z.object({
  listings: z.array(ListingCardSchema),
  total: z.number().int(),
  page: z.number().int(),
  size: z.number().int(),
  has_next: z.boolean(),
});

export type ListingsResponse = z.infer<typeof ListingsResponseSchema>;

export interface FetchListingsInput {
  filters: ListingFilters;
  bounds?: { south: number; west: number; north: number; east: number };
  page?: number;
  size?: number;
}

export async function fetchListings(input: FetchListingsInput): Promise<ListingsResponse> {
  const sp = toSearchParams(input.filters);
  if (input.bounds) {
    const { south, west, north, east } = input.bounds;
    sp.set("bounds", `${south},${west},${north},${east}`);
  }
  if (input.page !== undefined) sp.set("page", String(input.page));
  if (input.size !== undefined) sp.set("size", String(input.size));

  const json = await api.get(`listings?${sp.toString()}`).json<unknown>();
  return ListingsResponseSchema.parse(json);
}
```

- [ ] **Step 2.9: stores/listings.ts (Zustand)**

`apps/web/stores/listings.ts`:

```typescript
"use client";
import { create } from "zustand";
import type { ListingFilters, SortKey } from "@/lib/listings/filters";

export interface MapBounds {
  south: number;
  west: number;
  north: number;
  east: number;
}

interface ListingsState {
  bounds: MapBounds | undefined;
  filters: ListingFilters;
  selectedListingId: string | null;
  setBounds: (b: MapBounds) => void;
  setFilters: (next: ListingFilters) => void;
  patchFilters: (patch: Partial<ListingFilters>) => void;
  setSelectedListingId: (id: string | null) => void;
}

const DEFAULT_FILTERS: ListingFilters = {
  types: [],
  transactions: [],
  minAreaM2: undefined,
  maxAreaM2: undefined,
  minPriceKrw: undefined,
  maxPriceKrw: undefined,
  sort: "created_at_desc" as SortKey,
};

export const useListingsStore = create<ListingsState>((set) => ({
  bounds: undefined,
  filters: DEFAULT_FILTERS,
  selectedListingId: null,
  setBounds: (b) => set({ bounds: b }),
  setFilters: (next) => set({ filters: next }),
  patchFilters: (patch) =>
    set((state) => ({ filters: { ...state.filters, ...patch } })),
  setSelectedListingId: (id) => set({ selectedListingId: id }),
}));
```

- [ ] **Step 2.10: typecheck + lint + commit**

```bash
pnpm typecheck
pnpm lint
git add apps/web/lib/listings/ apps/web/stores/listings.ts apps/web/tests/unit/listings/
git commit -m "feat(6ii-T2): listings api.ts + zod + filters + format + Zustand store

- lib/listings/api.ts: ky + zod (ListingsResponseSchema 검증)
- lib/listings/filters.ts: URL query parse/serialize + 6 ListingType + 3 TransactionType + 5 SortKey
- lib/listings/format.ts: formatPriceKrw (1조 5,000억원) + formatAreaPyeong (m² → 평) + formatAreaM2
- lib/listings/pin-color.ts: 6 매물 종류 → unique hex color (red/blue/emerald/violet/orange/cyan)
- stores/listings.ts: Zustand { bounds, filters, selectedListingId } + setters/patchers
- 14 unit test (format 5 + pin-color 2 + filters 4)"
```

---
