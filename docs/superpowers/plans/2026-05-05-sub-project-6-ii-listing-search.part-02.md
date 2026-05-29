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

## Task 3: Naver Maps 통합 — loader + ListingMap + 핀 + 클러스터 + bounds 이벤트

**Files:**
- Modify: `apps/web/lib/env.ts` (NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID 추가)
- Modify: `apps/web/.env.local.example`
- Modify: `turbo.json` (globalEnv)
- Modify: `apps/web/package.json` (`@types/navermaps`)
- Create: `apps/web/lib/naver-maps.ts`
- Create: `apps/web/components/listings/listing-pin.tsx`
- Create: `apps/web/components/listings/listing-map.tsx`

- [ ] **Step 3.1: env.ts 확장 (zod)**

`apps/web/lib/env.ts` 의 `PublicEnvSchema` 에 `NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID` 추가:

```typescript
const PublicEnvSchema = z.object({
  NEXT_PUBLIC_API_BASE_URL: z.string().url().default("http://localhost:8080"),
  NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID: z.string().min(1).default("naver-maps-placeholder"),
});
```

(default 둠 — placeholder 면 지도 안 뜨지만 build 통과.)

`safeParse` 호출 시 객체에도 추가:

```typescript
const parsed = Schema.safeParse({
  NEXT_PUBLIC_API_BASE_URL: process.env.NEXT_PUBLIC_API_BASE_URL,
  NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID: process.env.NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID,
  // ... 기존 server env
});
```

- [ ] **Step 3.2: .env.local.example + turbo.json**

`apps/web/.env.local.example` 끝에:

```
# SP6-ii — Naver Maps
NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID=naver-maps-placeholder
```

`turbo.json` 의 `globalEnv` 끝에:

```json
"NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID",
```

- [ ] **Step 3.3: deps 추가**

```bash
pnpm --filter=@gongzzang/web add -D @types/navermaps@^3.7.0
```

- [ ] **Step 3.4: lib/naver-maps.ts (lazy script loader)**

`apps/web/lib/naver-maps.ts`:

```typescript
import { env } from "@/lib/env";

let _readyPromise: Promise<typeof naver> | null = null;

/**
 * Naver Maps SDK script lazy load. 한 번만 로드.
 *
 * `naver` 글로벌이 ready 되면 resolve.
 */
export function loadNaverMaps(): Promise<typeof naver> {
  if (_readyPromise) return _readyPromise;
  if (typeof window === "undefined") {
    return Promise.reject(new Error("loadNaverMaps must run in browser"));
  }
  _readyPromise = new Promise((resolve, reject) => {
    if (typeof naver !== "undefined" && naver.maps) {
      resolve(naver);
      return;
    }
    const script = document.createElement("script");
    script.src = `https://oapi.map.naver.com/openapi/v3/maps.js?ncpClientId=${env.NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID}&submodules=clustering`;
    script.async = true;
    script.onload = () => resolve(naver);
    script.onerror = () => reject(new Error("Naver Maps SDK failed to load"));
    document.head.appendChild(script);
  });
  return _readyPromise;
}
```

- [ ] **Step 3.5: components/listings/listing-pin.tsx (SVG marker template)**

`apps/web/components/listings/listing-pin.tsx`:

```typescript
import { getPinColor, type ListingTypeKey } from "@/lib/listings/pin-color";

/**
 * Naver Maps 의 marker icon 으로 사용할 SVG HTML string.
 * `new naver.maps.Marker({ icon: { content: pinIconHtml(...) } })`.
 */
