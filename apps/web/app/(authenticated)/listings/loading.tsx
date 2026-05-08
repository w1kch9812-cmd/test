import { getTranslations } from "next-intl/server";

export default async function ListingsLoading(): Promise<React.ReactElement> {
  const t = await getTranslations("listings");
  return (
    <main className="flex h-screen items-center justify-center">
      <div className="text-[var(--color-muted-fg)] text-sm">{t("loading")}</div>
    </main>
  );
}
