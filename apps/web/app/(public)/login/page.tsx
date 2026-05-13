import { Button } from "@gongzzang/ui";
import { getTranslations } from "next-intl/server";
import { API, ROUTES } from "@/lib/routes";

export default async function LoginPage({
  searchParams,
}: {
  searchParams: Promise<{ returnTo?: string }>;
}) {
  const t = await getTranslations("auth.login");
  const params = await searchParams;
  const returnTo = params.returnTo ?? ROUTES.profile;

  return (
    <main className="mx-auto flex min-h-screen max-w-md flex-col items-center justify-center gap-6 p-8">
      <h1 className="text-[length:var(--text-display-md)] font-semibold tracking-[var(--tracking-display-md)] text-[var(--color-ink)]">
        {t("title")}
      </h1>
      <p className="text-center text-[length:var(--text-body-md)] text-[var(--color-muted)]">
        {t("description")}
      </p>

      <form action={API.auth.login} method="POST" className="w-full">
        <input type="hidden" name="returnTo" value={returnTo} />
        <Button type="submit" size="lg" className="w-full">
          {t("loginButton")}
        </Button>
      </form>
    </main>
  );
}
