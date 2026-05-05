import { Button } from "@gongzzang/ui";
import { cookies } from "next/headers";
import { getTranslations } from "next-intl/server";
import { SID_COOKIE_NAME } from "@/lib/session/cookie";
import { getSession } from "@/lib/session/store";

export default async function ProfilePage() {
  const t = await getTranslations("auth.profile");
  const cookieStore = await cookies();
  const sid = cookieStore.get(SID_COOKIE_NAME)?.value;
  if (!sid) return null; // layout 이 redirect 처리, defense-in-depth

  const session = await getSession(sid);
  if (!session) return null;

  return (
    <main className="mx-auto flex min-h-screen max-w-2xl flex-col gap-6 p-8">
      <h1 className="text-[length:var(--text-display-sm)] font-semibold tracking-[var(--tracking-display-sm)] text-[var(--color-ink)]">
        {t("title")}
      </h1>
      <dl className="grid grid-cols-[8rem_1fr] gap-2 text-[length:var(--text-body-md)]">
        <dt className="text-[var(--color-muted)]">{t("userId")}</dt>
        <dd>{session.sub}</dd>
        <dt className="text-[var(--color-muted)]">{t("role")}</dt>
        <dd>{session.role}</dd>
      </dl>

      <form action="/api/auth/logout" method="POST">
        <Button type="submit" variant="secondary">
          {t("logoutButton")}
        </Button>
      </form>
    </main>
  );
}
