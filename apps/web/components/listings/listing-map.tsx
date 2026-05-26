"use client";
import { useTranslations } from "next-intl";
import { useEffect, useRef, useState } from "react";
import { type MapboxGLLike, setupMapboxRuntime } from "@/lib/map/listing-map-runtime";
import { buildListingMarkerLayerFilter } from "@/lib/map/listing-marker-filter";
import {
  buildListingMarkerServerKey,
  type ListingMarkerServerState,
  loadListingMarkerServerState,
} from "@/lib/map/listing-marker-server-state";
import { ALL_ACTIVE_MARKER_FILTER_HASH } from "@/lib/map/marker-tile-contract";
import { LISTING_MARKER_TILE_CIRCLE_LAYER_ID } from "@/lib/map/marker-tile-style";
import { loadNaverMaps } from "@/lib/naver-maps";
import { usePanelStack } from "@/lib/panel/use-panel-stack";
import { useListingsStore } from "@/stores/listings";

function initialMarkerServerState(): ListingMarkerServerState {
  return {
    filterHash: ALL_ACTIVE_MARKER_FILTER_HASH,
    totalCount: undefined,
    projectionVersion: undefined,
    anchorSnapshotId: undefined,
    requestKey: buildListingMarkerServerKey({
      filterHash: ALL_ACTIVE_MARKER_FILTER_HASH,
      projectionVersion: undefined,
      anchorSnapshotId: undefined,
    }),
  };
}

function isAbortError(err: unknown): boolean {
  return err instanceof DOMException && err.name === "AbortError";
}

export function ListingMap() {
  const containerRef = useRef<HTMLDivElement>(null);
  const mapRef = useRef<naver.maps.Map | null>(null);
  const [mapboxInstance, setMapboxInstance] = useState<MapboxGLLike | null>(null);
  const [, setMarkerServerState] = useState<ListingMarkerServerState>(initialMarkerServerState);
  const filters = useListingsStore((state) => state.filters);
  const { push: pushPanel } = usePanelStack();

  useEffect(() => {
    const mb = mapboxInstance;
    if (!mb?.setFilter || !mb.getLayer?.(LISTING_MARKER_TILE_CIRCLE_LAYER_ID)) return;
    mb.setFilter(LISTING_MARKER_TILE_CIRCLE_LAYER_ID, buildListingMarkerLayerFilter(filters));
  }, [filters, mapboxInstance]);

  useEffect(() => {
    const controller = new AbortController();

    loadListingMarkerServerState(filters, controller.signal)
      .then(setMarkerServerState)
      .catch((err: unknown) => {
        if (isAbortError(err)) return;
        console.warn(
          "[ListingMap] listing marker server state unavailable",
          err instanceof Error ? err.message : String(err),
        );
      });

    return () => {
      controller.abort();
    };
  }, [filters]);

  useEffect(() => {
    let cancelled = false;
    const cleanups: Array<() => void> = [];

    loadNaverMaps().then((naverNs) => {
      if (cancelled || !containerRef.current) return;
      const map = new naverNs.maps.Map(containerRef.current, {
        center: new naverNs.maps.LatLng(37.5665, 126.978),
        zoom: 8,
        minZoom: 7,
        maxZoom: 21,
        gl: true,
        zoomControl: false,
        mapTypeControl: false,
        disableKineticPan: false,
      } as naver.maps.MapOptions);
      mapRef.current = map;

      if (process.env.NODE_ENV !== "production") {
        (window as unknown as Record<string, unknown>).__listingMap = map;
      }

      setupMapboxRuntime(
        map,
        cleanups,
        () => cancelled,
        (pnu) => pushPanel({ kind: "parcel", id: pnu, view: "summary" }),
        (listingId) => pushPanel({ kind: "listing", id: listingId, view: "summary" }),
        setMapboxInstance,
      ).catch((e: unknown) => {
        if (!cancelled) {
          console.warn(
            "[ListingMap] mapbox-gl bridge unavailable; vector layers disabled",
            e instanceof Error ? e.message : String(e),
          );
        }
      });
    });
    return () => {
      cancelled = true;
      setMapboxInstance(null);
      for (const fn of cleanups) fn();
    };
  }, [pushPanel]);

  return (
    <div className="relative h-full w-full">
      <div ref={containerRef} className="h-full w-full" />
      <MapAttribution />
    </div>
  );
}

function MapAttribution() {
  const t = useTranslations("map.attribution");
  return (
    <aside
      className="pointer-events-auto absolute bottom-1 right-1 z-10 rounded bg-white/85 px-2 py-0.5 text-[10px] leading-tight text-gray-600 shadow-sm backdrop-blur-sm dark:bg-black/70 dark:text-gray-300"
      aria-label={t("ariaDataSource")}
    >
      <span>{t("parcelSourceLabel")}: </span>
      <a
        href="https://www.vworld.kr/dtmk/dtmk_ntads_s002.do?dsId=30563"
        target="_blank"
        rel="noopener noreferrer"
        className="underline hover:text-gray-900 dark:hover:text-white"
      >
        {t("vWorldLink")}
      </a>
      <span className="ml-1">{t("license")}</span>
    </aside>
  );
}
