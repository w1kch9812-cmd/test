# Naver Maps API

## 개요

- 운영 기관: 네이버 클라우드 플랫폼
- 공식 사이트: https://www.ncloud.com/product/applicationService/maps
- 우리 사용: 지도 렌더링, 마커 표시, 지오코딩, 경로 계산

## 인증

- 네이버 클라우드 플랫폼 가입 → 애플리케이션 등록 → Client ID/Secret 발급
- 환경변수: `NAVER_MAPS_CLIENT_ID`, `NAVER_MAPS_CLIENT_SECRET`
- *서비스 URL* 등록 (도메인 제한)

## 무료 티어 (2026 기준)

- **월 12만 호출** 무료 (지도 + 검색 + 경로 통합)
- 일 4,000 호출 (사용자 1,000명 = 일 1만 호출 가능)
- 초과 시 호출당 ~3원

## 핵심 SDK

| 종류 | 용도 |
|------|------|
| Maps JavaScript API | 브라우저 지도 렌더링 |
| Maps API for Web Dynamic | 동적 지도 + 마커 |
| Maps API for Web Static | 정적 이미지 |
| Geocoding API | 주소 → 좌표 |
| Reverse Geocoding API | 좌표 → 주소 |
| Directions API | 경로 (자동차/대중교통) |

## 좌표계

- **EPSG:4326 (WGS84)** — 입출력 표준
- 국내 주소·POI 검색은 자동으로 한국 좌표 처리

## 요청 예시 (지오코딩)

```
GET https://maps.apigw.ntruss.com/map-geocode/v2/geocode?
  query=서울특별시 강남구 테헤란로 123

Headers:
  X-NCP-APIGW-API-KEY-ID: {NAVER_MAPS_CLIENT_ID}
  X-NCP-APIGW-API-KEY: {NAVER_MAPS_CLIENT_SECRET}
```

## 클라이언트 SDK 사용 (Next.js)

```tsx
// packages/map/src/naver-map.tsx
import Script from "next/script";

export function NaverMap({ children }: Props) {
  return (
    <>
      <Script
        strategy="beforeInteractive"
        src={`https://oapi.map.naver.com/openapi/v3/maps.js?ncpClientId=${process.env.NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID}`}
      />
      {/* map container */}
    </>
  );
}
```

`NEXT_PUBLIC_*` = 브라우저 노출. 도메인 제한으로 보호.

## 마커 렌더링 (Canvas 2D, 수만 마커 고속)

브라우저 기본 마커는 ~1,000개 한계. 수만 마커는 Canvas 2D 직접 렌더 필요. Phase 3에서 도입 (sub-project 6).

→ `packages/map/src/canvas-markers.tsx` (Phase 3+)

## Circuit Breaker 정책

- timeout: 5초 (지도는 사용자 응답성 중요)
- retry: 0회 (지도 깨짐 방지)
- fallback: "지도를 불러올 수 없어요" 메시지

## 캐시 정책

| 종류 | TTL |
|------|-----|
| 지오코딩 결과 | 30일 (주소→좌표는 거의 안 바뀜) |
| 정적 지도 이미지 | CDN 7일 |
| 경로 계산 | 1시간 |

## 비용 추정

| Phase | 사용자 | 호출/월 | 비용 |
|-------|--------|--------|------|
| 1 | 0 | 1만 | 무료 |
| 2 | 1,000 | 30만 | 무료 한도 (12만) 초과 → ~₩6만/월 |
| 3 | 10,000 | 300만 | ~₩90만/월 |
| 4 | 100,000 | 3,000만 | ~₩900만/월 |

→ Phase 3+에서 캐시 전략 강화 (이지오코딩 결과 영구 보존).

## 대안

- Mapbox GL JS: 글로벌, 한국 사용자 익숙도 낮음
- MapLibre (OSS): 무료, 그러나 한국 데이터 약함
- Kakao Map: 거의 동등, 비교 후 결정 가능 (Phase 3 ADR)

→ ADR-0003 참조.

## 라이선스

- 네이버 클라우드 이용약관
- 정적 지도 다운로드 후 재배포 금지 (실시간 호출만)
- "Naver Maps" 로고 표기 의무 (지도 위)

## 한국 사용자 친화 기능

- POI 풍부 (식당, 카페, 시설)
- 한국 주소 자동완성
- 도로명·지번 둘 다 검색 가능
- 한국어 음성 안내 (경로)
