/**
 * SP7-i 가 채울 자리 — Sentry SDK 초기화.
 *
 * Next.js 16 표준 — instrumentation.ts 가 server / edge runtime 에서 자동 호출.
 * SP6-foundation 단계: empty register() — 통합 자리만 명시.
 */
export function register(): void {
  // SP7-i 가 추가:
  // import * as Sentry from "@sentry/nextjs";
  // Sentry.init({ dsn: process.env.SENTRY_DSN, ... });
}
