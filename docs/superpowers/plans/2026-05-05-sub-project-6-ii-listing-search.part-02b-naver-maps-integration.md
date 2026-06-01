# Sub-project 6-ii Listing Search - Part 02B: Naver Maps Integration

Parent index: [Sub-project 6-ii Listing Search - Part 02](./2026-05-05-sub-project-6-ii-listing-search.part-02.md).
## Task 3: Naver Maps 통합 — loader + ListingMap + 핀 + 클러스터 + bounds 이벤트

**Files:**
- Modify: `apps/web/lib/env.ts` (NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID 추가)
- Modify: `apps/web/.env.local.example`
- Modify: `turbo.json` (globalEnv)
- Modify: `apps/web/package.json` (`@types/navermaps`)
- Create: `apps/web/lib/naver-maps.ts`
- Create: `apps/web/components/listings/listing-pin.tsx`
- Create: `apps/web/components/listings/listing-map.tsx`

- [ ] **Step 3.1: env.ts 확장 (zod)**

`apps/web/lib/env.ts` 의 `PublicEnvSchema` 에 `NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID` 추가:

```typescript
const PublicEnvSchema = z.object({
  NEXT_PUBLIC_API_BASE_URL: z.string().url().default("http://localhost:8080"),
  NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID: z.string().min(1).default("naver-maps-placeholder"),
});
```

(default 둠 — placeholder 면 지도 안 뜨지만 build 통과.)

`safeParse` 호출 시 객체에도 추가:

```typescript
const parsed = Schema.safeParse({
  NEXT_PUBLIC_API_BASE_URL: process.env.NEXT_PUBLIC_API_BASE_URL,
  NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID: process.env.NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID,
  // ... 기존 server env
});
```

- [ ] **Step 3.2: .env.local.example + turbo.json**

`apps/web/.env.local.example` 끝에:

```
# SP6-ii — Naver Maps
NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID=naver-maps-placeholder
```

`turbo.json` 의 `globalEnv` 끝에:

```json
"NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID",
```

- [ ] **Step 3.3: deps 추가**

```bash
pnpm --filter=@gongzzang/web add -D @types/navermaps@^3.7.0
```

- [ ] **Step 3.4: lib/naver-maps.ts (lazy script loader)**

`apps/web/lib/naver-maps.ts`:

```typescript
import { env } from "@/lib/env";

let _readyPromise: Promise<typeof naver> | null = null;

/**
 * Naver Maps SDK script lazy load. 한 번만 로드.
 *
 * `naver` 글로벌이 ready 되면 resolve.
 */
export function loadNaverMaps(): Promise<typeof naver> {
  if (_readyPromise) return _readyPromise;
  if (typeof window === "undefined") {
    return Promise.reject(new Error("loadNaverMaps must run in browser"));
  }
  _readyPromise = new Promise((resolve, reject) => {
    if (typeof naver !== "undefined" && naver.maps) {
      resolve(naver);
      return;
    }
    const script = document.createElement("script");
    script.src = `https://oapi.map.naver.com/openapi/v3/maps.js?ncpClientId=${env.NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID}&submodules=clustering`;
    script.async = true;
    script.onload = () => resolve(naver);
    script.onerror = () => reject(new Error("Naver Maps SDK failed to load"));
    document.head.appendChild(script);
  });
  return _readyPromise;
}
```

- [ ] **Step 3.5: components/listings/listing-pin.tsx (SVG marker template)**

`apps/web/components/listings/listing-pin.tsx`:

```typescript
import { getPinColor, type ListingTypeKey } from "@/lib/listings/pin-color";

/**
 * Naver Maps 의 marker icon 으로 사용할 SVG HTML string.
 * `new naver.maps.Marker({ icon: { content: pinIconHtml(...) } })`.
 */
