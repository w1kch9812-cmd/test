import type { Route } from "next";
import Link from "next/link";
import { getTranslations } from "next-intl/server";
import { ROUTES } from "@/lib/routes";

export default async function NotFound(): Promise<React.ReactElement> {
  const t = await getTranslations("listings.detailNotFound");

  return (
    <main className="mx-auto max-w-md px-4 py-20 text-center">
      <h1 className="text-2xl font-bold text-[var(--color-ink)]">{t("title")}</h1>
      <p className="mt-2 text-sm text-[var(--color-muted)]">{t("description")}</p>
      <Link
        href={ROUTES.listings.index as Route}
        className="mt-6 inline-block text-sm text-[var(--color-primary)] underline"
      >
        {t("backToSearch")}
      </Link>
    </main>
  );
}
