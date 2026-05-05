import { getTranslations } from "next-intl/server";

export default async function ListingsLoading() {
  const t = await getTranslations("listings");
  return (
    <main className="flex h-screen items-center justify-center">
      <div className="text-sm" style={{ color: "var(--color-muted-fg)" }}>
        {t("loading")}
      </div>
    </main>
  );
}
