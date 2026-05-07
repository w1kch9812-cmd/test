"use client";
import { Skeleton } from "@gongzzang/ui";
import { useTranslations } from "next-intl";

export function ListingLoadingSkeleton() {
  return (
    <div className="flex flex-col gap-3 p-6">
      <Skeleton className="aspect-[4/3] w-full" />
      <Skeleton className="h-6 w-32" />
      <Skeleton className="h-4 w-64" />
    </div>
  );
}

export function ListingErrorCard({ error }: { error: unknown }) {
  const t = useTranslations("panels.listing");
  return (
    <div className="p-6">
      <div className="text-[var(--color-error)]">{t("errors.loadFailed")}</div>
      <div className="mt-2 text-[length:var(--text-caption)] text-[var(--color-muted)]">
        {error instanceof Error ? error.message : String(error)}
      </div>
    </div>
  );
}

export function ListingEmptyCard() {
  const t = useTranslations("panels.listing");
  return <div className="p-6 text-center text-[var(--color-muted)]">{t("notFound")}</div>;
}
