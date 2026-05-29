import { Button } from "@gongzzang/ui";
import Link from "next/link";
import { getTranslations } from "next-intl/server";

export default async function NotFound() {
  const t = await getTranslations("notFoundPage");

  return (
    <main className="flex min-h-screen flex-col items-center justify-center gap-4 p-8">
      <h2 className="text-2xl font-bold">{t("title")}</h2>
      <p className="text-[var(--color-muted-fg)]">{t("description")}</p>
      <Button asChild>
        <Link href="/">{t("homeLink")}</Link>
      </Button>
    </main>
  );
}
