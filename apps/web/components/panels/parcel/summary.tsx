// apps/web/components/panels/parcel/summary.tsx
"use client";
import { useTranslations } from "next-intl";
import type { ParcelInfo } from "@/lib/api/parcels";
import type { PanelStackEntry } from "@/lib/panel/types";
import { usePanelStack } from "@/lib/panel/use-panel-stack";

export function ParcelSummaryCard({
  entry,
  data,
}: {
  entry: Extract<PanelStackEntry, { kind: "parcel" }>;
  data: ParcelInfo;
}) {
  const t = useTranslations("panels.parcel.summary");
  const { push } = usePanelStack();

  // T3 P1 gap: backend returns Korean names as empty strings until a
  // code → name lookup table lands (shared_kernel admin_division has codes only).
  // Render names if present, else fall back to code-based heading.
  const names = [data.sido_name, data.sigungu_name, data.eupmyeondong_name]
    .filter((s) => s.trim() !== "")
    .join(" ");
  const heading = names || t("codeFallback", { code: data.eupmyeondong_code });

  return (
    <div className="flex flex-col gap-4 p-6">
      <header>
        <div className="font-mono text-[length:var(--text-caption)] text-[var(--color-muted)]">
          PNU {entry.id}
        </div>
        <h2 className="text-[length:var(--text-title-lg)] font-semibold text-[var(--color-ink)]">
          {heading}
        </h2>
      </header>
      <dl className="grid grid-cols-2 gap-y-2 text-[length:var(--text-body-sm)]">
        <dt className="text-[var(--color-muted)]">{t("landUse")}</dt>
        <dd className="text-[var(--color-ink)]">{data.land_use_type}</dd>
        {data.zoning && (
          <>
            <dt className="text-[var(--color-muted)]">{t("zoning")}</dt>
            <dd className="text-[var(--color-ink)]">{data.zoning}</dd>
          </>
        )}
        {data.official_land_price_per_m2 != null && (
          <>
            <dt className="text-[var(--color-muted)]">{t("officialPrice")}</dt>
            <dd className="text-[var(--color-ink)]">
              {t("officialPricePerM2", {
                value: data.official_land_price_per_m2.toLocaleString("ko-KR"),
              })}
            </dd>
          </>
        )}
      </dl>
      <nav className="mt-4 flex flex-col gap-2">
        <button
          type="button"
          onClick={() => push({ kind: "parcel", id: entry.id, view: "buildings" })}
          className="rounded-md border border-[var(--color-hairline)] px-3 py-2 text-left hover:bg-[var(--color-surface-cream-strong)]"
        >
          {t("viewBuildings")} ›
        </button>
        <button
          type="button"
          onClick={() => push({ kind: "parcel", id: entry.id, view: "listings" })}
          className="rounded-md border border-[var(--color-hairline)] px-3 py-2 text-left hover:bg-[var(--color-surface-cream-strong)]"
        >
          {t("viewListings")} ›
        </button>
      </nav>
    </div>
  );
}
