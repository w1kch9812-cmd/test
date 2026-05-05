"use client";
import { MultiSelect, RangeSlider } from "@gongzzang/ui";
import { useTranslations } from "next-intl";
import type { ListingType, SortKey, TransactionType } from "@/lib/listings/filters";
import { formatAreaM2, formatPriceKrw } from "@/lib/listings/format";
import { useListingsStore } from "@/stores/listings";

const TYPES: ListingType[] = [
  "factory",
  "warehouse",
  "office",
  "knowledge_industry_center",
  "industrial_land",
  "logistics_center",
];
const TXNS: TransactionType[] = ["sale", "monthly_rent", "jeonse"];
const SORTS: SortKey[] = ["created_at_desc", "price_asc", "price_desc", "area_asc", "area_desc"];

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
          className="rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-3 py-2 text-sm"
          aria-label={t("filter.sort")}
        >
          {SORTS.map((s) => (
            <option key={s} value={s}>
              {t(`sort.${s}`)}
            </option>
          ))}
        </select>
      </section>
    </div>
  );
}
