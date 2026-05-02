# 공짱 (Gongzzang)

> 산업용 부동산 정보 플랫폼 — 공장·창고·산업단지·지식산업센터·사옥 매물과 제조업체 정보를 통합 제공합니다.

## 개요

매수자(투자자·기업), 매도자, 공인중개사, 시행사가 사용하는 **B2B 산업 부동산 통합 플랫폼**입니다.

- **차별점**: 산업용 특화 + V-World/법제처/공공데이터포털 등 공공 데이터 자동 보강 + 입지·규제 분석
- **시장**: 한국 (i18n 인프라 없음, 한국어 + KRW + KST 고정)
- **수익**: 광고 + 구독 (확장: 등록비, 데이터 판매)
- **디바이스**: 반응형 웹 + PWA (추후 네이티브 앱)

## 기술 스택 (요약)

| 영역 | 선택 |
|------|------|
| 백엔드 | Rust + Axum + SQLx |
| 프론트엔드 | Next.js 16 + React 19 + TypeScript |
| DB | PostgreSQL 17 + PostGIS |
| 지도 | Naver Maps |
| 인증 | Zitadel (OIDC/OAuth2) |
| 캐시 | moka (L1) + Valkey (L2) |
| 코드 스타일 | Biome v2.4 |
| 모노레포 | Cargo workspace + pnpm + Turborepo |
| IaC | Pulumi (TypeScript) |
| 관측성 | Grafana + Prometheus + Loki + Tempo + Sentry + OpenTelemetry |

상세: [TECH.md](./TECH.md)

## 빠른 시작

```bash
# 의존성 설치
pnpm install

# Rust toolchain 설치 (한 번만)
rustup install 1.85.0 && rustup default 1.85.0

# 로컬 환경 변수
cp .env.example .env  # 값 채우기

# 개발 서버 (sub-project 6 이후)
pnpm dev
```

## 프로젝트 진입점

- [AGENTS.md](./AGENTS.md) — AI 에이전트 라우터 (필수, 모든 도구가 먼저 읽음)
- [CLAUDE.md](./CLAUDE.md) — Claude Code (1줄 위임)
- [TECH.md](./TECH.md) — 기술 스택 + SSOT 맵
- [docs/](./docs/) — 도메인별 SSOT 문서
- [MEMORY.md](./MEMORY.md) — 자동 메모리 인덱스

## SSS 7 기둥

이 프로젝트는 *하이엔드 엔터프라이즈 SSS급* 품질을 목표로 합니다:

1. 일관성 / 2. 자동 강제 / 3. 추적성 / 4. 안전성 / 5. 가시성 / 6. SSOT / 7. 명확성

상세: → `docs/sss-charter.md` (작성 예정)

## 라이선스

[LICENSE](./LICENSE) — Proprietary (사내 비공개). 외부 의존성 라이선스는 `deny.toml`로 자동 검증.

## 문의

내부 문서: [docs/governance/](./docs/governance/)
