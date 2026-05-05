import type { NextConfig } from "next";
import createNextIntlPlugin from "next-intl/plugin";

const withNextIntl = createNextIntlPlugin("./i18n.ts");

const baseHeaders = [
  { key: "X-Frame-Options", value: "DENY" },
  { key: "X-Content-Type-Options", value: "nosniff" },
  { key: "Referrer-Policy", value: "strict-origin-when-cross-origin" },
  { key: "Permissions-Policy", value: "camera=(), microphone=(), geolocation=()" },
];

// HSTS preload 는 production 전용 — dev HTTP localhost 에 적용 시 Chrome 이 2년간
// HSTS 캐시에 등록되어 개발자가 manually clear 하기 전까지 plain HTTP 차단됨.
const productionHeaders = [
  ...baseHeaders,
  { key: "Strict-Transport-Security", value: "max-age=63072000; includeSubDomains; preload" },
];

const nextConfig: NextConfig = {
  // Naver Maps gl 이 React Strict Mode 의 이중 마운트와 호환 안 됨 (지도 이중 렌더링).
  // Reference: gongzzang-design-lab 의 next.config 패턴 따름.
  reactStrictMode: false,
  typedRoutes: true,
  async headers() {
    const headers = process.env.NODE_ENV === "production" ? productionHeaders : baseHeaders;
    return [{ source: "/(.*)", headers }];
  },
};

export default withNextIntl(nextConfig);
