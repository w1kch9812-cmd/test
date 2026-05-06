# ADR-0017: 매물 마커 렌더링 — Naver Marker + Canvas content + BitmapStampCache (단일 렌더 박자)

| | |
|---|---|
| 작성일 | 2026-05-06 |
| 상태 | Accepted |
| 결정자 | 사용자 |
| 컨텍스트 | SP9 폴리곤 base layer 채택 ([ADR 0016](./0016-medallion-base-layer-postgis-silver-pmtiles-gold.md)) 직후 — 같은 지도 위에 표시될 매물/실거래/산단/광고 마커의 렌더 방식을 박제 |

## 컨텍스트

ADR 0016 이 폴리곤을 PMTiles 로 mapbox-gl 의 같은 WebGL 캔버스 안에 그리기로 확정했음. 이 캔버스 위에 동시에 살아있을 마커 종류:

- 매물 핀 (Listing — SP6-ii 진행 중)
- 실거래가 (RealTransaction — Phase 2+)
- 산업단지 라벨 (IndustrialComplex — Phase 2+)
- 매물 광고 (MapAdvertisement — Phase 2+)

같은 도메인을 운영해 본 형제 코드베이스 두 종류가 있음:

- `gongzzang-design-lab` — Canvas content + BitmapStampCache + Naver Marker 단일화
- `gongzzang-develop` (운영) — GL Symbol Layer + Canvas Overlay + DOM 마커 혼합 + GL/Canvas 애니 오케스트레이터

직접 비교 결과 (코드 + 시각 관찰):

| | design-lab | develop |
|---|---|---|
| 렌더 박자 | 한 갈래 (mapbox-gl 캔버스 + Naver Marker(canvas content)) | 세 갈래 (GL Symbol + Canvas Overlay + DOM) |
| 같은 화면 부드러움 | 끊김 없음 | 박자 어긋남 관측 |
| store | 1개 | 3개 (마이그레이션 진행 중) |
| 코드 정교함 | 낮음 (단순) | 높음 (Adapter, EventBus, 충돌 해결, 애니 오케스트레이터) |

**시각적 부드러움은 "코드의 정교함" 이 아니라 "렌더 박자가 한 갈래인가"** 가 결정. develop 의 GL/Canvas/DOM 3-레이어 동기화 비용이 정교함의 이득을 깎아먹음.

현재 `gongzzang_2` 의 [`apps/web/components/listings/listing-map.tsx`](../../apps/web/components/listings/listing-map.tsx) 는 SP6-ii MVP 로 `naver.maps.Marker` 를 매번 재생성하는 가장 단순한 형태 — 매물 100개 이상에서 무너질 구조.

## 결정

매물 및 같은 지도 위 모든 마커를 다음 단일 패턴으로 렌더한다:

> **Naver Marker (1개 인스턴스/마커)**
> └ icon content = `<div>` 컨테이너 안 `<canvas>`
> └ canvas 그림은 `BitmapStampCache` 에서 미리 구운 비트맵을 `drawImage` 로 stamp

핵심 원칙:

1. **렌더 박자는 한 갈래** — mapbox-gl WebGL 캔버스 (베이스 + 폴리곤) + Naver Marker (Canvas content 비트맵 stamp) 둘 뿐. GL Symbol Layer 안 씀, DOM-only 마커 안 씀
2. **마커 비트맵은 캐시** — 같은 type/state 의 마커는 한 번만 그리고 N번 stamp
3. **마커 인스턴스는 풀링** — `MarkerManager` 가 type 별 풀 보유, 위치/내용 hash 비교 후 변경분만 갱신
4. **마커 도메인 분리는 "데이터 차원" 에서만** — 매물/실거래/산단/광고가 *컴포넌트로 분리되지 않음*. 단일 `MarkersLayer` 가 모든 도메인의 마커 데이터를 받아 한 번에 그림 (develop 의 도메인별 레이어 컴포넌트 폭증 패턴 차단)
5. **상태는 단일 store** — 마커 상태/선택/호버는 모두 한 Zustand store
6. **개별 마커 애니메이션은 CSS transform 으로** — GL pulse ring / GL hover 같은 GL-side 애니 안 씀. CSS `transform: scale()` + `opacity` 만

## 대안

| 안 | 평가 |
|---|---|
| **A. 본 결정 — Naver Marker + Canvas + BitmapStampCache** | ✅ 한 박자, 비트맵 캐시로 1000+ 마커 부드러움, 텍스트 antialiasing 양호, 검증된 lab 패턴 |
| B. GL Symbol Layer (mapbox-gl 안에 마커도 직접 추가) | 🟡 5000+ 마커에서 이론상 가장 빠름. 그러나 텍스트 렌더 약함, Naver SDK 위에서 layer 추가 시 SDK 호환성 위험, develop 처럼 다른 렌더와 섞이면 박자 어긋남 |
| C. DOM-only 마커 (`<div>` 위치 absolute) | ❌ 100개부터 reflow 비용 폭증, 풀링/캐시 효과 없음 |
| D. develop 식 혼합 (GL + Canvas + DOM) | ❌ 정교한 효과 가능하나 박자 어긋남으로 시각 품질 역행, 운영 부채 큼 |
| E. Naver SDK 의 `clustering` 서브모듈에 의존 | 🟡 단순 클러스터링은 가능하나 마커 디자인 자유도 낮음 — 보조 도구로만 |

