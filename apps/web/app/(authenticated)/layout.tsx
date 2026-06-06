import type { Route } from "next";
import { cookies } from "next/headers";
import Link from "next/link";
import { redirect } from "next/navigation";
import { getTranslations } from "next-intl/server";
import { NotificationBell } from "@/components/notifications/notification-bell";
import { ROUTES } from "@/lib/routes";
import { SID_COOKIE_NAME } from "@/lib/session/cookie";
import { getSession } from "@/lib/session/store";

// cast: typed route generation (next build) 전 단계에서도 타입 오류 없이 redirect 가능
const LOGIN_ROUTE = ROUTES.login as Route;
const LISTINGS_ROUTE = ROUTES.listings.index as Route;

export const dynamic = "force-dynamic";

export default async function AuthenticatedLayout({ children }: { children: React.ReactNode }) {
  const t = await getTranslations("common");
  const cookieStore = await cookies();
  const sid = cookieStore.get(SID_COOKIE_NAME)?.value;
  if (!sid) {
    redirect(LOGIN_ROUTE);
  }
  const session = await getSession(sid);
  if (!session) {
    redirect(LOGIN_ROUTE);
  }
  return (
    <>
      <header className="flex items-center justify-between border-b border-[var(--color-hairline)] px-4 py-2">
        <Link href={LISTINGS_ROUTE} className="text-sm font-medium text-[var(--color-ink)]">
          {t("brandName")}
        </Link>
        <NotificationBell />
      </header>
      {children}
    </>
  );
}
