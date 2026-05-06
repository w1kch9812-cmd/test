"use client";

/**
 * SP6-v: 헤더 종 badge 용 unread count polling hook.
 *
 * 1분 interval (spec § 7 위험 — 트래픽 ↑ 시 5분 또는 SSE FU 80). retry 0 —
 * 401 인경우 layout 가 redirect.
 */

import { useQuery } from "@tanstack/react-query";

import { fetchUnreadCount } from "@/lib/notifications/api";

const POLL_INTERVAL_MS = 60_000;

export function useUnreadCount(): { count: number; isLoading: boolean } {
  const query = useQuery({
    queryKey: ["notifications", "unread-count"],
    queryFn: fetchUnreadCount,
    refetchInterval: POLL_INTERVAL_MS,
    refetchOnWindowFocus: true,
    retry: 0,
  });

  return {
    count: query.data ?? 0,
    isLoading: query.isLoading,
  };
}