## 결과

### 긍정
- 시각: 폴리곤(GL) 과 마커(Canvas stamp) 가 같은 mapbox-gl frame timing 안에서 동기화 → 드래그/줌 부드러움 보장
- 성능: 비트맵 캐시 stamp 는 마커당 거의 무료. 1000+ 마커 60fps 가능
- 단순성: 마커 컴포넌트 1개 (`MarkersLayer`) — 도메인 추가는 데이터 차원 추가일 뿐, 새 컴포넌트 마운트 아님
- 호환: SP9 폴리곤 PMTiles 패턴과 충돌 없음. lab 의 `UnifiedPolygonGLLayer` + `UnifiedMarkerLayer` 가 같이 살던 패턴 그대로

### 부정
- 거대 마커 (광고 카드 등) 가 캐시 키 폭증을 유발할 수 있음 → `BitmapStampCache.maxEntries` 에 LRU 제한 필요 (lab 은 800)
- GL Symbol 의 회전 동기화 같은 효과는 포기 (3D 회전 시 마커가 캔버스 평면에 고정 — 의도된 trade-off)
- 마커 hit-test 는 Naver Marker 의 click 이벤트에 의존 — 더 정교한 hit area 는 별도 구현 필요

### 영향 영역
- `apps/web/components/map/` (신규 폴더 — 현재 `components/listings/listing-map.tsx` 의 단일 파일은 SP9 프론트 T 에서 흡수 후 삭제)
  - `MarkersLayer.tsx` — 단일 마커 레이어 컴포넌트
  - `BitmapStampCache.ts` — lab 패턴 포팅
  - `MarkerManager.ts` — 풀링/생명주기 (lab 패턴 포팅)
  - `marker-renderers/` — type 별 canvas 그리기 함수 (`drawListingPin`, `drawRealTransactionDot`, …)
- `apps/web/lib/naver-maps.ts` — 변경 없음 (SDK loader 그대로)
- `apps/web/stores/listings.ts` 외 단일 map store 통합 검토

### develop 에서 *가져오지 않을* 것
- GL Symbol Layer (`useGLMarkerLayer`, `useGLSymbolLayer`)
- GL 애니메이션 오케스트레이터 (`glFadeAnimation`, `glHoverAnimation`, `glPulseRingSharedLayer`, `glMarkerAnimationOrchestrator`)
- 도메인별 마커 컴포넌트 분리 (`features/sale-property/`, `features/court-auction/` 식 마커 폭증)
- store 다중화 (`useMapStore` + `useNewMapStore` + `useUnifiedMapStore`)
- `EventBus` (충돌 실제로 발생할 때 별도 ADR 로 도입)

## 재검토 트리거

- 매물 마커 동시 표시 5000개 초과가 정규 시나리오가 됨 → GL Symbol Layer (대안 B) 부분 도입 ADR
- 광고 마커가 비트맵 캐시 폭증을 유발 (`maxEntries` LRU 미스율 > 30%) → 광고만 별도 렌더 경로 ADR
- 마커 클릭과 폴리곤 클릭이 같은 좌표에서 충돌 → 우선순위 EventBus ADR (develop 패턴 부분 도입)
- 3D 모드 도입 (tilt/pitch) → 마커 GL Symbol 전환 재평가

## 참조

- → [ADR 0013](./0013-listing-search-naver-maps.md) (Naver Maps SDK 채택)
- → [ADR 0016](./0016-medallion-base-layer-postgis-silver-pmtiles-gold.md) (PMTiles 폴리곤 base layer)
- → [SP9 spec](../superpowers/specs/2026-05-06-sub-project-9-medallion-base-layer-design.md)
- 형제 프로젝트 reference (Canvas + BitmapStampCache 검증):
  - [`gongzzang-design-lab/lib/map/BitmapStampCache.ts`](C:\Users\User\Desktop\gongzzang\gongzzang\apps\gongzzang-design-lab\lib\map\BitmapStampCache.ts)
  - [`gongzzang-design-lab/lib/map/MarkerManager.ts`](C:\Users\User\Desktop\gongzzang\gongzzang\apps\gongzzang-design-lab\lib\map\MarkerManager.ts)
  - [`gongzzang-design-lab/lib/map/CanvasMarkerRenderer.ts`](C:\Users\User\Desktop\gongzzang\gongzzang\apps\gongzzang-design-lab\lib\map\CanvasMarkerRenderer.ts)
  - [`gongzzang-design-lab/components/map/naver/UnifiedMarkerLayer.tsx`](C:\Users\User\Desktop\gongzzang\gongzzang\apps\gongzzang-design-lab\components\map\naver\UnifiedMarkerLayer.tsx)
- 반례 (혼합 렌더 박자가 시각 품질을 역행):
  - `gongzzang-develop/gongzzang-client/apps/platform-web/src/components/map/UnifiedMap/collision/`
- AGENTS.md § 0 SSS 7 기둥 — 일관성(같은 도메인 같은 패턴), 명확성(렌더 박자 단일화)
