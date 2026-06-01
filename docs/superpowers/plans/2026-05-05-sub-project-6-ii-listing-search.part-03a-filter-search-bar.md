# Sub-project 6-ii Listing Search - Part 03A: Filter and Search Bar

Parent index: [Sub-project 6-ii Listing Search - Part 03](./2026-05-05-sub-project-6-ii-listing-search.part-03.md).
## Task 4: Filter / Search bar — 종류 + 거래 + 평수 + 가격 + URL query 동기화

**Files:**
- Create: `packages/ui/primitives/range-slider.tsx`
- Create: `packages/ui/primitives/multi-select.tsx`
- Modify: `packages/ui/index.ts` (export)
- Create: `apps/web/components/listings/search-bar.tsx`
- Create: `apps/web/components/listings/filter-bar.tsx`
- Test: `apps/web/tests/unit/listings/filter-bar.test.tsx`

(주: shadcn/Radix 의 Slider primitive 와 Combobox 패턴 사용. 이미 packages/ui 에 Radix 가 있으므로 추가 dep 불필요 — Radix Slider 직접 사용.)

- [ ] **Step 4.1: Radix Slider 추가 + range-slider primitive**

```bash
pnpm --filter=@gongzzang/ui add @radix-ui/react-slider@^1.2.3
```

`packages/ui/primitives/range-slider.tsx`:

```typescript
"use client";
import * as SliderPrimitive from "@radix-ui/react-slider";
import { cn } from "../lib/cn";

interface RangeSliderProps {
  min: number;
  max: number;
  step?: number;
  value: [number, number];
  onValueChange: (next: [number, number]) => void;
  formatValue?: (v: number) => string;
  className?: string;
}

export function RangeSlider({
  min, max, step = 1, value, onValueChange, formatValue, className,
}: RangeSliderProps) {
  return (
    <div className={cn("flex flex-col gap-2", className)}>
      <SliderPrimitive.Root
        min={min}
        max={max}
        step={step}
        value={value}
        onValueChange={(v) => onValueChange([v[0], v[1]] as [number, number])}
        className="relative flex h-5 w-full touch-none select-none items-center"
      >
        <SliderPrimitive.Track className="relative h-1 w-full grow overflow-hidden rounded-full bg-muted">
          <SliderPrimitive.Range className="absolute h-full bg-primary" />
        </SliderPrimitive.Track>
        <SliderPrimitive.Thumb className="block h-4 w-4 rounded-full border border-primary bg-background shadow focus:outline-none focus:ring-2 focus:ring-ring" />
        <SliderPrimitive.Thumb className="block h-4 w-4 rounded-full border border-primary bg-background shadow focus:outline-none focus:ring-2 focus:ring-ring" />
      </SliderPrimitive.Root>
      <div className="flex justify-between text-xs text-muted-foreground">
        <span>{formatValue ? formatValue(value[0]) : value[0]}</span>
        <span>{formatValue ? formatValue(value[1]) : value[1]}</span>
      </div>
    </div>
  );
}
```

- [ ] **Step 4.2: multi-select primitive (간단 chip 형태)**

`packages/ui/primitives/multi-select.tsx`:

```typescript
"use client";
import { cn } from "../lib/cn";

interface MultiSelectOption {
  value: string;
  label: string;
}

interface MultiSelectProps {
  options: MultiSelectOption[];
  value: string[];
  onValueChange: (next: string[]) => void;
  className?: string;
}

export function MultiSelect({ options, value, onValueChange, className }: MultiSelectProps) {
  const toggle = (v: string) => {
    if (value.includes(v)) onValueChange(value.filter((x) => x !== v));
    else onValueChange([...value, v]);
  };
  return (
    <div className={cn("flex flex-wrap gap-2", className)}>
      {options.map((opt) => {
        const selected = value.includes(opt.value);
        return (
          <button
            key={opt.value}
            type="button"
            aria-pressed={selected}
            onClick={() => toggle(opt.value)}
            className={cn(
              "rounded-full border px-3 py-1 text-sm transition",
              selected
                ? "border-primary bg-primary text-primary-foreground"
                : "border-border bg-background hover:bg-muted",
            )}
          >
            {opt.label}
          </button>
        );
      })}
    </div>
  );
}
```

