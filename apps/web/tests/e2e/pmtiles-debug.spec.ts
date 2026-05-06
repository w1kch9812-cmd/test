/**
 * SP9 ADR 0019 spike 검증 — addSourceType + PMTilesSource path.
 * - browser → /pmtiles/parcels.pmtiles (직결 byte-range) 만 일어남, /api/tiles 없음
 * - parcels-fill layer 의 queryRenderedFeatures > 0
 */

import { randomBytes } from "node:crypto";
import { test } from "@playwright/test";
import { Redis } from "ioredis";

const SID_COOKIE_NAME = "sid";

async function plantSession(): Promise<string> {
  const redis = new Redis(process.env.REDIS_URL || "redis://localhost:6379");
  const sid = randomBytes(32).toString("hex");
  const data = {
    sub: "playwright",
    jti: "j",
    role: "Buyer",
    access_token: "f",
    refresh_token: "f",
    id_token: "f",
    exp: Math.floor(Date.now() / 1000) + 3600,
  };
  await redis.set(`session:${sid}`, JSON.stringify(data), "EX", 3600);
  await redis.quit();
  return sid;
}

test("ADR 0019 A2 + SW — VectorTileSource subclass + Service Worker transport", async ({
  page,
  context,
}) => {
  test.setTimeout(120_000);

  const sid = await plantSession();
  await context.addCookies([
    {
      name: SID_COOKIE_NAME,
      value: sid,
      domain: "localhost",
      path: "/",
      httpOnly: true,
      sameSite: "Lax",
    },
  ]);

  const consoleMessages: string[] = [];
  const networkLog: { url: string; status: number; range?: string }[] = [];
  page.on("console", (m) => consoleMessages.push(`[${m.type()}] ${m.text()}`));
  // mapbox-gl worker (Naver SDK 가 만든 web worker) console capture
  context.on("page", (p) => {
    p.on("console", (m) => consoleMessages.push(`[page-console ${m.type()}] ${m.text()}`));
  });
  page.on("worker", (worker) => {
    consoleMessages.push(`[worker created] ${worker.url()}`);
    // Web Worker 의 console.log 는 page.on("console") 에 자동 capture 됨 (Playwright 가)
  });
  page.on("response", (r) => {
    const u = r.url();
    if (
      u.includes("/pmtiles/") ||
      u.includes("/__pmtiles__/") ||
      u.includes("/sw-pmtiles") ||
      u.includes("/api/tiles/")
    ) {
      networkLog.push({
        url: u,
        status: r.status(),
        range: r.request().headers().range,
      });
    }
  });

  await page.goto("http://localhost:3000/listings", { waitUntil: "load", timeout: 60000 });
  await page.waitForTimeout(15000);

  // 부평 panTo + zoom 17 (cached parcels.pmtiles bounds: 126.666~126.780 / 37.381~37.479)
  await page.evaluate(() => {
    const m = (
      window as unknown as {
        __listingMap?: { setCenterGL?: (ll: unknown) => void; setZoomGL?: (z: number) => void };
      }
    ).__listingMap;
    const naverGlobal = (
      window as unknown as {
        naver?: { maps?: { LatLng?: new (lat: number, lng: number) => unknown } };
      }
    ).naver;
    const ll = naverGlobal?.maps?.LatLng ? new naverGlobal.maps.LatLng(37.4, 126.7) : null;
    if (m && ll) {
      m.setCenterGL?.(ll);
      m.setZoomGL?.(17);
    }
  });
  await page.waitForTimeout(8000);

  const result = await page.evaluate(() => {
    const m = (
      window as unknown as {
        __listingMap?: { getMapbox?: () => unknown };
      }
    ).__listingMap;
    const mb = m?.getMapbox?.() as
      | {
          queryRenderedFeatures?: (point?: unknown, options?: { layers?: string[] }) => unknown[];
          getStyle?: () => {
            sources?: Record<string, { type: string }>;
            layers?: Array<{ id: string }>;
          };
        }
      | undefined;
    if (!mb) return { error: "no mb" };
    const style = mb.getStyle?.();
    return {
      sources: style?.sources
        ? Object.entries(style.sources).map(([id, s]) => ({ id, type: s.type }))
        : [],
      layerIds:
        style?.layers?.map((l) => l.id).filter((id) => /parcels|admin|complex/.test(id)) ?? [],
      parcelsFillFeatures:
        mb.queryRenderedFeatures?.(undefined, { layers: ["parcels-fill"] })?.length ?? -1,
    };
  });

  console.log("\n=== addSourceType spike result ===");
  console.log(JSON.stringify(result, null, 2));

  console.log("\n=== Browser console (filtered) ===");
  for (const m of consoleMessages) {
    if (
      /pmtiles|addSourceType|error|warn|sw-register|controllerchange|register|sw-pmtiles/i.test(m)
    )
      console.log(m);
  }

  console.log("\n=== PMTiles + tiles network requests ===");
  console.log(`총 ${networkLog.length} requests`);
  // Direct byte-range = path /pmtiles/, server proxy = /api/tiles/
  const direct = networkLog.filter((r) => r.url.includes("/pmtiles/")).length;
  const proxy = networkLog.filter((r) => r.url.includes("/api/tiles/")).length;
  const withRange = networkLog.filter((r) => r.range).length;
  console.log(
    `  direct (/pmtiles/): ${direct}, server proxy (/api/tiles/): ${proxy}, with Range header: ${withRange}`,
  );
  for (const r of networkLog.slice(0, 8)) {
    console.log(`  ${r.status} ${r.url} (range=${r.range || "(none)"})`);
  }

  await page.screenshot({
    path: "C:/Users/User/Desktop/gongzzang_2/var/sample/spike-a.png",
    fullPage: false,
  });
  console.log("\nscreenshot: var/sample/spike-a.png");
});
