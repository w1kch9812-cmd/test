# architecture/

시스템 아키텍처 SSOT. 세부는 각 파일로 분해 예정 (500줄 규칙).

## 문서 계획

| 파일 | 내용 | 상태 |
|------|------|------|
| data-flow.md | 사용자 요청 → Gongzzang API → Platform Core contract / Gongzzang DB → 응답 | Active |
| layers.md | Clean Architecture 계층 + 의존성 방향 | Active |
| mcp-vs-api.md | 에이전트 경로 vs 프로덕션 경로 상세 | Active |
| geo-pipeline.md | PostGIS 인덱싱·타일·공간쿼리 파이프라인 | Active |
| caching.md | Redis 레이어, Platform Core contract cache, 법령 캐시 | Active |
| observability.md | Sentry + OTel + 로깅 | Active |
| traffic-auth-policy-registry/ | Traffic/Auth 정책 fragment SSOT. `traffic-auth-policy-registry.v1.json`은 생성 aggregate | Active |

## 현재 확정된 원칙

1. **3-service 데이터 접근** — [AGENTS.md §0.5](../../AGENTS.md)
2. **Clean Architecture** — UI → UseCase → Port → Adapter
3. **Platform Core가 Catalog SSOT** — parcel geometry, PNU anchors, public/reference spatial layers
4. **Gongzzang은 B2C product semantics SSOT** — listing, listing photo, user, market, insights
5. **좌표계 규칙** — Platform Core contract 가 SRID를 명시하고, Gongzzang read model은 사본임을 드러냄

## 초기 데이터 플로우 스케치

```
[User]
  ↓ HTTPS
[Next.js Edge/SSR]
  ↓ Server Action
[Gongzzang Use Case Layer]
  ├─▶ [PostGIS Repo] ─▶ Postgres + PostGIS
  ├─▶ [Platform Core Client] ─▶ Catalog / Workforce published contracts
  ├─▶ [Law API Client] ─▶ open.law.go.kr
  └─▶ [Gongzzang-owned External API Client] ─▶ product-owned external source

[Claude Code 세션] ─▶ MCP (korean-land / korean-law / opendata)
  └─ 개발·운영 조회 전용, 프로덕션 경로와 분리
```

세부 다이어그램은 data-flow.md에서.
