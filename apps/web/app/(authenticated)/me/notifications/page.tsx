/**
 * SP6-v: `/me/notifications` 알림 페이지.
 *
 * 인증 사용자 전용 (proxy.ts 가 default protected — admin 가드 불필요).
 */

import { getTranslations } from "next-intl/server";
import { NotificationList } from "@/components/notifications/notification-list";

export default async function NotificationsPage(): Promise<React.ReactElement> {
  const t = await getTranslations("notifications.page");

  return (
    <main className="mx-auto max-w-2xl px-4 py-8">
      <header className="mb-6">
        <h1 className="text-2xl font-bold text-[var(--color-ink)]">{t("title")}</h1>
        <p className="mt-2 text-sm text-[var(--color-muted)]">{t("description")}</p>
      </header>
      <NotificationList />
    </main>
  );
}
