/**
 * `services/api` 의 `POST /internal/auth/event` 단일 SSOT 호출 helper.
 *
 * Audit (2026-05-08) — 이전엔 callback / logout / refresh 3 routes 가 *inline fetch*
 * 패턴 중복 + `X-Internal-Auth` 헤더 누락 → API 측 unauthenticated 차단을 우회.
 * 본 helper 가 헤더 + URL + 직렬화 단일화. 새 emit 위치 추가 시 여기서.
 *
 * Failure 정책:
 * - audit log emit 실패 = *서비스 영향 0* (사용자 경험 정상). 로그만 console.warn.
 * - server-only (BFF). client bundle 미포함.
 */

import { env } from "@/lib/env";

export async function emitAuthEvent(
  event: string,
  payload: Record<string, unknown>,
): Promise<void> {
  try {
    const res = await fetch(`${env.NEXT_PUBLIC_API_BASE_URL}/internal/auth/event`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
        "x-internal-auth": env.INTERNAL_AUTH_SECRET,
      },
      body: JSON.stringify({ event, payload }),
    });
    if (!res.ok) {
      console.warn("[emitAuthEvent] HTTP %d for event=%s", res.status, event);
    }
  } catch (e) {
    // 네트워크 실패 — audit log 누락만, 사용자 경험 정상.
    console.warn("[emitAuthEvent] fetch failed:", e instanceof Error ? e.message : String(e));
  }
}
