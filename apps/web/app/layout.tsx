import type { Metadata } from "next";
import { NextIntlClientProvider } from "next-intl";
import { getLocale, getMessages, getTranslations } from "next-intl/server";
import "./globals.css";
import { Toaster } from "@gongzzang/ui";
import { env } from "@/lib/env";
import { QueryProvider } from "@/lib/query";

// Pretendard Variable v1.3.9 — OFL 1.1 (https://github.com/orioncactus/pretendard)
// dynamic-subset CSS 는 unicode-range 단위로 92개 woff2 를 분리 → 브라우저가 실제
// 렌더된 글자에 해당하는 subset 만 자동 다운로드 (한국어 페이지 보통 150-300 KB).
// 4 weight 통째 self-host (3 MB+) → variable axis 1 file × subset = 95% 절감.

export async function generateMetadata(): Promise<Metadata> {
  const t = await getTranslations("meta");
  return {
    title: t("title"),
    description: t("description"),
  };
}

export default async function RootLayout({ children }: { children: React.ReactNode }) {
  const locale = await getLocale();
  const messages = await getMessages();

  return (
    <html lang={locale}>
      <head>
        <link rel="stylesheet" href="/fonts/pretendardvariable-dynamic-subset.css" />
        {/* Naver Maps SDK 는 동기 로드 필수 — gl/clustering 서브모듈이 WebGL 백엔드를
            window.naver.maps 에 등록하려면 첫 Map 생성 시점에 이미 로드되어 있어야 함.
            dynamic script injection 으로는 gl: true 가 활성화되지 않음. */}
        {/* eslint-disable-next-line @next/next/no-sync-scripts */}
        <script
          type="text/javascript"
          src={`https://oapi.map.naver.com/openapi/v3/maps.js?ncpKeyId=${env.NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID}&submodules=gl,clustering`}
        />
      </head>
      <body>
        <NextIntlClientProvider messages={messages}>
          <QueryProvider>
            {children}
            <Toaster />
          </QueryProvider>
        </NextIntlClientProvider>
      </body>
    </html>
  );
}
