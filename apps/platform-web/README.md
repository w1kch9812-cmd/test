# apps/platform-web

메인 사용자 사이트 (Next.js 16, App Router) — 매수자/매도자/중개사/시행사/기업.

## 의존
- `@gongzzang/api-client` — Rust API 자동 생성 SDK
- `@gongzzang/ui-web` — shadcn/ui 디자인 시스템
- `@gongzzang/map` — 네이버 지도 + Canvas 마커
- `@gongzzang/shared` — 공용 훅·유틸
- `@gongzzang/tsconfig` — TS 설정

## 정책
- LLM/MCP import **금지** (옵션 A 준수)
- 비즈니스 로직 0줄 (Server Action = 얇은 프록시만)
- PWA + 반응형 (Phase 1)
- 한국어 단일 (i18n 미사용)

## 화면 (sub-project 6+)
- `/` 랜딩 + 매물 검색
- `/listings/[id]` 매물 상세
- `/parcels/[pnu]` 필지 분석
- `/companies/[id]` 제조업체 상세
- `/dashboard` 사용자 대시보드
- `/auth/*` 로그인/가입 (Zitadel)
