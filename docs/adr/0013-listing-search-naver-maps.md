# ADR-0013: Listing 검색 화면의 지도 vendor — Naver Maps

| | |
|---|---|
| 작성일 | 2026-05-05 |
| 상태 | Accepted |
| 결정자 | 사용자 |
| 컨텍스트 | SP6-ii (매물 검색 화면) — 지도 SDK 선택 |

## 컨텍스트

SP6-ii 매물 검색 화면에서 지도를 렌더하고 매물 핀을 표시해야 합니다.
한국 산업용 부동산 플랫폼 특성상 한국 행정구역·산업단지 정확도가 핵심입니다.
브라우저에서 직접 로드하는 JS SDK 방식이 필요하고, 무료 개발 quota 와 향후 V-World / 공시지가 레이어 통합 가능성을 고려합니다.

## 결정

**Naver Maps JavaScript SDK** 를 SP6-ii 의 지도 vendor 로 채택.

## 대안

| 기준 | Naver Maps | 카카오맵 | Google Maps |
|---|---|---|---|
| 한국 산업단지 정확도 | ◎ | ◎ | △ (해외 base) |
| 무료 quota (dev) | 10만 호출/월 | 30만 호출/월 | 28,000 호출/월 |
| 부동산 표준 | ◎ (네이버 부동산) | ○ | △ |
| 공시지가 / 산업단지 layer | 별도 | 별도 | X |
| API key 발급 | NCP 가입 필요 | 카카오 Dev 가입 | Google Cloud |
| 한국어 UI / docs | ◎ | ◎ | ○ |

- **카카오맵**: 무료 quota 더 많음. 그러나 네이버 부동산 사용자 친숙도 + 향후 Naver geocoding(PNU 매핑) 연계 가능성으로 기각.
- **Google Maps**: 무료 quota 가장 적고, 한국 산업단지 데이터 정확도 낮음. 해외 매물 추가 시 재검토.

## 결과

- **긍정**:
  - 네이버 부동산 = 한국 부동산 표준 — 사용자 친숙도 즉시 확보
  - KSURE / GIS 기반 산업단지 표시 정확도
  - 향후 V-World / 공시지가 layer 통합 시 Naver geocoding (PNU 매핑) 호환
  - 클러스터링 submodule 내장 — 다핀 UI 별도 구현 불필요
- **부정**:
  - NCP 계정·결제 수단 등록 필요 (free tier 초과 시 자동 과금)
  - CSP `script-src` / `img-src` / `connect-src` 에 Naver Maps domain 명시 필요
  - SSR 불가 (`'use client'` 강제) — Next.js App Router SEO 영향 없음 (지도 = 클라이언트 인터랙션)
- **영향 영역**:
  - `apps/web/lib/naver-maps.ts` — lazy SDK loader (동적 script 주입)
  - `apps/web/components/listings/listing-map.tsx` — Naver Maps 핀 + 클러스터
  - `apps/web/app/api/proxy/proxy.ts` — CSP header 추가
  - `.env.local` — `NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID`

## 재검토 트리거

- Naver Maps 무료 quota 초과 경보 발생 시 → SP7-i 의 quota alert + SP6-data-sync 배치 분리 먼저 적용, 그래도 초과 시 카카오맵 fallback 검토
- 해외 매물 추가 요구 시 → Google Maps 또는 Mapbox multi-vendor 별도 ADR
- NCP Maps 서비스 정책 변경(가격·API 중단) 시

## 참조

- → @docs/superpowers/specs/2026-05-05-sub-project-6-ii-listing-search-design.md
- → @docs/superpowers/plans/2026-05-05-sub-project-6-ii-listing-search.md
- → @docs/frontend/listings-search.md
- → @docs/adr/0003-frontend-nextjs-react19.md
- Naver Cloud Platform Maps: https://www.ncloud.com/product/applicationService/maps
