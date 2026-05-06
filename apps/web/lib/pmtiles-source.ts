/**
 * SP9 ADR 0019 — PMTilesSource via VectorTileSource subclass + `addSourceType`.
 *
 * 핵심: mapbox-gl 의 built-in `VectorTileSource` 를 subclass 해서 `load()` /
 * `loadTile()` 만 override. 클라가 PMTiles 단일 파일에 직접 byte-range request,
 * worker-side parsing 은 default VectorWorkerSource 가 처리.
 *
 * Class 는 *factory function* 으로 lazy 생성 — Naver bundle 의 mapbox-gl
 * VectorTileSource 클래스 reference 가 mb 인스턴스 로딩 후에야 가능.
 */

import { PMTiles } from "pmtiles";

// ===== mapbox-gl v2 internal type stubs =====

interface CanonicalTileID {
  z: number;
  x: number;
  y: number;
}
interface OverscaledTileID {
  canonical: CanonicalTileID;
  overscaledZ: number;
  wrap: number;
  overscaleFactor(): number;
}
interface Actor {
  send(
    type: string,
    data: unknown,
    callback?: (err: Error | null, result?: unknown) => void,
    targetMapId?: string,
    mustQueue?: boolean,
  ): { cancel: () => void };
}
interface Dispatcher {
  getActor(): Actor;
}
interface Tile {
  tileID: OverscaledTileID;
  uid: number;
  state: string;
  actor?: Actor;
  setExpiryData?: (data: { cacheControl?: string; expires?: string }) => void;
  loadVectorData?: (data: unknown, painter: unknown) => void;
  unloadVectorData?: () => void;
  hasData?: () => boolean;
  request?: { cancel: () => void };
}
interface Evented {
  fire(event: { type: string; [k: string]: unknown }): unknown;
  on?(type: string, listener: (e: unknown) => void): unknown;
  off?(type: string, listener: (e: unknown) => void): unknown;
  setEventedParent?(parent: Evented | null, data?: Record<string, unknown>): unknown;
}

/**
 * mb 가 보유한 mapbox-gl Style class — `getSourceType` static 메서드 노출.
 * Naver fork 가 mapbox-gl v2 표준 그대로 보존 (전수조사 확인).
 */
interface MapboxStyle {
  constructor: {
    getSourceType: (name: string) => unknown;
    setSourceType?: (name: string, cls: unknown) => void;
  };
}
interface MapboxMap {
  style?: MapboxStyle;
  showCollisionBoxes?: boolean;
  // biome-ignore lint/suspicious/noExplicitAny: mapbox-gl internal painter
  painter?: any;
}

// ===== Source 등록용 spec =====

export interface PMTilesSourceSpec {
  type: "pmtiles";
  url: string;
  /** vector tile attribute → feature.id (예: "PNU"). */
  promoteId?: string;
  minzoom?: number;
  maxzoom?: number;
  attribution?: string;
}

// ===== mapbox-gl v2 Source class shape (subclassable) =====

// biome-ignore lint/suspicious/noExplicitAny: mapbox-gl runtime class
type SourceClass = new (...args: any[]) => any;

