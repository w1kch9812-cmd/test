"use client";

/**
 * SP6-v: 헤더 종 badge.
 *
 * unread count 표시 + 클릭 시 `/me/notifications` navigate. 1분 polling
 * (FU 80 = SSE 로 evolve).
 */

import { Badge } from "@gongzzang/ui";
import { Bell } from "lucide-react";
import type { Route } from "next";
import Link from "next/link";

import { useUnreadCount } from "@/lib/notifications/use-unread-count";

export function NotificationBell(): React.ReactElement {
  const { count } = useUnreadCount();
  const display = count > 99 ? "99+" : String(count);

  return (
    <Link
      href={"/me/notifications" as Route}
      aria-label={count > 0 ? `미읽음 알림 ${count}건` : "알림"}
      className="relative inline-flex items-center justify-center rounded-md p-2 hover:bg-[var(--color-surface-cream-strong)]"
    >
      <Bell className="h-5 w-5" aria-hidden="true" />
      {count > 0 && (
        <Badge
          variant="coral"
          className="absolute -right-1 -top-1 px-1.5 py-0 text-xs"
          aria-hidden="true"
        >
          {display}
        </Badge>
      )}
    </Link>
  );
}
