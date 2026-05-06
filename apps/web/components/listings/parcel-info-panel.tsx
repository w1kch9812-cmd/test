"use client";

import { useTranslations } from "next-intl";
import { useListingsStore } from "@/stores/listings";

/**
 * 폴리곤 클릭 시 활성화되는 PNU 칩 — ADR 0018 (PNU-First identity).
 *
 * 사용자가 필지 폴리곤을 클릭 → store.filters.pnu 가 채워짐 → 카드 list 가 그 PNU 매물만
 * 표시. 본 컴포넌트는 *현재 어느 PNU 가 잠겨있는지* 시각화하고, X 버튼으로 해제.
 *
 * 설치 위치는 호출자 결정 (지도 위 overlay or 카드 list 위 sticky).
 */
export function ParcelInfoPanel() {
  const t = useTranslations("listings");
  const pnu = useListingsStore((s) => s.filters.pnu);
  const patchFilters = useListingsStore((s) => s.patchFilters);

  if (!pnu) return null;

  return (
    <div className="flex items-center gap-2 rounded-md border border-[var(--color-hairline)] bg-[var(--color-canvas)] px-3 py-2 text-sm">
      <span className="font-mono text-[var(--color-ink)]">PNU {pnu}</span>
      <button
        type="button"
        className="ml-auto text-[var(--color-muted)] hover:text-[var(--color-ink)]"
        aria-label={t("clearPnuFilter")}
        onClick={() => patchFilters({ pnu: undefined })}
      >
        ×
      </button>
    </div>
  );
}
