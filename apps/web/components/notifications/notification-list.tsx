"use client";

/**
 * SP6-v: 알림 목록 client component.
 *
 * 최근 365일 알림 fetch + read state 별 표시. mark-all-read 버튼 (kind 별).
 * 단순 list — virtualization 은 FU (수십 건 수준 가정).
 */

import { Button } from "@gongzzang/ui";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { NotificationCard } from "@/components/notifications/notification-card";
import {
  fetchNotifications,
  markAllNotificationsRead,
  type Notification,
  type NotificationKind,
} from "@/lib/notifications/api";

export function NotificationList(): React.ReactElement {
  const qc = useQueryClient();

  const { data, isLoading, isError } = useQuery({
    queryKey: ["notifications", "list"],
    queryFn: () => fetchNotifications({ limit: 100 }),
  });

  const markAll = useMutation({
    mutationFn: (kind: NotificationKind) => markAllNotificationsRead(kind),
    onSuccess(count, kind) {
      toast.success(`${count}건 읽음 처리 (${labelForKind(kind)})`);
      void qc.invalidateQueries({ queryKey: ["notifications"] });
    },
    onError() {
      toast.error("읽음 처리 중 오류가 발생했어요");
    },
  });

  if (isLoading) {
    return <p className="text-sm text-[var(--color-muted)]">불러오는 중...</p>;
  }

  if (isError || !data) {
    return <p className="text-sm text-red-600">알림을 불러오지 못했어요.</p>;
  }

  if (data.length === 0) {
    return <p className="text-sm text-[var(--color-muted)]">알림이 없어요.</p>;
  }

  const grouped = groupByKind(data);

  return (
    <div className="space-y-6">
      {Object.entries(grouped).map(([kind, items]) => (
        <section key={kind} aria-labelledby={`heading-${kind}`} className="space-y-3">
          <header className="flex items-center justify-between">
            <h2 id={`heading-${kind}`} className="text-sm font-medium text-[var(--color-ink)]">
              {labelForKind(kind as NotificationKind)} ({items.length})
            </h2>
            {items.some((n) => n.read_at == null) && (
              <Button
                type="button"
                variant="ghost"
                onClick={() => markAll.mutate(kind as NotificationKind)}
                disabled={markAll.isPending}
              >
                모두 읽음
              </Button>
            )}
          </header>
          <div className="space-y-2">
            {items.map((n) => (
              <NotificationCard key={n.id} notification={n} />
            ))}
          </div>
        </section>
      ))}
    </div>
  );
}

function groupByKind(notifications: Notification[]): Record<string, Notification[]> {
  const groups: Record<string, Notification[]> = {};
  for (const n of notifications) {
    const bucket = groups[n.kind] ?? [];
    bucket.push(n);
    groups[n.kind] = bucket;
  }
  return groups;
}

function labelForKind(kind: NotificationKind | string): string {
  switch (kind) {
    case "listing_approved":
      return "매물 승인";
    case "listing_rejected":
      return "매물 반려";
    case "listing_bookmarked":
      return "즐겨찾기";
    default:
      return "기타";
  }
}