export function pinIconHtml(listingType: string, options: { selected?: boolean } = {}): string {
  const color = getPinColor(listingType);
  const size = options.selected ? 36 : 28;
  const stroke = options.selected ? "#ffffff" : "#1f2937";
  const strokeWidth = options.selected ? 3 : 1.5;
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 24 24" fill="${color}" stroke="${stroke}" stroke-width="${strokeWidth}">
    <path d="M12 2C7.58 2 4 5.58 4 10c0 5.25 7 12 8 12s8-6.75 8-12c0-4.42-3.58-8-8-8z"/>
    <circle cx="12" cy="10" r="3" fill="#ffffff"/>
  </svg>`;
}
```

- [ ] **Step 3.6: components/listings/listing-map.tsx**

`apps/web/components/listings/listing-map.tsx`:

```typescript
"use client";
import { useEffect, useRef } from "react";
import { loadNaverMaps } from "@/lib/naver-maps";
import { useListingsStore } from "@/stores/listings";
import type { ListingCard } from "@/lib/listings/api";
import { pinIconHtml } from "@/components/listings/listing-pin";

interface ListingMapProps {
  listings: ListingCard[];
}

export function ListingMap({ listings }: ListingMapProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const mapRef = useRef<naver.maps.Map | null>(null);
  const markersRef = useRef<naver.maps.Marker[]>([]);
  const setBounds = useListingsStore((s) => s.setBounds);
  const selectedId = useListingsStore((s) => s.selectedListingId);
  const setSelected = useListingsStore((s) => s.setSelectedListingId);

  // 1. 지도 초기화 (1회)
  useEffect(() => {
    let cancelled = false;
    loadNaverMaps().then((naverNs) => {
      if (cancelled || !containerRef.current) return;
      const map = new naverNs.maps.Map(containerRef.current, {
        center: new naverNs.maps.LatLng(37.5665, 126.978),  // 서울 시청
        zoom: 8,
        mapTypeControl: false,
      });
      mapRef.current = map;

      // bounds 변경 이벤트 (debounce)
      let timer: ReturnType<typeof setTimeout> | null = null;
      naverNs.maps.Event.addListener(map, "bounds_changed", () => {
        if (timer) clearTimeout(timer);
        timer = setTimeout(() => {
          const bounds = map.getBounds() as naver.maps.LatLngBounds;
          const sw = bounds.getMin();
          const ne = bounds.getMax();
          setBounds({
            south: sw.y,
            west: sw.x,
            north: ne.y,
            east: ne.x,
          });
        }, 350);
      });
      // 초기 bounds 도 emit
      const b = map.getBounds() as naver.maps.LatLngBounds;
      setBounds({
        south: b.getMin().y,
        west: b.getMin().x,
        north: b.getMax().y,
        east: b.getMax().x,
      });
    });
    return () => {
      cancelled = true;
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
```

- [ ] **Step 3.7: typecheck**

```bash
pnpm --filter=@gongzzang/web typecheck
```

Expected: PASS (단 `@types/navermaps` 의 정확한 type 시그니처에 따라 일부 cast 필요할 수 있음 — 발견 시 inline `as` 또는 `// eslint-disable` 대신 type narrowing 으로 해결).

- [ ] **Step 3.8: Commit**

```bash
git add apps/web/lib/naver-maps.ts apps/web/lib/env.ts apps/web/.env.local.example apps/web/components/listings/listing-pin.tsx apps/web/components/listings/listing-map.tsx apps/web/package.json apps/web/pnpm-lock.yaml turbo.json
git commit -m "feat(6ii-T3): Naver Maps 통합 — lazy SDK loader + ListingMap + SVG pin

- lib/env.ts: NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID (zod public schema)
- lib/naver-maps.ts: lazy script loader (한 번만 inject + ready Promise)
- components/listings/listing-pin.tsx: SVG marker template (6 종 unique color + selected highlight)
- components/listings/listing-map.tsx: Naver Maps + 마커 + bounds_changed debounced (350ms) → store + click → selected
- @types/navermaps 추가
- turbo.json globalEnv 추가"
```

---
