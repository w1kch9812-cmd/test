---
name: 기술 스택 확정
description: 2026-05-01 기준 sub-project 1에서 확정된 기술 스택
type: project
---

## 무거운 결정 (사실상 못 바꿈)

- **백엔드**: Rust 1.85+ + Axum + SQLx + Tokio
- **프론트엔드**: Next.js 16 + React 19 + TypeScript 5.7 strict
- **DB**: PostgreSQL 17 + PostGIS
- **지도**: Naver Maps SDK
- **시장**: 한국만 (i18n 없음, KRW + KST 고정)
- **범위**: 옵션 A 데이터 플랫폼 (AI 텍스트 생성 X, 임베딩 OK)

## 가벼운 결정 (Phase 1엔 이걸로 시작, 필요 시 전환 가능)

- **인증 IdP**: Zitadel (vs Keycloak — 운영 가벼움 우선) — ADR-0005
- **코드 스타일**: Biome v2.4 단독 (vs ESLint+Prettier — Vercel/Google 채택)
- **캐시**: moka L1 + Valkey L2 (vs ElastiCache Redis — 라이선스 안전)
- **검색**: PostgreSQL FTS (Phase 1) → Meilisearch (Phase 3)
- **임베딩**: Gemini Embedding 2 + pgvector (Phase 3+)
- **IaC**: Pulumi TypeScript

## 인프라 (Phase 따라 단계적 확장)

- **호스팅**: AWS Seoul + Cloudflare Free (CDN/WAF/DDoS 무료)
- **컨테이너**: ECS Fargate (Phase 1-3) → EKS (Phase 4+)
- **관측**: Grafana Cloud Free → Sentry 셀프호스트 → 풀스택 (Phase 3+)

**Why**: 사용자가 SSS 엔터프라이즈 + Rust 강하게 선호 + 시간 무관 + 비용은 돈만 고려.
**How to apply**:
- 무거운 결정은 *변경 없이* 진행
- 가벼운 결정은 운영 6개월 후 재평가 가능
- 모든 결정은 ADR 0001-0011에 영구 기록
