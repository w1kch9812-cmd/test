import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { expect, type Page, type TestInfo, test } from "@playwright/test";
import { plantAuthenticatedSession } from "../e2e/auth";

const OUT_DIR = "var/sample";
const LISTINGS_URL = "http://localhost:3000/listings";
const MAP_BOOT_TIMEOUT_MS = 15_000;

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

async function writeProbeJson(
  testInfo: TestInfo,
  filename: string,
  data: unknown,
): Promise<string> {
  const path = join(OUT_DIR, filename);
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, JSON.stringify(data, null, 2));
  await testInfo.attach(filename, { path, contentType: "application/json" });
  return path;
}

async function openAuthenticatedListings(page: Page) {
  await page.goto(LISTINGS_URL, { waitUntil: "load", timeout: 60_000 });
  await page.waitForTimeout(MAP_BOOT_TIMEOUT_MS);
}

async function setNaverGlViewport(page: Page, viewport: ViewportSpec): Promise<void> {
  await page.evaluate((nextViewport) => {
    const root = window as unknown as {
      __listingMap?: {
        setCenterGL?: (latLng: unknown) => void;
        setZoomGL?: (zoom: number) => void;
      };
      naver?: { maps?: { LatLng?: new (lat: number, lng: number) => unknown } };
    };
    const LatLng = root.naver?.maps?.LatLng;
    if (!root.__listingMap || !LatLng) return;
    root.__listingMap.setCenterGL?.(new LatLng(nextViewport.lat, nextViewport.lng));
    root.__listingMap.setZoomGL?.(nextViewport.zoom);
  }, viewport);
  await page.waitForTimeout(5_000);
}

async function captureStyleCatalog(page: Page): Promise<unknown> {
  return page.evaluate(() => {
    type Dict = Record<string, unknown>;

    function asRecord(value: unknown): Dict {
      return typeof value === "object" && value !== null ? (value as Dict) : {};
    }

    function asArray(value: unknown): unknown[] {
      return Array.isArray(value) ? value : [];
    }

    function stringValue(value: unknown): string | undefined {
      return typeof value === "string" ? value : undefined;
    }

    function keysOf(value: unknown): string[] {
      return Object.keys(asRecord(value));
    }

    function getMapbox() {
      const root = window as unknown as { __listingMap?: { getMapbox?: () => unknown } };
      return asRecord(root.__listingMap?.getMapbox?.());
    }

    const style = asRecord((getMapbox().getStyle as (() => unknown) | undefined)?.());
    const layers = asArray(style.layers).map((layer) => {
      const item = asRecord(layer);
      return {
        id: stringValue(item.id),
        type: stringValue(item.type),
        source: stringValue(item.source),
        sourceLayer: stringValue(item["source-layer"]),
        minzoom: item.minzoom,
        maxzoom: item.maxzoom,
        filter: item.filter,
        paintKeys: keysOf(item.paint),
        layoutKeys: keysOf(item.layout),
      };
    });

    const layerCountByType = layers.reduce<Record<string, number>>((acc, layer) => {
      const type = layer.type ?? "unknown";
      acc[type] = (acc[type] ?? 0) + 1;
      return acc;
    }, {});

    const sources = Object.entries(asRecord(style.sources)).map(([id, source]) => ({
      id,
      ...asRecord(source),
    }));

    return {
      layerCount: layers.length,
      sourceCount: sources.length,
      layerCountByType,
      layers,
      sources,
    };
  });
}

