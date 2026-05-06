/**
 * SP9 T3b.2 디버그 — Redis 에 mock 세션 직접 박아서 listings 페이지 들어간 후
 * PMTiles 폴리곤 layer 가 실제 등록 / 렌더되는지 검사.
 */

import { randomBytes } from "node:crypto";
import { test } from "@playwright/test";
import { Redis } from "ioredis";

const SID_COOKIE_NAME = "sid";

async function plantSession(): Promise<string> {
  const redis = new Redis(process.env.REDIS_URL || "redis://localhost:6379");
  const sid = randomBytes(32).toString("hex");
  const data = {
    sub: "playwright-debug",
    jti: "pwd-jti",
    role: "Buyer",
    access_token: "fake-at",
    refresh_token: "fake-rt",
    id_token: "fake-id",
    exp: Math.floor(Date.now() / 1000) + 3600,
  };
  await redis.set(`session:${sid}`, JSON.stringify(data), "EX", 3600);
  await redis.quit();
  return sid;
}

test("listings page polygon layer inspection", async ({ page, context }) => {
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
  const pageErrors: string[] = [];
  const pmtilesRequests: { url: string; status: number; range?: string }[] = [];

  page.on("console", (m) => consoleMessages.push(`[${m.type()}] ${m.text()}`));
  page.on("pageerror", (e) => pageErrors.push(`${e.message}`));
  page.on("response", (r) => {
    const u = r.url();
    if (u.includes("pmtiles") || u.includes("/api/tiles/")) {
      pmtilesRequests.push({
        url: u,
        status: r.status(),
        range: r.request().headers().range,
      });
    }
  });

  // networkidle 안 씀 — 401 호출이 무한 반복돼 idle 도달 안 함.
  await page.goto("http://localhost:3000/listings", { waitUntil: "load", timeout: 60000 });

  // 페이지 로드 + Naver SDK + PMTiles 로딩 시간 (headed 는 더 느림).
  await page.waitForTimeout(15000);

  // 인천 부평 (T3b.2 transitional — design-lab cached parcels.pmtiles 가 부평 일대만 cover).
  // 우리 ETL 로 진짜 강남 필지 빌드되면 좌표 강남으로 복귀.
  await page.evaluate(() => {
    const m = (window as unknown as { __listingMap?: unknown }).__listingMap as
      | {
          setCenterGL?: (ll: unknown) => void;
          setZoomGL?: (z: number) => void;
          setCenter?: (ll: unknown) => void;
          setZoom?: (z: number) => void;
        }
      | undefined;
    if (!m) return;
    const naverGlobal = (
      window as unknown as {
        naver?: { maps?: { LatLng?: new (lat: number, lng: number) => unknown } };
      }
    ).naver;
    // pmtiles probe 로 확인된 실제 데이터 위치: z=14/13957/6354 → 부평 남단 (37.383, 126.665).
    const ll = naverGlobal?.maps?.LatLng ? new naverGlobal.maps.LatLng(37.4, 126.7) : null;
    if (ll) {
      if (typeof m.setCenterGL === "function") m.setCenterGL(ll);
      else m.setCenter?.(ll);
    }
    if (typeof m.setZoomGL === "function") m.setZoomGL(17);
    else m.setZoom?.(17);
  });
  await page.waitForTimeout(5000);

  // 재귀 inspect — prototype chain 포함, depth 4 까지, mapbox-gl 공개 API
  // (addSource/addLayer/getStyle/queryRenderedFeatures) 를 가진 *모든* 객체를 찾음.
  const probe = await page.evaluate(() => {
    // biome-ignore lint/suspicious/noExplicitAny: debug
    const w = window as any;
    const naverExists = typeof w.naver === "object" && typeof w.naver.maps === "object";
    const map = w.__listingMap;
    if (!map) return { error: "__listingMap missing" };

    const MAPBOX_API = [
      "addSource",
      "addLayer",
      "getStyle",
      "queryRenderedFeatures",
      "addProtocol",
    ];

    // own + prototype chain 의 모든 property 이름 dump.
    function allProps(obj: object): string[] {
      const out = new Set<string>();
      let cur: object | null = obj;
      while (cur && cur !== Object.prototype) {
        for (const k of Object.getOwnPropertyNames(cur)) out.add(k);
        cur = Object.getPrototypeOf(cur);
      }
      return [...out];
    }

    // 한 객체가 mapbox-gl 공개 API 를 메서드로 갖고 있는지.
    function looksMapbox(obj: unknown): { hits: string[]; ctor?: string } {
      if (!obj || (typeof obj !== "object" && typeof obj !== "function")) return { hits: [] };
      const hits: string[] = [];
      for (const m of MAPBOX_API) {
        // biome-ignore lint/suspicious/noExplicitAny: dynamic
        if (typeof (obj as any)[m] === "function") hits.push(m);
      }
      const ctor = (obj as { constructor?: { name?: string } }).constructor?.name;
      return { hits, ctor };
    }

    type Hit = { path: string; ctor?: string; hits: string[] };
    const hits: Hit[] = [];
    const seen = new WeakSet();

    function walk(obj: unknown, path: string, depth: number) {
      if (!obj || depth > 4) return;
      if (typeof obj !== "object" && typeof obj !== "function") return;
      if (seen.has(obj as object)) return;
      seen.add(obj as object);

      const lm = looksMapbox(obj);
      if (lm.hits.length > 0) {
        hits.push({ path, ctor: lm.ctor, hits: lm.hits });
      }

      // own props 만 (prototype chain 은 method probe 만 — 객체 그래프 walking 은 own props 로 충분)
      for (const k of Object.getOwnPropertyNames(obj)) {
        if (k.startsWith("$") || k === "constructor") continue;
        let v: unknown;
        try {
          // biome-ignore lint/suspicious/noExplicitAny: dynamic
          v = (obj as any)[k];
        } catch {
          continue;
        }
        if (v && (typeof v === "object" || typeof v === "function")) {
          walk(v, `${path}.${k}`, depth + 1);
        }
      }
    }

    walk(map, "map", 0);

    // 글로벌도 함께 search.
    const globalCandidates = ["mapboxgl", "maplibregl", "mapbox", "_mapboxgl"];
    const globalHits: Record<string, { hits: string[]; ctor?: string }> = {};
    for (const g of globalCandidates) {
      // biome-ignore lint/suspicious/noExplicitAny: dynamic
      if ((w as any)[g]) globalHits[g] = looksMapbox((w as any)[g]);
    }

    // map 의 모든 own + prototype property names (full, 슬라이싱 X).
    const mapAllProps = allProps(map);

    return {
      naverExists,
      mapCtor: map.constructor?.name ?? "(none)",
      mapAllPropsCount: mapAllProps.length,
      mapAllProps,
      hits,
      globalHits,
    };
  });

  console.log("\n========== Naver Map probe ==========");
  console.log(JSON.stringify(probe, null, 2));

  console.log("\n========== Browser console (last 40) ==========");
  for (const m of consoleMessages.slice(-40)) console.log(m);

  console.log("\n========== Page errors ==========");
  for (const e of pageErrors) console.log(e);

  console.log("\n========== PMTiles network requests ==========");
  for (const r of pmtilesRequests)
    console.log(`  ${r.status} ${r.url} (range=${r.range || "(none)"})`);

  // mapbox-gl 버전 + addProtocol 노출 여부 확정.
  const mbInfo = await page.evaluate(() => {
    // biome-ignore lint/suspicious/noExplicitAny: dev-only debug
    const m = (window as any).__listingMap;
    const mb = m?.getMapbox?.();
    if (!mb) return { error: "getMapbox null" };
    const ctor = mb.constructor;
    const protoMethods = Object.getOwnPropertyNames(ctor.prototype || {})
      .filter((k) => typeof ctor.prototype[k] === "function")
      .slice(0, 80);
    const staticMethods = Object.getOwnPropertyNames(ctor).filter(
      (k) => typeof ctor[k] === "function",
    );
    return {
      ctor: ctor.name,
      version: mb.version || ctor.version || mb.constructor.version,
      hasAddProtocol: typeof ctor.addProtocol === "function",
      hasInstanceAddProtocol: typeof mb.addProtocol === "function",
      // PMTiles protocol 등록 후보 위치들
      mapboxglGlobal: typeof (window as unknown as Record<string, unknown>).mapboxgl,
      protoMethods,
      staticMethods,
      // sources/layers 현재 상태 — addSource 가 호출됐는지 검증
      currentSources: Object.keys(mb.getStyle?.()?.sources ?? {}),
      currentLayerIds: (mb.getStyle?.()?.layers ?? []).map((l: { id: string }) => l.id),
    };
  });
  console.log("\n========== mapbox-gl version + protocol ==========");
  console.log(JSON.stringify(mbInfo, null, 2));

  // queryRenderedFeatures 로 폴리곤 layer 가 실제 그려졌는지.
  const features = await page.evaluate(() => {
    // biome-ignore lint/suspicious/noExplicitAny: dev-only debug
    const m = (window as any).__listingMap;
    try {
      // map 자체에도 queryRenderedFeatures 가 있고, getMapbox().queryRenderedFeatures 도 있음.
      const mb = m?.getMapbox?.();
      if (mb && typeof mb.queryRenderedFeatures === "function") {
        const all = mb.queryRenderedFeatures();
        const fill = mb.queryRenderedFeatures(undefined, { layers: ["parcels-fill"] });
        return { source: "mb", total: all.length, parcelsFill: fill.length };
      }
      if (m && typeof m.queryRenderedFeatures === "function") {
        const all = m.queryRenderedFeatures();
        return { source: "map", total: all.length, parcelsFill: -1 };
      }
    } catch (e) {
      return { error: String(e) };
    }
    return { error: "no queryRenderedFeatures" };
  });
  console.log("\n========== queryRenderedFeatures ==========");
  console.log(JSON.stringify(features, null, 2));

  await page.screenshot({
    path: "C:/Users/User/Desktop/gongzzang_2/var/sample/listings-page.png",
    fullPage: false,
  });
  console.log("\nscreenshot: var/sample/listings-page.png");
});
