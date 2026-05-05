import { Button } from "@gongzzang/ui";
import { getTranslations } from "next-intl/server";

export default async function LoginPage({
  searchParams,
}: {
  searchParams: Promise<{ returnTo?: string }>;
}) {
  const t = await getTranslations("auth.login");
  const params = await searchParams;
  const returnTo = params.returnTo ?? "/profile";

  return (
    <main className="mx-auto flex min-h-screen max-w-md flex-col items-center justify-center gap-6 p-8">
      <h1 className="text-2xl font-bold">{t("title")}</h1>
      <p className="text-center text-muted-foreground">{t("description")}</p>

      <form action="/api/auth/login" method="POST" className="w-full">
        <input type="hidden" name="returnTo" value={returnTo} />
        <Button type="submit" className="w-full">
          {t("loginButton")}
        </Button>
      </form>
    </main>
  );
}