- [ ] **Step 4.3: packages/ui/index.ts export**

```typescript
export { RangeSlider } from "./primitives/range-slider";
export { MultiSelect } from "./primitives/multi-select";
```

- [ ] **Step 4.4: i18n listings.ko.json**

`apps/web/lib/i18n/messages/listings.ko.json`:

```json
{
  "listings": {
    "page": {
      "title": "매물 검색"
    },
    "search": {
      "placeholder": "지역명을 검색해 주세요"
    },
    "filter": {
      "type": "매물 종류",
      "transaction": "거래 방식",
      "areaM2": "면적 (m²)",
      "priceKrw": "가격 (원)",
      "sort": "정렬"
    },
    "type": {
      "factory": "공장",
      "warehouse": "창고",
      "office": "사무실",
      "knowledge_industry_center": "지식산업센터",
      "industrial_land": "산업단지",
      "logistics_center": "물류센터"
    },
    "transaction": {
      "sale": "매매",
      "monthly_rent": "월세",
      "jeonse": "전세"
    },
    "sort": {
      "created_at_desc": "최신순",
      "price_asc": "가격 낮은 순",
      "price_desc": "가격 높은 순",
      "area_asc": "면적 좁은 순",
      "area_desc": "면적 넓은 순"
    },
    "card": {
      "viewCount": "조회",
      "bookmarkCount": "관심",
      "favoritePlaceholder": "즐겨찾기"
    },
    "empty": "조건에 맞는 매물이 없어요",
    "loading": "매물을 불러오는 중이에요",
    "errors": {
      "fetchFailed": "매물을 불러오지 못했어요. 잠시 후 다시 시도해 주세요."
    }
  }
}
```

`apps/web/i18n.ts` 의 message merge 에 listings 추가:

```typescript
const [common, auth, listings] = await Promise.all([
  import("./lib/i18n/ko.json"),
  import("./lib/i18n/messages/auth.ko.json"),
  import("./lib/i18n/messages/listings.ko.json"),
]);
return {
  locale,
  messages: { ...common.default, ...auth.default, ...listings.default },
};
```

- [ ] **Step 4.5: filter-bar.tsx + search-bar.tsx**

`apps/web/components/listings/search-bar.tsx`:

```typescript
"use client";
import { useTranslations } from "next-intl";
import { Input } from "@gongzzang/ui";
import { useState } from "react";

export function SearchBar() {
  const t = useTranslations("listings.search");
  const [value, setValue] = useState("");
  return (
    <div className="w-full max-w-md">
      <Input
        type="search"
        value={value}
        onChange={(e) => setValue(e.target.value)}
        placeholder={t("placeholder")}
        aria-label={t("placeholder")}
      />
    </div>
  );
}
```

(NOTE: 실제 지역 검색은 미래 — 지금은 input 만 자리. T9 Open question 1.)

`apps/web/components/listings/filter-bar.tsx`:

