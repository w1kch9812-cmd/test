// apps/web/components/panels/parcel/buildings.tsx
"use client";
import { useTranslations } from "next-intl";
import type { BuildingsResponse } from "@/lib/api/buildings";
import type { PanelStackEntry } from "@/lib/panel/types";

export function ParcelBuildingsCard({
  entry,
  data,
}: {
  entry: Extract<PanelStackEntry, { kind: "parcel" }>;
  data: BuildingsResponse;
}) {
  const t = useTranslations("panels.parcel.buildings");
  if (data.buildings.length === 0) {
    return <div className="p-6 text-center text-[var(--color-muted)]">{t("none")}</div>;
  }
  return (
    <div className="flex flex-col gap-3 p-6">
      <header className="flex items-baseline gap-2">
        <h2 className="text-[length:var(--text-title-md)] font-semibold">{t("title")}</h2>
        <span className="text-[length:var(--text-caption)] text-[var(--color-muted)]">
          {data.buildings.length} {t("count")}
        </span>
      </header>
      <ul className="flex flex-col gap-2">
        {data.buildings.map((b) => (
          <li
            key={b.id}
            className="rounded-md border border-[var(--color-hairline)] p-3 text-[length:var(--text-body-sm)]"
          >
            <div className="font-semibold text-[var(--color-ink)]">{b.name}</div>
            <div className="text-[var(--color-muted)]">
              {b.purpose} · {b.total_area_m2.toLocaleString("ko-KR")} ㎡
              {b.approved_at && ` · ${b.approved_at}`}
            </div>
          </li>
        ))}
      </ul>
      {/* PNU 의 entry.id 는 i18n 라벨 표시 외 미사용 — 본 view 는 list-only */}
      <span className="hidden">{entry.id}</span>
    </div>
  );
}
