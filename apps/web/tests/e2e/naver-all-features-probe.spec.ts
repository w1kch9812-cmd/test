/**
 * SP9 ADR 0020 follow-up — Naver gl SDK 의 *모든 layer type* (polygon 외) 식별자 전수조사.
 *
 * ADR 0020 의 기존 probe (naver-polygons-probe.spec.ts) 는 fill/fill-extrusion 만 dump.
 * 본 spec 은 symbol/line/circle/raster/heatmap 까지 sweep — 산업용 부동산 platform 의
 * *모든 식별 needs* (지하철역, 학교, 도로, POI 등) 의 stable id / properties schema 검증.
 *
 * 검증 대상:
 *  1. 전체 layer 의 type / source / sourceLayer / paint / layout 카탈로그
 *  2. type 별 layer 분포 (fill, line, symbol, circle, raster, heatmap, fill-extrusion, background)
 *  3. 다양한 viewport 에서 queryRenderedFeatures dump
 *     - 강남 zoom 17 (산업/오피스 + POI 밀도)
 *     - 부평 zoom 17 (공장/창고 밀도)
 *     - 서울역 zoom 16 (지하철 station POI)
 *  4. 각 type 별로 *유의미한 properties keys + sample values* 박제
 *  5. setFeatureState 작동 가능 후보 식별 (feature.id 보유 layer)
 *  6. naver.maps.CadastralLayer enable 비교 (별도 page 로드)
 */

import { randomBytes } from "node:crypto";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { test } from "@playwright/test";
import { Redis } from "ioredis";

const SID_COOKIE_NAME = "sid";
const OUT_DIR = "var/sample";

interface ViewportSpec {
  name: string;
  lat: number;
  lng: number;
  zoom: number;
}

const VIEWPORTS: ViewportSpec[] = [
  { name: "gangnam", lat: 37.4979, lng: 127.0276, zoom: 17 },
  { name: "bupyeong", lat: 37.4, lng: 126.7, zoom: 17 },
  { name: "seoul-station", lat: 37.5547, lng: 126.9707, zoom: 16 },
];

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

function writeJson(filename: string, data: unknown): void {
  const path = `${OUT_DIR}/${filename}`;
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, JSON.stringify(data, null, 2));
  const size = (JSON.stringify(data).length / 1024).toFixed(1);
  console.log(`written ${path} (${size} KB)`);
}

