// apps/web/components/panels/parcel/listings.tsx
"use client";
import { useTranslations } from "next-intl";
import type { ListingsResponse } from "@/lib/listings/api";
import { UNITS } from "@/lib/listings/format";
import type { PanelStackEntry } from "@/lib/panel/types";
import { usePanelStack } from "@/lib/panel/use-panel-stack";

export function ParcelListingsCard({
  entry,
  data,
}: {
  entry: Extract<PanelStackEntry, { kind: "parcel" }>;
  data: ListingsResponse;
}) {
  const t = useTranslations("panels.parcel.listings");
  const { push } = usePanelStack();
  if (data.listings.length === 0) {
    return <div className="p-6 text-center text-[var(--color-muted)]">{t("none")}</div>;
  }
  return (
    <div className="flex flex-col gap-3 p-6">
      <header>
        <h2 className="text-[length:var(--text-title-md)] font-semibold">
          {t("title", { count: data.total })}
        </h2>
      </header>
      <ul className="flex flex-col gap-2">
        {data.listings.map((l) => (
          <li key={l.id}>
            <button
              type="button"
              onClick={() => push({ kind: "listing", id: l.id, view: "summary" })}
              className="block w-full rounded-md border border-[var(--color-hairline)] p-3 text-left hover:bg-[var(--color-surface-cream-strong)]"
            >
              <div className="font-semibold text-[var(--color-ink)]">{l.title}</div>
              <div className="text-[length:var(--text-caption)] text-[var(--color-muted)]">
                {l.price_krw.toLocaleString("ko-KR")} {UNITS.krw} ·{" "}
                {l.area_m2.toLocaleString("ko-KR")} {UNITS.m2}
              </div>
            </button>
          </li>
        ))}
      </ul>
      <span className="hidden">{entry.id}</span>
    </div>
  );
}
