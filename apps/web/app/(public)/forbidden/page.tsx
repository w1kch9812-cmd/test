import { getTranslations } from "next-intl/server";

export default async function ForbiddenPage() {
  const t = await getTranslations("auth.forbidden");
  return (
    <main className="mx-auto flex min-h-screen max-w-md flex-col items-center justify-center gap-4 p-8 text-center">
      <h1 className="text-2xl font-bold">{t("title")}</h1>
      <p className="text-muted-foreground">{t("description")}</p>
    </main>
  );
}
