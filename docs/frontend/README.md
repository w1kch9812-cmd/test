# frontend/

Next.js + React + UI SSOT.

## 책임 영역
- Next.js 16 App Router (apps/platform-web, apps/admin-web)
- React 19 (Server Components + Client Components)
- TypeScript 5.7 strict
- Tailwind v4 + shadcn/ui + Radix UI
- TanStack Query (서버 상태)
- Zustand (클라이언트 상태)
- Naver Maps (packages/map)
- Canvas 2D 마커 (Phase 3, 수만 마커)
- PMTiles (Cloudflare R2 호스팅, Phase 3)
- PWA (manifest + service worker)
- 한국어 (i18n 미사용)
- 접근성 (WCAG 2.2 AA)
- CSP + Trusted Types
- 성능 예산 (Lighthouse CI)

## 작성 예정 문서 (sub-project 6+)
- `nextjs.md` — App Router + Server Action 패턴
- `shadcn-radix.md` — 디자인 시스템
- `tanstack-query.md` — 서버 상태
- `naver-maps.md` — 지도 통합 + Canvas 마커
- `canvas-markers.md` — 수만 마커 고속 렌더
- `pmtiles.md` — 벡터 타일 (Phase 3)
- `pwa.md` — manifest + service worker
- `i18n.md` — 한국어 단일, 향후 확장 자리
- `a11y-wcag.md` — 접근성 체크리스트
- `csp.md` — Content Security Policy
- `performance-budget.md` — Lighthouse CI

## 관련 ADR
- → @docs/adr/0003-frontend-nextjs-react19.md

## 관련 컨벤션
- → @docs/conventions/typescript.md
- → @docs/conventions/ui-writing-korean.md