/**
 * Factory — mb 의 built-in VectorTileSource 를 base 로 PMTilesSource 클래스 반환.
 *
 * mb 인스턴스 별로 1회 호출. Naver fork 의 minified VectorTileSource 클래스를
 * 동적으로 subclass — 우리가 mapbox-gl 의 Evented / SourceCache lifecycle 자동 inherit.
 *
 * @returns PMTilesSource class. `mb.addSourceType("pmtiles", cls, cb)` 로 등록.
 * @throws mb.style.constructor.getSourceType 가 vector 클래스 못 찾으면 Error.
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
    // 우리 인스턴스 식별
    _pmtiles: PMTiles;
    _pmtilesUrl: string;
    _headerLoaded = false;

    constructor(
      id: string,
      options: PMTilesSourceSpec,
      dispatcher: Dispatcher,
      eventedParent: Evented,
    ) {
      // VectorTileSource 의 표준 spec 으로 변환 — type:"vector" + 더미 tiles[].
      // 우리가 loadTile override 해서 dummy URL 은 실제 fetch 안 됨.
      const fakeOptions = {
        ...options,
        type: "vector",
        tiles: [`pmtiles-internal://${id}/{z}/{x}/{y}.pbf`],
        scheme: "xyz",
        minzoom: options.minzoom ?? 0,
        maxzoom: options.maxzoom ?? 22,
      };
      super(id, fakeOptions, dispatcher, eventedParent);
      this._pmtilesUrl = options.url;
      this._pmtiles = new PMTiles(options.url);
    }

    /**
     * VectorTileSource.load() 는 TileJSON fetch 를 시도. 우리는 PMTiles header
     * 로 metadata 채우고 mapbox-gl 의 표준 'data' 이벤트 firing.
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
          // mapbox-gl Source._loaded internal field 도 set — SourceCache 가 polling.
          // biome-ignore lint/suspicious/noExplicitAny: parent internal
          (this as any)._loaded = true;
          this.fire({ type: "data", dataType: "source", sourceDataType: "metadata" });
          this.fire({ type: "data", dataType: "source", sourceDataType: "content" });
        })
        .catch((err: Error) => {
          // PMTiles 파일이 없거나 (404) 손상된 경우 — silent fail (지도는 정상 동작).
          // dev 에서는 어떤 source 가 실패했는지 알 수 있게 console.warn.
          if (process.env.NODE_ENV !== "production") {
            console.warn(`[PMTilesSource:${this.id}] header load 실패:`, err.message);
          }
          this.fire({ type: "error", error: err });
        });
    }

    loaded(): boolean {
      return this._headerLoaded;
    }

    hasTile(_tileID: OverscaledTileID): boolean {
      return true;
    }

    /**
     * 핵심 — PMTiles.getZxy 로 raw .pbf bytes 추출 후 *blob URL* 만들어 worker 에
     * dispatch. worker 의 standard VectorTileWorkerSource.loadVectorData 가
     * `params.request.url` 을 fetch 하는데, blob URL 은 즉시 응답 → 자연스럽게
     * 표준 path. worker bundle 작성 / loadVectorData override 모두 불필요.
     *
     * blob URL 은 callback 안에서 즉시 revoke (메모리 leak 방지).
     */
    loadTile(tile: Tile, callback: (err?: Error | null, result?: unknown) => void): void {
      const z = tile.tileID.canonical.z;
      const x = tile.tileID.canonical.x;
      const y = tile.tileID.canonical.y;

      const ac = new AbortController();
      tile.request = { cancel: () => ac.abort() };

      this._pmtiles
        .getZxy(z, x, y, ac.signal)
        .then((resp) => {
          if (!resp || resp.data.byteLength === 0) {
            callback(null);
            return;
          }

          // PMTiles 는 *압축 해제된* raw MVT bytes 반환. blob 으로 wrap.
          const blob = new Blob([resp.data], { type: "application/x-protobuf" });
          const blobUrl = URL.createObjectURL(blob);

          // 표준 VectorTileSource 가 보내는 params 와 동일 shape — request 만 우리 blob URL.
          const params = {
            type: "vector",
            request: {
              url: blobUrl,
              // 표준 mapbox-gl ajax 가 무시하는 metadata.
              headers: {},
              method: "GET",
            },
            uid: tile.uid,
            tileID: tile.tileID,
            tileZoom: z,
            zoom: z,
            // biome-ignore lint/suspicious/noExplicitAny: parent class field
            tileSize: ((this as any).tileSize ?? 512) * tile.tileID.overscaleFactor(),
            // biome-ignore lint/suspicious/noExplicitAny: parent class field
            source: (this as any).id,
            pixelRatio: globalThis.devicePixelRatio || 1,
            showCollisionBoxes:
              // biome-ignore lint/suspicious/noExplicitAny: parent class field
              ((this as any).map as MapboxMap | undefined)?.showCollisionBoxes ?? false,
            // biome-ignore lint/suspicious/noExplicitAny: parent class field
            promoteId: (this as any).promoteId,
          };

          const done = (err: Error | null, data: unknown) => {
            // blob URL 즉시 cleanup — worker 가 fetch 끝낸 후라 안전.
            URL.revokeObjectURL(blobUrl);
            if (err) {
              callback(err);
              return;
            }
            if (tile.loadVectorData && data) {
              // biome-ignore lint/suspicious/noExplicitAny: parent class field
              tile.loadVectorData(data, ((this as any).map as MapboxMap | undefined)?.painter);
            }
            callback(null);
          };

          if (!tile.actor || tile.state === "expired") {
            // biome-ignore lint/suspicious/noExplicitAny: parent class field
            tile.actor = ((this as any).dispatcher as Dispatcher).getActor();
            tile.actor.send("loadTile", params, done, undefined, true);
          } else {
            tile.actor.send("reloadTile", params, done, undefined, true);
          }
        })
        .catch((err: Error) => {
          if (err.name === "AbortError") {
            callback(null);
            return;
          }
          callback(err);
        });
    }

    abortTile(tile: Tile, callback: (err?: Error | null) => void): void {
      if (tile.request) {
        tile.request.cancel();
        tile.request = undefined;
      }
      if (tile.actor) {
        // biome-ignore lint/suspicious/noExplicitAny: parent field
        tile.actor.send("abortTile", { uid: tile.uid, type: "vector", source: (this as any).id });
      }
      callback(null);
    }

    unloadTile(tile: Tile, callback?: (err?: Error | null) => void): void {
      if (tile.unloadVectorData) tile.unloadVectorData();
      if (tile.actor) {
        // biome-ignore lint/suspicious/noExplicitAny: parent field
        tile.actor.send("removeTile", { uid: tile.uid, type: "vector", source: (this as any).id });
      }
      callback?.(null);
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

    hasTransition(): boolean {
      return false;
    }
  }

  return PMTilesSource;
}
