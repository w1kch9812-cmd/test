/**
 * SP6-v: 알림 API client (zod schemas + fetch + mutations).
 *
 * Backend `notification_domain::kind::NotificationKind` 와 동기화 — utoipa
 * 자동 생성 (FU 55) 까지는 manual fork.
 */

import { z } from "zod";
import { apiProxyClient } from "@/lib/api/api-proxy-client.generated";

export const NOTIFICATION_KINDS = [
  "listing_approved",
  "listing_rejected",
  "listing_bookmarked",
  "other",
] as const;

export type NotificationKind = (typeof NOTIFICATION_KINDS)[number];

export const NotificationSchema = z.object({
  id: z.string(),
  kind: z.string(),
  payload: z.record(z.string(), z.unknown()),
  read_at: z.string().nullish(),
  created_at: z.string(),
});

export type Notification = z.infer<typeof NotificationSchema>;

export const ListResponseSchema = z.object({
  notifications: z.array(NotificationSchema),
});

export const UnreadCountResponseSchema = z.object({
  count: z.number().int().nonnegative(),
});

export interface ListNotificationsInput {
  unreadOnly?: boolean;
  limit?: number;
}

export async function fetchNotifications(
  input: ListNotificationsInput = {},
): Promise<Notification[]> {
  const sp = new URLSearchParams();
  if (input.unreadOnly) sp.set("unread_only", "true");
  if (input.limit !== undefined) sp.set("limit", String(input.limit));
  const json = await apiProxyClient.notificationsList.getJson<unknown>({ searchParams: sp });
  return ListResponseSchema.parse(json).notifications;
}

export async function fetchUnreadCount(): Promise<number> {
  const json = await apiProxyClient.notificationsUnreadCount.getJson<unknown>();
  return UnreadCountResponseSchema.parse(json).count;
}

export async function markNotificationRead(id: string): Promise<void> {
  await apiProxyClient.notificationMarkRead.patch({ id });
}

export async function markAllNotificationsRead(kind: NotificationKind): Promise<number> {
  const json = await apiProxyClient.notificationsMarkAllRead.postJson<unknown>({
    searchParams: new URLSearchParams({ kind }),
  });
  return z.object({ marked_count: z.number().int().nonnegative() }).parse(json).marked_count;
}