async function captureViewportDump(page: Page, viewport: ViewportSpec): Promise<unknown> {
  return page.evaluate((currentViewport) => {
    type Dict = Record<string, unknown>;

    interface FeatureSummary {
      id: unknown;
      source: unknown;
      sourceLayer: unknown;
      layer: unknown;
      layerType: string;
      geometryType: unknown;
      propertyKeys: string[];
      properties: unknown;
    }

    function asRecord(value: unknown): Dict {
      return typeof value === "object" && value !== null ? (value as Dict) : {};
    }

    function asArray(value: unknown): unknown[] {
      return Array.isArray(value) ? value : [];
    }

    function layerTypeOf(feature: unknown): string {
      const layer = asRecord(asRecord(feature).layer);
      return typeof layer.type === "string" ? layer.type : "unknown";
    }

    function summarizeFeature(feature: unknown): FeatureSummary {
      const item = asRecord(feature);
      const layer = asRecord(item.layer);
      const geometry = asRecord(item.geometry);
      const properties = asRecord(item.properties);
      return {
        id: item.id,
        source: item.source,
        sourceLayer: item.sourceLayer,
        layer: layer.id,
        layerType: layerTypeOf(feature),
        geometryType: geometry.type,
        propertyKeys: Object.keys(properties).slice(0, 30),
        properties,
      };
    }

    function groupFeatureSamples(features: unknown[]): Record<string, unknown> {
      return features.reduce<Record<string, unknown>>((groups, feature) => {
        const summary = summarizeFeature(feature);
        const group = groups[summary.layerType] as { count: number; samples: FeatureSummary[] };
        groups[summary.layerType] = group
          ? { count: group.count + 1, samples: group.samples }
          : { count: 1, samples: [summary] };
        return groups;
      }, {});
    }

    function probeFeatureState(features: unknown[], mapbox: Dict): unknown[] {
      const setFeatureState = mapbox.setFeatureState as
        | ((target: Dict, state: Dict) => void)
        | undefined;
      const getFeatureState = mapbox.getFeatureState as ((target: Dict) => Dict) | undefined;
      const removeFeatureState = mapbox.removeFeatureState as ((target: Dict) => void) | undefined;
      if (!setFeatureState || !getFeatureState)
        return [{ ok: false, error: "feature-state API unavailable" }];

      return features
        .map(summarizeFeature)
        .filter((feature) => feature.id !== undefined && feature.id !== null)
        .slice(0, 8)
        .map((feature) => {
          const target = {
            source: feature.source,
            sourceLayer: feature.sourceLayer,
            id: feature.id,
          };
          setFeatureState(target, { __probe: true });
          const state = getFeatureState(target);
          removeFeatureState?.(target);
          return {
            type: feature.layerType,
            source: feature.source,
            sourceLayer: feature.sourceLayer,
            layer: feature.layer,
            id: feature.id,
            ok: state.__probe === true,
          };
        });
    }

    const root = window as unknown as { __listingMap?: { getMapbox?: () => unknown } };
    const mapbox = asRecord(root.__listingMap?.getMapbox?.());
    const queryRenderedFeatures = mapbox.queryRenderedFeatures as (() => unknown) | undefined;
    const features = asArray(queryRenderedFeatures?.());

    return {
      viewport: currentViewport,
      totalFeaturesInViewport: features.length,
      featuresByType: features.reduce<Record<string, number>>((acc, feature) => {
        const type = layerTypeOf(feature);
        acc[type] = (acc[type] ?? 0) + 1;
        return acc;
      }, {}),
      sampleByType: groupFeatureSamples(features),
      stateTests: probeFeatureState(features, mapbox),
    };
  }, viewport);
}

async function captureCadastralLayer(page: Page): Promise<unknown> {
  return page.evaluate(() => {
    type Dict = Record<string, unknown>;
    type CadastralLayer = { setMap?: (map: unknown) => void; getMap?: () => unknown };
    type CadastralLayerCtor = new () => CadastralLayer;

    const root = window as unknown as {
      __listingMap?: unknown;
      naver?: { maps?: { CadastralLayer?: CadastralLayerCtor } };
    };
    const CadastralLayer = root.naver?.maps?.CadastralLayer;

    if (!root.__listingMap || !CadastralLayer) {
      return {
        cadastralAvailable: false,
        note: "naver.maps.CadastralLayer unavailable",
      };
    }

    const cadastral = new CadastralLayer();
    cadastral.setMap?.(root.__listingMap);
    const prototype = Object.getPrototypeOf(cadastral) as Dict;

    return {
      cadastralAvailable: true,
      cadastralCtor: typeof CadastralLayer,
      cadastralKeys: Object.keys(cadastral),
      cadastralPrototype: Object.getOwnPropertyNames(prototype),
      hasGetMap: typeof cadastral.getMap === "function",
      mapAfterSet: Boolean(cadastral.getMap?.()),
    };
  });
}

test.describe("Naver SDK probes", () => {
  test.beforeEach(async ({ context }) => {
    await plantAuthenticatedSession(context);
  });

  test("catalogs style layers and rendered feature-state support", async ({ page }, testInfo) => {
    test.setTimeout(300_000);
    await openAuthenticatedListings(page);

    await writeProbeJson(
      testInfo,
      "naver-all-features-catalog.json",
      await captureStyleCatalog(page),
    );

    for (const viewport of VIEWPORTS) {
      await setNaverGlViewport(page, viewport);
      await writeProbeJson(
        testInfo,
        `naver-all-features-${viewport.name}.json`,
        await captureViewportDump(page, viewport),
      );
    }
  });

  test("catalogs cadastral layer availability", async ({ page }, testInfo) => {
    test.setTimeout(120_000);
    await openAuthenticatedListings(page);
    const result = await captureCadastralLayer(page);
    await writeProbeJson(testInfo, "naver-cadastral-layer.json", result);
    expect(result).toBeDefined();
  });
});
