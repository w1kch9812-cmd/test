// apps/web/components/panels/parcel/skeletons.tsx
"use client";
import { Skeleton } from "@gongzzang/ui";
import { useTranslations } from "next-intl";

export function ParcelLoadingSkeleton() {
  return (
    <div className="flex flex-col gap-3 p-6">
      <Skeleton className="h-6 w-32" />
      <Skeleton className="h-4 w-64" />
      <Skeleton className="h-4 w-48" />
      <Skeleton className="h-32 w-full" />
    </div>
  );
}

export function ParcelErrorCard({ error }: { error: unknown }) {
  const t = useTranslations("panels.parcel");
  const msg = error instanceof Error ? error.message : String(error);
  return (
    <div className="p-6">
      <div className="text-[length:var(--text-body-md)] font-semibold text-[var(--color-error)]">
        {t("errors.loadFailed")}
      </div>
      <div className="mt-2 text-[length:var(--text-caption)] text-[var(--color-muted)]">{msg}</div>
    </div>
  );
}

export function ParcelEmptyCard() {
  const t = useTranslations("panels.parcel");
  return <div className="p-6 text-center text-[var(--color-muted)]">{t("empty")}</div>;
}
