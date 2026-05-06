/**
 * SP9 ADR 0019 — PMTiles Service Worker (transport layer).
 *
 * 역할: 브라우저의 모든 HTTP 요청 중 `/__pmtiles__/<encoded>/{z}/{x}/{y}.pbf` 패턴
 * 만 가로채 PMTiles JS lib 으로 byte-range fetch + tile 추출 → fake `Response` 반환.
 * mapbox-gl 의 standard VectorTileWorkerSource 가 *평범한 vector tile* 처럼 parsing.
 *
 * 본 spike 의 결과 = T3b.4 commit `f055630` 의 메시지에 박제 (worker uncontrolled wall).
 *
 * 본 파일은 *spike 결과 박제 용도* — 다음 세션이 working dir reset 또는 별도
 * revert commit 결정. tsconfig 의 lib 에 `webworker` 가 없어서 별도 type 선언.
 */

import { PMTiles } from "pmtiles";

// SW context types — tsconfig lib 에 "webworker" 없어 ad-hoc 선언.
// biome-ignore lint/style/useNamingConvention: web platform global types
interface ServiceWorkerGlobalScopeStub {
  addEventListener: (type: string, listener: (e: unknown) => void) => void;
  fetch: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>;
  skipWaiting: () => Promise<void>;
  clients: { claim: () => Promise<void> };
}
declare const self: ServiceWorkerGlobalScopeStub;

// Service Worker 첫 로드 시 즉시 통제 (claim 패턴).
self.addEventListener("install", () => {
  self.skipWaiting();
});
self.addEventListener("activate", (event: unknown) => {
  // ExtendableEvent.waitUntil — webworker lib 없는 main tsconfig 에서 ad-hoc cast.
  (event as { waitUntil: (p: Promise<unknown>) => void }).waitUntil(self.clients.claim());
});

// PMTiles 인스턴스 캐시 (SW thread 내). 같은 source URL 의 여러 tile request 가
// 동일 PMTiles 인스턴스 + 동일 byte cache 공유.
const pmtilesCache = new Map<string, PMTiles>();
const MAX_CACHE = 16;

function getPmtiles(url: string): PMTiles {
  let pm = pmtilesCache.get(url);
  if (pm) return pm;
  if (pmtilesCache.size >= MAX_CACHE) {
    const oldest = pmtilesCache.keys().next().value;
    if (oldest) pmtilesCache.delete(oldest);
  }
  pm = new PMTiles(url);
  pmtilesCache.set(url, pm);
  return pm;
}

function decodeBase64Url(s: string): string {
  const pad = s.length % 4 === 0 ? "" : "=".repeat(4 - (s.length % 4));
  return atob(s.replace(/-/g, "+").replace(/_/g, "/") + pad);
}

const PMTILES_URL_RE = /\/__pmtiles__\/([^/]+)\/(\d+)\/(\d+)\/(\d+)\.pbf(?:\?.*)?$/;

self.addEventListener("fetch", (event: unknown) => {
  const fe = event as {
    request: Request;
    respondWith: (r: Promise<Response> | Response) => void;
  };
  const url = fe.request.url;
  const m = url.match(PMTILES_URL_RE);
  if (!m) return;

  // biome-ignore lint/style/noNonNullAssertion: regex matched 4 groups
  const [, encoded, zStr, xStr, yStr] = m as unknown as [string, string, string, string, string];
  const z = Number.parseInt(zStr, 10);
  const x = Number.parseInt(xStr, 10);
  const y = Number.parseInt(yStr, 10);

  fe.respondWith(
    (async () => {
      let realUrl: string;
      try {
        realUrl = decodeURIComponent(decodeBase64Url(encoded));
      } catch {
        return new Response(null, { status: 400, statusText: "bad pmtiles encoding" });
      }

      try {
        const pm = getPmtiles(realUrl);
        const resp = await pm.getZxy(z, x, y);
        if (!resp || resp.data.byteLength === 0) {
          return new Response(null, { status: 204 });
        }
        return new Response(resp.data, {
          status: 200,
          headers: {
            "Content-Type": "application/x-protobuf",
            // Tier A 위생 — tile bytes immutable.
            "Cache-Control": "public, max-age=86400, immutable",
          },
        });
      } catch (e) {
        return new Response(null, {
          status: 500,
          statusText: e instanceof Error ? e.message.slice(0, 200) : "pmtiles error",
        });
      }
    })(),
  );
});
