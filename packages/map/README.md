# packages/map

Naver Maps 통합 + Canvas 마커 + 좌표 헬퍼.

## 의존
- `@gongzzang/shared`, `@gongzzang/tsconfig`
- Naver Maps JS API (Script tag, NOT npm)

## 제공 (sub-project 6+)
- `<NaverMap>` — 지도 컨테이너 컴포넌트
- `<MarkerLayer>` — Canvas 2D 마커 렌더 (수만 개 고속)
- `<PMTilesLayer>` — 벡터 타일 (Phase 3+, Cloudflare R2)
- `useMapState` — Zustand 지도 상태
- `useGeolocation` — 사용자 위치
- `coordTransform` — 좌표계 변환 (4326 ↔ 3857)
- `pnuToCenter` — PNU → 좌표

## 정책
- 좌표 입출력: EPSG:4326 (WGS84)
- 클라이언트는 *표시만*, 공간 연산은 PostGIS (서버)
- 마커 클러스터링: Supercluster
- 타일: PMTiles (정적, R2 호스팅)
- 출처 표기: 지도 위 "Naver Maps" 로고 + 공공 데이터 출처

→ ADR-0003, → @docs/frontend/naver-maps.md