test("Naver gl SDK 모든 layer type 식별자 전수조사 (multi-viewport)", async ({ page, context }) => {
  test.setTimeout(300_000);
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

  await page.goto("http://localhost:3000/listings", { waitUntil: "load", timeout: 60000 });
  await page.waitForTimeout(15000);

  // ===== Pass 1: 전체 layer 카탈로그 (viewport 무관) =====
  const catalog = await page.evaluate(() => {
    const m = (
      window as unknown as {
        __listingMap?: { getMapbox?: () => unknown };
      }
    ).__listingMap;
    // biome-ignore lint/suspicious/noExplicitAny: dynamic mapbox
    const mb = m?.getMapbox?.() as any;
    if (!mb) return { error: "no mb" };

    const style = mb.getStyle?.();
    if (!style) return { error: "no style" };

    // biome-ignore lint/suspicious/noExplicitAny: layer spec
    const layers = (style.layers ?? []).map((l: any) => ({
      id: l.id,
      type: l.type,
      source: l.source,
      sourceLayer: l["source-layer"],
      minzoom: l.minzoom,
      maxzoom: l.maxzoom,
      filter: l.filter,
      paintKeys: l.paint ? Object.keys(l.paint) : [],
      layoutKeys: l.layout ? Object.keys(l.layout) : [],
    }));

    const byType: Record<string, number> = {};
    // biome-ignore lint/suspicious/noExplicitAny: spec
    for (const l of layers) byType[l.type] = (byType[l.type] ?? 0) + 1;

    const sources = Object.entries(style.sources ?? {}).map(([id, s]) => ({
      id,
      // biome-ignore lint/suspicious/noExplicitAny: source spec
      ...(s as any),
    }));

    return {
      layerCount: layers.length,
      sourceCount: sources.length,
      layerCountByType: byType,
      layers,
      sources,
    };
  });
  writeJson("naver-all-features-catalog.json", catalog);

  // ===== Pass 2: viewport 별 dump =====
  for (const vp of VIEWPORTS) {
    await page.evaluate((vpArg) => {
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
      const ll = naverGlobal?.maps?.LatLng
        ? new naverGlobal.maps.LatLng(vpArg.lat, vpArg.lng)
        : null;
      if (m && ll) {
        m.setCenterGL?.(ll);
        m.setZoomGL?.(vpArg.zoom);
      }
    }, vp);
    await page.waitForTimeout(5000);

    const dump = await page.evaluate((vpArg) => {
      const m = (
        window as unknown as {
          __listingMap?: { getMapbox?: () => unknown };
        }
      ).__listingMap;
      // biome-ignore lint/suspicious/noExplicitAny: dynamic mapbox
      const mb = m?.getMapbox?.() as any;
      if (!mb) return { error: "no mb" };

      // ALL features in viewport — type 무관
      const allFeatures = mb.queryRenderedFeatures?.() ?? [];

      // type 별 layer.id grouping
      // biome-ignore lint/suspicious/noExplicitAny: feature
      const byType: Record<string, any[]> = {};
      // biome-ignore lint/suspicious/noExplicitAny: feature
      for (const f of allFeatures as any[]) {
        const t = f.layer?.type ?? "unknown";
        if (!byType[t]) byType[t] = [];
        byType[t].push(f);
      }

      // type 별로 sample (각 type 의 첫 30개 feature 의 properties keys 분석)
      // biome-ignore lint/suspicious/noExplicitAny: feature
      const sampleByType: Record<string, unknown> = {};
      for (const [t, feats] of Object.entries(byType)) {
        const grouped: Record<string, { count: number; sample: unknown }> = {};
        // biome-ignore lint/suspicious/noExplicitAny: feature
        for (const f of feats as any[]) {
          const k = `${f.source}/${f.sourceLayer ?? "(none)"}/${f.layer?.id}`;
          if (!grouped[k]) {
            grouped[k] = {
              count: 0,
              sample: {
                hasId: f.id !== undefined && f.id !== null,
                idType: typeof f.id,
                idSample: f.id,
                geometryType: f.geometry?.type,
                propertyKeys: f.properties ? Object.keys(f.properties).slice(0, 30) : [],
                properties: f.properties,
              },
            };
          }
          grouped[k].count += 1;
        }
        sampleByType[t] = {
          totalFeatures: (feats as unknown[]).length,
          uniqueLayerSourceCombos: Object.keys(grouped).length,
          grouped,
        };
      }

      // setFeatureState 테스트 — feature.id 가 있는 *모든 type* 의 첫 1개 layer
      const stateTests: Array<Record<string, unknown>> = [];
      for (const [t, feats] of Object.entries(byType)) {
        // biome-ignore lint/suspicious/noExplicitAny: feature
        const idFeats = (feats as any[]).filter(
          // biome-ignore lint/suspicious/noExplicitAny: feature
          (f: any) => f.id !== undefined && f.id !== null,
        );
        if (idFeats.length === 0) {
          stateTests.push({ type: t, idCount: 0, note: "no feature.id present" });
          continue;
        }
        const f = idFeats[0];
        try {
          mb.setFeatureState?.(
            { source: f.source, sourceLayer: f.sourceLayer, id: f.id },
            { __probe: true },
          );
          const state = mb.getFeatureState?.({
            source: f.source,
            sourceLayer: f.sourceLayer,
            id: f.id,
          });
          stateTests.push({
            type: t,
            source: f.source,
            sourceLayer: f.sourceLayer,
            layer: f.layer?.id,
            id: f.id,
            stateAfterSet: state,
            ok: state?.__probe === true,
          });
          mb.removeFeatureState?.({ source: f.source, sourceLayer: f.sourceLayer, id: f.id });
        } catch (e) {
          stateTests.push({
            type: t,
            source: f.source,
            layer: f.layer?.id,
            error: (e as Error).message,
          });
        }
      }

      return {
        viewport: vpArg,
        totalFeaturesInViewport: allFeatures.length,
        featuresByType: Object.fromEntries(
          Object.entries(byType).map(([t, feats]) => [t, (feats as unknown[]).length]),
        ),
        sampleByType,
        stateTests,
      };
    }, vp);

    writeJson(`naver-all-features-${vp.name}.json`, dump);
    console.log(
      `\n=== ${vp.name} (z${vp.zoom}) — ${dump.totalFeaturesInViewport} features (${Object.entries(
        dump.featuresByType ?? {},
      )
        .map(([t, c]) => `${t}:${c}`)
        .join(", ")}) ===`,
    );
    for (const s of dump.stateTests ?? []) {
      console.log(JSON.stringify(s));
    }
  }
});

/**
 * naver.maps.CadastralLayer enable 후 dump — Naver 가 *별도 옵션* 으로 cadastral overlay 제공.
 * raster / vector / 클릭 식별 path 비교용. 우리 PMTiles 와 별개.
 */
test("Naver CadastralLayer enable + 비교 dump", async ({ page, context }) => {
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

  await page.goto("http://localhost:3000/listings", { waitUntil: "load", timeout: 60000 });
  await page.waitForTimeout(15000);

  const result = await page.evaluate(() => {
    const m = (
      window as unknown as {
        __listingMap?: {
          getMapbox?: () => unknown;
          // biome-ignore lint/suspicious/noExplicitAny: naver Map dynamic
        } & any;
      }
    ).__listingMap;
    const naverGlobal = (
      window as unknown as {
        naver?: {
          maps?: {
            // biome-ignore lint/suspicious/noExplicitAny: naver SDK
            CadastralLayer?: any;
            LatLng?: new (lat: number, lng: number) => unknown;
          };
        };
      }
    ).naver;

    if (!m || !naverGlobal?.maps?.CadastralLayer) {
      return {
        cadastralAvailable: false,
        note: "naver.maps.CadastralLayer 미노출 — submodules=cadastral 필요?",
      };
    }

    const cadastral = new naverGlobal.maps.CadastralLayer();
    cadastral.setMap(m);

    return {
      cadastralAvailable: true,
      cadastralCtor: typeof naverGlobal.maps.CadastralLayer,
      cadastralKeys: Object.keys(cadastral),
      // biome-ignore lint/suspicious/noExplicitAny: dynamic
      cadastralPrototype: Object.getOwnPropertyNames(Object.getPrototypeOf(cadastral) as any),
      // biome-ignore lint/suspicious/noExplicitAny: dynamic
      hasGetMap: typeof (cadastral as any).getMap === "function",
      // biome-ignore lint/suspicious/noExplicitAny: dynamic
      mapAfterSet: !!(cadastral as any).getMap?.(),
    };
  });
  writeJson("naver-cadastral-layer.json", result);
  console.log(JSON.stringify(result, null, 2));
});