```typescript
"use client";
import { useTranslations } from "next-intl";
import { MultiSelect, RangeSlider } from "@gongzzang/ui";
import { useListingsStore } from "@/stores/listings";
import type { ListingType, TransactionType, SortKey } from "@/lib/listings/filters";
import { formatAreaM2, formatPriceKrw } from "@/lib/listings/format";

const TYPES: ListingType[] = [
  "factory", "warehouse", "office",
  "knowledge_industry_center", "industrial_land", "logistics_center",
];
const TXNS: TransactionType[] = ["sale", "monthly_rent", "jeonse"];
const SORTS: SortKey[] = [
  "created_at_desc", "price_asc", "price_desc", "area_asc", "area_desc",
];

const AREA_MIN = 0;
const AREA_MAX = 10_000;
const PRICE_MIN = 0;
const PRICE_MAX = 100_000_000_000; // 1000억

export function FilterBar() {
  const t = useTranslations("listings");
  const filters = useListingsStore((s) => s.filters);
  const patch = useListingsStore((s) => s.patchFilters);

  const typeOptions = TYPES.map((v) => ({ value: v, label: t(`type.${v}`) }));
  const txnOptions = TXNS.map((v) => ({ value: v, label: t(`transaction.${v}`) }));

  return (
    <div className="flex flex-col gap-4 p-4">
      <section aria-label={t("filter.type")}>
        <h3 className="mb-2 text-sm font-semibold">{t("filter.type")}</h3>
        <MultiSelect
          options={typeOptions}
          value={filters.types}
          onValueChange={(v) => patch({ types: v as ListingType[] })}
        />
      </section>
      <section aria-label={t("filter.transaction")}>
        <h3 className="mb-2 text-sm font-semibold">{t("filter.transaction")}</h3>
        <MultiSelect
          options={txnOptions}
          value={filters.transactions}
          onValueChange={(v) => patch({ transactions: v as TransactionType[] })}
        />
      </section>
      <section aria-label={t("filter.areaM2")}>
        <h3 className="mb-2 text-sm font-semibold">{t("filter.areaM2")}</h3>
        <RangeSlider
          min={AREA_MIN}
          max={AREA_MAX}
          step={100}
          value={[filters.minAreaM2 ?? AREA_MIN, filters.maxAreaM2 ?? AREA_MAX]}
          onValueChange={([min, max]) =>
            patch({
              minAreaM2: min === AREA_MIN ? undefined : min,
              maxAreaM2: max === AREA_MAX ? undefined : max,
            })
          }
          formatValue={formatAreaM2}
        />
      </section>
      <section aria-label={t("filter.priceKrw")}>
        <h3 className="mb-2 text-sm font-semibold">{t("filter.priceKrw")}</h3>
        <RangeSlider
          min={PRICE_MIN}
          max={PRICE_MAX}
          step={10_000_000}
          value={[filters.minPriceKrw ?? PRICE_MIN, filters.maxPriceKrw ?? PRICE_MAX]}
          onValueChange={([min, max]) =>
            patch({
              minPriceKrw: min === PRICE_MIN ? undefined : min,
              maxPriceKrw: max === PRICE_MAX ? undefined : max,
            })
          }
          formatValue={formatPriceKrw}
        />
      </section>
      <section aria-label={t("filter.sort")}>
        <h3 className="mb-2 text-sm font-semibold">{t("filter.sort")}</h3>
        <select
          value={filters.sort}
          onChange={(e) => patch({ sort: e.target.value as SortKey })}
          className="rounded border border-border bg-background px-3 py-2 text-sm"
          aria-label={t("filter.sort")}
        >
          {SORTS.map((s) => (
            <option key={s} value={s}>{t(`sort.${s}`)}</option>
          ))}
        </select>
      </section>
    </div>
  );
}
```

- [ ] **Step 4.6: typecheck + lint + commit**

```bash
pnpm typecheck && pnpm lint
git add packages/ui/primitives/range-slider.tsx packages/ui/primitives/multi-select.tsx packages/ui/index.ts packages/ui/package.json apps/web/components/listings/search-bar.tsx apps/web/components/listings/filter-bar.tsx apps/web/lib/i18n/messages/listings.ko.json apps/web/i18n.ts pnpm-lock.yaml
git commit -m "feat(6ii-T4): RangeSlider + MultiSelect primitives + FilterBar + i18n

- packages/ui: RangeSlider (Radix Slider) + MultiSelect (chip toggle) primitives
- listings/search-bar: 지역 검색 input (실 검색 API 통합 미래)
- listings/filter-bar: 종류/거래 multi + 평수/가격 range + 정렬 select + i18n
- listings.ko.json: 6 type + 3 transaction + 5 sort + filter labels"
```

---
