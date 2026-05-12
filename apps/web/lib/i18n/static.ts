// apps/web/lib/i18n/static.ts
//
// Middleware / proxy.ts 같은 *non-component* context 에서 사용하는 static i18n
// fallback. next-intl/server 의 getTranslations 는 *server component 또는
// server action* 에서만 작동 — middleware 에서 부르면 timing 문제.
//
// 현재 ko.json 만 import → 한국 single-locale fallback. 다국어 추가 시 locale
// detection (req.headers['accept-language'] 등) 으로 분기.

import ko from "./ko.json";

type Json = Record<string, unknown>;

/**
 * dot-path 로 i18n key lookup. 누락 시 path 자체 return (개발자 신호).
 *
 * 예: tStatic('server.proxy.rateLimitedTitle') → '요청이 너무 많아요'
 */
export function tStatic(path: string): string {
  const segments = path.split(".");
  let cur: unknown = ko;
  for (const seg of segments) {
    if (cur && typeof cur === "object" && seg in (cur as Json)) {
      cur = (cur as Json)[seg];
    } else {
      return path;
    }
  }
  return typeof cur === "string" ? cur : path;
}
