/**
 * SP9 ADR 0019 — PMTilesSource via VectorTileSource subclass.
 *
 * *완전 표준* mapbox-gl plugin pattern + Service Worker transport:
 * - main thread 의 PMTilesSource 가 VectorTileSource subclass
 * - `tiles: ["/__pmtiles__/<encoded-url>/{z}/{x}/{y}.pbf"]` — Service Worker 가 가로챔
 * - `load()` override — TileJSON fetch 대신 PMTiles header (metadata)
 * - `loadTile()` override 0 — parent VectorTileSource 의 표준 path 그대로
 *
 * Service Worker (lib/workers/sw-pmtiles-src.ts → public/sw-pmtiles.js) 가
 * `/__pmtiles__/` URL 가로채 PMTiles JS lib 으로 raw .pbf bytes 반환.
 * mapbox-gl 의 standard VectorTileWorkerSource 가 그 bytes parsing — 100% 표준.
 *
 * Blob URL trick / workerSourceURL / monkey-patch 모두 0.
 */

import { PMTiles } from "pmtiles";

interface Evented {
  fire(event: { type: string; [k: string]: unknown }): unknown;
  on?(type: string, listener: (e: unknown) => void): unknown;
  setEventedParent?(parent: Evented | null, data?: Record<string, unknown>): unknown;
}
interface Dispatcher {
  getActor(): unknown;
}
interface MapboxStyle {
  constructor: { getSourceType: (name: string) => unknown };
}
interface MapboxMap {
  style?: MapboxStyle;
}

export interface PMTilesSourceSpec {
  type: "pmtiles";
  url: string;
  promoteId?: string;
  minzoom?: number;
  maxzoom?: number;
  attribution?: string;
}

// biome-ignore lint/suspicious/noExplicitAny: mapbox-gl runtime class
type SourceClass = new (...args: any[]) => any;

/** PMTiles 파일 URL 을 base64url 로 encoding — Service Worker 의 /__pmtiles__/ path 매칭. */
function encodePmtilesUrl(url: string): string {
  const utf8Safe = encodeURIComponent(url);
  return btoa(utf8Safe).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

/**
 * Factory — mb 의 built-in VectorTileSource 를 base 로 PMTilesSource 클래스 반환.
 */
export function createPMTilesSourceClass(mb: MapboxMap): SourceClass {
  const style = mb.style;
  if (!style) throw new Error("[PMTilesSource] mb.style 미존재 — gl init 미완료");
  const StyleCtor = style.constructor;
  if (typeof StyleCtor.getSourceType !== "function") {
    throw new Error("[PMTilesSource] mb.style.constructor.getSourceType 미존재");
  }
  // biome-ignore lint/suspicious/noExplicitAny: minified base class
  const VectorTileSource = StyleCtor.getSourceType("vector") as any;
  if (typeof VectorTileSource !== "function") {
    throw new Error("[PMTilesSource] built-in vector source 못 찾음");
  }

  class PMTilesSource extends VectorTileSource {
    _pmtiles: PMTiles;
    _pmtilesUrl: string;
    _headerLoaded = false;

    constructor(
      id: string,
      options: PMTilesSourceSpec,
      dispatcher: Dispatcher,
      eventedParent: Evented,
    ) {
      // VectorTileSource 의 표준 spec — `url` (TileJSON URL) 과 `tiles` 둘 중 하나만 줘야 함.
      // url 이 있으면 parent 가 *TileJSON fetch 시도* 하고 우리 tiles 무시.
      // 따라서 *fakeOptions 에 url 제거* + tiles 만 사용.
      // 우리 PMTiles 파일 URL 은 internal `_pmtilesUrl` 로만 보관 (header fetch 용).
      const encoded = encodePmtilesUrl(options.url);
      // biome-ignore lint/correctness/noUnusedVariables: destructure to drop `url` from spread
      const { url: _pmtilesFileUrl, ...rest } = options;
      const fakeOptions = {
        ...rest,
        type: "vector",
        tiles: [`/__pmtiles__/${encoded}/{z}/{x}/{y}.pbf`],
        scheme: "xyz",
        minzoom: options.minzoom ?? 0,
        maxzoom: options.maxzoom ?? 22,
      };
      super(id, fakeOptions, dispatcher, eventedParent);
      // Naver fork VectorTileSource 가 _options 에 tiles 보관 — instance level
      // `this.tiles` 는 super 가 안 set. parent 의 loadTile 이 `this.tiles` 사용
      // (canonical.url(this.tiles, scheme)) 하므로 직접 set 필요.
      // biome-ignore lint/suspicious/noExplicitAny: parent internal fields
      (this as any).tiles = fakeOptions.tiles;
      // biome-ignore lint/suspicious/noExplicitAny: parent internal fields
      (this as any).scheme = fakeOptions.scheme;
      // biome-ignore lint/suspicious/noExplicitAny: parent internal fields
      (this as any).tileSize = 512;
      this._pmtilesUrl = options.url;
      this._pmtiles = new PMTiles(options.url);
    }

    /**
     * VectorTileSource.load() 는 TileJSON fetch 시도. 우리는 PMTiles header 로
     * metadata 채우고 mapbox-gl 의 표준 'data' 이벤트 firing.
     */
    load(): void {
      this.fire({ type: "dataloading", dataType: "source" });

      this._pmtiles
        .getHeader()
        .then((h) => {
          this.minzoom = h.minZoom;
          this.maxzoom = h.maxZoom;
          this.tileSize = 512;
          this._headerLoaded = true;
          // biome-ignore lint/suspicious/noExplicitAny: parent internal
          (this as any)._loaded = true;
          this.fire({ type: "data", dataType: "source", sourceDataType: "metadata" });
          this.fire({ type: "data", dataType: "source", sourceDataType: "content" });
        })
        .catch((err: Error) => {
          if (process.env.NODE_ENV !== "production") {
            console.warn(`[PMTilesSource:${this.id}] header load 실패:`, err.message);
          }
          this.fire({ type: "error", error: err });
        });
    }

    loaded(): boolean {
      return this._headerLoaded;
    }

    serialize(): PMTilesSourceSpec {
      return {
        type: "pmtiles",
        url: this._pmtilesUrl,
        // biome-ignore lint/suspicious/noExplicitAny: parent fields
        promoteId: (this as any).promoteId,
        // biome-ignore lint/suspicious/noExplicitAny: parent fields
        minzoom: (this as any).minzoom,
        // biome-ignore lint/suspicious/noExplicitAny: parent fields
        maxzoom: (this as any).maxzoom,
      };
    }
  }

  return PMTilesSource;
}
