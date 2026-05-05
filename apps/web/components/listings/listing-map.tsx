"use client";
import { useEffect, useRef } from "react";
import { pinIconHtml } from "@/components/listings/listing-pin";
import type { ListingCard } from "@/lib/listings/api";
import { loadNaverMaps } from "@/lib/naver-maps";
import { useListingsStore } from "@/stores/listings";

interface ListingMapProps {
  listings: ListingCard[];
}

export function ListingMap({ listings }: ListingMapProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const mapRef = useRef<naver.maps.Map | null>(null);
  const markersRef = useRef<naver.maps.Marker[]>([]);
  const boundsTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const setBounds = useListingsStore((s) => s.setBounds);
  const selectedId = useListingsStore((s) => s.selectedListingId);
  const setSelected = useListingsStore((s) => s.setSelectedListingId);

  // 1. 지도 초기화 (1회)
  useEffect(() => {
    let cancelled = false;
    loadNaverMaps().then((naverNs) => {
      if (cancelled || !containerRef.current) return;
      const map = new naverNs.maps.Map(containerRef.current, {
        center: new naverNs.maps.LatLng(37.5665, 126.978), // 서울 시청
        zoom: 8,
        mapTypeControl: false,
      });
      mapRef.current = map;

      // bounds 변경 이벤트 (debounce 350ms)
      naverNs.maps.Event.addListener(map, "bounds_changed", () => {
        if (boundsTimerRef.current) clearTimeout(boundsTimerRef.current);
        boundsTimerRef.current = setTimeout(() => {
          const bounds = map.getBounds() as naver.maps.LatLngBounds;
          const sw = bounds.getSW();
          const ne = bounds.getNE();
          setBounds({
            south: sw.lat(),
            west: sw.lng(),
            north: ne.lat(),
            east: ne.lng(),
          });
        }, 350);
      });

      // 초기 bounds 도 emit
      const b = map.getBounds() as naver.maps.LatLngBounds;
      setBounds({
        south: b.getSW().lat(),
        west: b.getSW().lng(),
        north: b.getNE().lat(),
        east: b.getNE().lng(),
      });
    });
    return () => {
      cancelled = true;
      if (boundsTimerRef.current) {
        clearTimeout(boundsTimerRef.current);
        boundsTimerRef.current = null;
      }
    };
  }, [setBounds]);

  // 2. 매물 변경 → marker 재생성
  useEffect(() => {
    if (!mapRef.current) return;
    const map = mapRef.current;

    // 기존 marker 제거
    for (const m of markersRef.current) m.setMap(null);
    markersRef.current = [];

    // 새 marker 생성
    for (const listing of listings) {
      const marker = new naver.maps.Marker({
        position: new naver.maps.LatLng(listing.lat, listing.lng),
        map,
        icon: {
          content: pinIconHtml(listing.listing_type, { selected: listing.id === selectedId }),
          anchor: new naver.maps.Point(14, 28),
        },
      });
      naver.maps.Event.addListener(marker, "click", () => {
        setSelected(listing.id);
      });
      markersRef.current.push(marker);
    }
  }, [listings, selectedId, setSelected]);

  return <div ref={containerRef} className="h-full w-full" />;
}
