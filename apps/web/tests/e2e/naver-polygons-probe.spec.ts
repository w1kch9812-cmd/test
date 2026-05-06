/**
 * SP9 ADR 0018/0019 후속 — Naver 의 vector 들 특정 가능성 전수조사.
 *
 * 검사 항목:
 * 1. 모든 layer 의 type / source / source-layer / paint dump
 * 2. polygon type (fill / fill-extrusion) 필터
 * 3. 부평 zoom 17 viewport 에서 queryRenderedFeatures
 * 4. 각 feature 의 id, properties 확인 (stable id 보유 여부)
 * 5. setFeatureState 호출 후 render 변화 측정
 *
 * 출력: var/sample/naver-polygons.json (full dump for analysis)
 */

import { randomBytes } from "node:crypto";
import { writeFileSync } from "node:fs";
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

test("Naver 폴리곤 vector 특정 가능성 전수조사", async ({ page, context }) => {
  test.setTimeout(180_000);
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

  // 부평 panTo + zoom 17 — 건물/도로/POI 가 다양하게 보이는 viewport.
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
  await page.waitForTimeout(5000);

  const dump = await page.evaluate(() => {
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

    // 1. 모든 layer 의 type / source / source-layer / paint 첫 1단 keys.
    const layers = (style.layers ?? []).map(
      // biome-ignore lint/suspicious/noExplicitAny: layer spec
      (l: any) => ({
        id: l.id,
        type: l.type,
        source: l.source,
        sourceLayer: l["source-layer"],
        minzoom: l.minzoom,
        maxzoom: l.maxzoom,
        paintKeys: l.paint ? Object.keys(l.paint) : [],
        layoutKeys: l.layout ? Object.keys(l.layout) : [],
      }),
    );

    // 2. polygon type 만 필터.
    const polygonLayers = layers.filter(
      // biome-ignore lint/suspicious/noExplicitAny: spec
      (l: any) => l.type === "fill" || l.type === "fill-extrusion",
    );

    // 3. 모든 source 정보 (type / url 등).
    const sources = Object.entries(style.sources ?? {}).map(([id, s]) => ({
      id,
      // biome-ignore lint/suspicious/noExplicitAny: source spec
      ...(s as any),
    }));

    // 4. viewport 전체 에서 queryRenderedFeatures (point 없이 = 전체 viewport).
    //    point 지정 X = viewport 안의 모든 visible feature.
    const polygonLayerIds = polygonLayers.map(
      // biome-ignore lint/suspicious/noExplicitAny: spec
      (l: any) => l.id,
    );
    const polygonFeatures =
      mb.queryRenderedFeatures?.(undefined, { layers: polygonLayerIds }) ?? [];
    const allFeatures = mb.queryRenderedFeatures?.() ?? [];

    // 6. feature 별 id / properties / source / sourceLayer dump.
    const featureSummary = polygonFeatures.slice(0, 30).map(
      // biome-ignore lint/suspicious/noExplicitAny: feature
      (f: any) => ({
        id: f.id,
        source: f.source,
        sourceLayer: f.sourceLayer,
        layer: f.layer?.id,
        type: f.geometry?.type,
        propertyKeys: f.properties ? Object.keys(f.properties) : [],
        properties: f.properties,
      }),
    );

    // 7. setFeatureState 시도 — Naver 의 첫 polygon layer 에.
    const stateTests: Array<Record<string, unknown>> = [];
    for (const f of polygonFeatures.slice(0, 3)) {
      try {
        if (f.id === undefined || f.id === null) {
          stateTests.push({ source: f.source, layer: f.layer?.id, error: "feature.id 없음" });
          continue;
        }
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
          source: f.source,
          layer: f.layer?.id,
          id: f.id,
          stateAfterSet: state,
          ok: state?.__probe === true,
        });
        // cleanup
        mb.removeFeatureState?.({ source: f.source, sourceLayer: f.sourceLayer, id: f.id });
      } catch (e) {
        stateTests.push({
          source: f.source,
          layer: f.layer?.id,
          error: (e as Error).message,
        });
      }
    }

    // group features by source / sourceLayer
    const bySource: Record<string, number> = {};
    for (const f of polygonFeatures) {
      const k = `${f.source}/${f.sourceLayer ?? "(none)"}`;
      bySource[k] = (bySource[k] ?? 0) + 1;
    }

    return {
      layerCount: layers.length,
      polygonLayerCount: polygonLayers.length,
      sourceCount: sources.length,
      sources,
      polygonLayers,
      allFeaturesInViewport: allFeatures.length,
      polygonFeaturesInViewport: polygonFeatures.length,
      polygonFeaturesGroupedBySource: bySource,
      featureSummary,
      stateTests,
    };
  });

  const out = "C:/Users/User/Desktop/gongzzang_2/var/sample/naver-polygons.json";
  writeFileSync(out, JSON.stringify(dump, null, 2));
  console.log(`\n=== written ${out} (${(JSON.stringify(dump).length / 1024).toFixed(1)} KB) ===`);
  console.log(
    `layers: ${dump.layerCount}, polygons: ${dump.polygonLayerCount}, sources: ${dump.sourceCount}, features in viewport (polygon): ${dump.polygonFeaturesInViewport}`,
  );
  console.log("\n=== setFeatureState 테스트 ===");
  for (const s of dump.stateTests ?? []) console.log(JSON.stringify(s));
});
