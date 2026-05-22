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
import { useTranslations } from "next-intl";

import type { Notification } from "@/lib/notifications/api";
import { ROUTES } from "@/lib/routes";

interface NotificationCardProps {
  notification: Notification;
}

export function NotificationCard({ notification }: NotificationCardProps): React.ReactElement {
  const t = useTranslations("notifications");
  const { title, description, href, Icon } = renderByKind(notification, t);
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

type TFn = (key: string, values?: Record<string, string | number>) => string;

function renderByKind(n: Notification, t: TFn): RenderResult {
  const payload = n.payload as Record<string, unknown>;
  const listingId = typeof payload.listing_id === "string" ? payload.listing_id : null;
  const listingTitle =
    typeof payload.title === "string" ? payload.title : t("fallbackListingTitle");

  switch (n.kind) {
    case "listing_approved":
      return {
        title: t("approved.title", { title: listingTitle }),
        description: t("approved.description"),
        href: listingId ? ROUTES.listings.detail(listingId) : undefined,
        Icon: ListChecks,
      };
    case "listing_rejected":
      return {
        title: t("rejected.title", { title: listingTitle }),
        description:
          typeof payload.reason === "string"
            ? t("rejected.reasonPrefix", { reason: payload.reason })
            : t("rejected.descriptionFallback"),
        href: listingId ? ROUTES.listings.detail(listingId) : undefined,
        Icon: XCircle,
      };
    case "listing_bookmarked": {
      const bookmarker =
        typeof payload.bookmarker_name === "string"
          ? payload.bookmarker_name
          : t("fallbackBookmarker");
      return {
        title: t("bookmarked.title", { bookmarker, title: listingTitle }),
        href: listingId ? ROUTES.listings.detail(listingId) : undefined,
        Icon: Heart,
      };
    }
    default:
      return {
        title: t("default"),
        Icon: ListChecks,
      };
  }
}
