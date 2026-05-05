"use client";
import {
  MultiSelect,
  RangeSlider,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@gongzzang/ui";
import { useTranslations } from "next-intl";
import type { ListingType, SortKey, TransactionType } from "@/lib/listings/filters";
import { formatAreaM2, formatPriceKrw } from "@/lib/listings/format";
import { useListingsStore } from "@/stores/listings";

// 공짱 = 산업용 부동산. 현재 노출 = 공장 / 창고 / 토지 (3종).
// backend schema 는 6종 (office, knowledge_industry_center, logistics_center 자리).
const TYPES: ListingType[] = ["factory", "warehouse", "industrial_land"];
const TXNS: TransactionType[] = ["sale", "monthly_rent", "jeonse"];
const SORTS: SortKey[] = ["created_at_desc", "price_asc", "price_desc", "area_asc", "area_desc"];

const AREA_MIN = 0;
const AREA_MAX = 10_000;
const PRICE_MIN = 0;
const PRICE_MAX = 100_000_000_000; // 1000억

/**
 * 상단 horizontal bar 형태의 필터.
 * 매물 종류 + 거래방식 chip + 면적/가격 range + 정렬 select.
 */
export function FilterBar() {
  const t = useTranslations("listings");
  const filters = useListingsStore((s) => s.filters);
  const patch = useListingsStore((s) => s.patchFilters);

  const typeOptions = TYPES.map((v) => ({ value: v, label: t(`type.${v}`) }));
  const txnOptions = TXNS.map((v) => ({ value: v, label: t(`transaction.${v}`) }));

  const labelClass =
    "text-[length:var(--text-caption-uppercase)] font-medium tracking-[var(--tracking-uppercase)] uppercase text-[var(--color-muted)] whitespace-nowrap";

  return (
    <div className="flex flex-wrap items-center gap-x-6 gap-y-3 bg-[var(--color-canvas)] px-6 py-3.5">
      <div className="flex items-center gap-3">
        <span className={labelClass}>{t("filter.type")}</span>
        <MultiSelect
          options={typeOptions}
          value={filters.types}
          onValueChange={(v) => patch({ types: v as ListingType[] })}
        />
      </div>
      <div className="flex items-center gap-3">
        <span className={labelClass}>{t("filter.transaction")}</span>
        <MultiSelect
          options={txnOptions}
          value={filters.transactions}
          onValueChange={(v) => patch({ transactions: v as TransactionType[] })}
        />
      </div>
      <div className="flex items-center gap-3">
        <span className={labelClass}>{t("filter.areaM2")}</span>
        <div className="w-44">
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
        </div>
      </div>
      <div className="flex items-center gap-3">
        <span className={labelClass}>{t("filter.priceKrw")}</span>
        <div className="w-44">
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
        </div>
      </div>
      <div className="ml-auto flex items-center gap-3">
        <span className={labelClass}>{t("filter.sort")}</span>
        <Select value={filters.sort} onValueChange={(v) => patch({ sort: v as SortKey })}>
          <SelectTrigger className="h-9 w-44" aria-label={t("filter.sort")}>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {SORTS.map((s) => (
              <SelectItem key={s} value={s}>
                {t(`sort.${s}`)}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
    </div>
  );
}
