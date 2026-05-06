"use client";

/**
 * SP6-v: 알림 단건 카드.
 *
 * kind 별 한국어 메시지 + click 시 navigation. payload 구조는 backend
 * NotificationKind 별로 다름 — defensive parsing.
 */

import { Card, CardContent } from "@gongzzang/ui";
import { Heart, ListChecks, type LucideIcon, XCircle } from "lucide-react";
import type { Route } from "next";
import Link from "next/link";

import type { Notification } from "@/lib/notifications/api";

interface NotificationCardProps {
  notification: Notification;
}

export function NotificationCard({ notification }: NotificationCardProps): React.ReactElement {
  const { title, description, href, Icon } = renderByKind(notification);
  const isRead = notification.read_at != null;

  const inner = (
    <Card surface={isRead ? "default" : "cream-card"} className="transition-shadow">
      <CardContent className="flex gap-3 py-4">
        <Icon className="h-5 w-5 shrink-0 text-[var(--color-primary)]" aria-hidden="true" />
        <div className="flex-1">
          <p className={isRead ? "text-sm text-[var(--color-muted)]" : "font-medium text-sm"}>
            {title}
          </p>
          {description && <p className="mt-1 text-xs text-[var(--color-muted)]">{description}</p>}
          <p className="mt-2 text-xs text-[var(--color-muted)]">
            {new Date(notification.created_at).toLocaleString("ko-KR")}
          </p>
        </div>
      </CardContent>
    </Card>
  );

  return href ? (
    <Link href={href as Route} className="block">
      {inner}
    </Link>
  ) : (
    inner
  );
}

interface RenderResult {
  title: string;
  description?: string;
  href?: string;
  Icon: LucideIcon;
}

function renderByKind(n: Notification): RenderResult {
  const payload = n.payload as Record<string, unknown>;
  const listingId = typeof payload.listing_id === "string" ? payload.listing_id : null;
  const listingTitle = typeof payload.title === "string" ? payload.title : "매물";

  switch (n.kind) {
    case "listing_approved":
      return {
        title: `매물 "${listingTitle}" 이(가) 승인됐어요`,
        description: "이제 다른 사용자에게 공개돼요.",
        href: listingId ? `/listings/${listingId}` : undefined,
        Icon: ListChecks,
      };
    case "listing_rejected":
      return {
        title: `매물 "${listingTitle}" 이(가) 반려됐어요`,
        description:
          typeof payload.reason === "string"
            ? `사유: ${payload.reason}`
            : "사유는 매물 상세에서 확인해 주세요.",
        href: listingId ? `/listings/${listingId}` : undefined,
        Icon: XCircle,
      };
    case "listing_bookmarked": {
      const bookmarker =
        typeof payload.bookmarker_name === "string" ? payload.bookmarker_name : "한 사용자";
      return {
        title: `${bookmarker} 님이 "${listingTitle}" 를 즐겨찾기 했어요`,
        href: listingId ? `/listings/${listingId}` : undefined,
        Icon: Heart,
      };
    }
    default:
      return {
        title: "알림이 도착했어요",
        Icon: ListChecks,
      };
  }
}