export function pinIconHtml(listingType: string, options: { selected?: boolean } = {}): string {
  const color = getPinColor(listingType);
  const size = options.selected ? 36 : 28;
  const stroke = options.selected ? "#ffffff" : "#1f2937";
  const strokeWidth = options.selected ? 3 : 1.5;
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 24 24" fill="${color}" stroke="${stroke}" stroke-width="${strokeWidth}">
    <path d="M12 2C7.58 2 4 5.58 4 10c0 5.25 7 12 8 12s8-6.75 8-12c0-4.42-3.58-8-8-8z"/>
    <circle cx="12" cy="10" r="3" fill="#ffffff"/>
  </svg>`;
}
```

- [ ] **Step 3.6: components/listings/listing-map.tsx**

`apps/web/components/listings/listing-map.tsx`:

```typescript
"use client";
import { useEffect, useRef } from "react";
import { loadNaverMaps } from "@/lib/naver-maps";
import { useListingsStore } from "@/stores/listings";
import type { ListingCard } from "@/lib/listings/api";
import { pinIconHtml } from "@/components/listings/listing-pin";

interface ListingMapProps {
  listings: ListingCard[];
}

export function ListingMap({ listings }: ListingMapProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const mapRef = useRef<naver.maps.Map | null>(null);
  const markersRef = useRef<naver.maps.Marker[]>([]);
  const setBounds = useListingsStore((s) => s.setBounds);
  const selectedId = useListingsStore((s) => s.selectedListingId);
  const setSelected = useListingsStore((s) => s.setSelectedListingId);

  // 1. 지도 초기화 (1회)
  useEffect(() => {
    let cancelled = false;
    loadNaverMaps().then((naverNs) => {
      if (cancelled || !containerRef.current) return;
      const map = new naverNs.maps.Map(containerRef.current, {
        center: new naverNs.maps.LatLng(37.5665, 126.978),  // 서울 시청
        zoom: 8,
        mapTypeControl: false,
      });
      mapRef.current = map;

      // bounds 변경 이벤트 (debounce)
      let timer: ReturnType<typeof setTimeout> | null = null;
      naverNs.maps.Event.addListener(map, "bounds_changed", () => {
        if (timer) clearTimeout(timer);
        timer = setTimeout(() => {
          const bounds = map.getBounds() as naver.maps.LatLngBounds;
          const sw = bounds.getMin();
          const ne = bounds.getMax();
          setBounds({
            south: sw.y,
            west: sw.x,
            north: ne.y,
            east: ne.x,
          });
        }, 350);
      });
      // 초기 bounds 도 emit
      const b = map.getBounds() as naver.maps.LatLngBounds;
      setBounds({
        south: b.getMin().y,
        west: b.getMin().x,
        north: b.getMax().y,
        east: b.getMax().x,
      });
    });
    return () => {
      cancelled = true;
    };
  }, [setBounds]);

  // 2. 매물 변경 → marker 재생성
  useEffect(() => {
    if (!mapRef.current) return;
    const map = mapRef.current;
    // 기존 marker 제거
    for (const m of markersRef.current) m.setMap(null);
    markersRef.current = [];

    // 새 marker 생성
    for (const listing of listings) {
      const marker = new naver.maps.Marker({
        position: new naver.maps.LatLng(listing.lat, listing.lng),
        map,
        icon: {
          content: pinIconHtml(listing.listing_type, { selected: listing.id === selectedId }),
          anchor: new naver.maps.Point(14, 28),
        },
      });
      naver.maps.Event.addListener(marker, "click", () => {
        setSelected(listing.id);
      });
      markersRef.current.push(marker);
    }
  }, [listings, selectedId, setSelected]);

  return <div ref={containerRef} className="h-full w-full" />;
}
```

- [ ] **Step 3.7: typecheck**

```bash
pnpm --filter=@gongzzang/web typecheck
```

Expected: PASS (단 `@types/navermaps` 의 정확한 type 시그니처에 따라 일부 cast 필요할 수 있음 — 발견 시 inline `as` 또는 `// eslint-disable` 대신 type narrowing 으로 해결).

- [ ] **Step 3.8: Commit**

```bash
git add apps/web/lib/naver-maps.ts apps/web/lib/env.ts apps/web/.env.local.example apps/web/components/listings/listing-pin.tsx apps/web/components/listings/listing-map.tsx apps/web/package.json apps/web/pnpm-lock.yaml turbo.json
git commit -m "feat(6ii-T3): Naver Maps 통합 — lazy SDK loader + ListingMap + SVG pin

- lib/env.ts: NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID (zod public schema)
- lib/naver-maps.ts: lazy script loader (한 번만 inject + ready Promise)
- components/listings/listing-pin.tsx: SVG marker template (6 종 unique color + selected highlight)
- components/listings/listing-map.tsx: Naver Maps + 마커 + bounds_changed debounced (350ms) → store + click → selected
- @types/navermaps 추가
- turbo.json globalEnv 추가"
```

---

