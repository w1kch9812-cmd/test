# architecture/

시스템 아키텍처 SSOT. 세부는 각 파일로 분해 예정 (500줄 규칙).

## 문서 계획

| 파일 | 내용 | 상태 |
|------|------|------|
| data-flow.md | 사용자 요청 → 공식 API → DB 캐시 → 응답 | TODO |
| layers.md | Clean Architecture 계층 + 의존성 방향 | TODO |
| mcp-vs-api.md | 에이전트 경로 vs 프로덕션 경로 상세 | TODO |
| geo-pipeline.md | PostGIS 인덱싱·타일·공간쿼리 파이프라인 | TODO |
| caching.md | Redis 레이어, V-World TTL, 법령 캐시 | TODO |
| observability.md | Sentry + OTel + 로깅 | TODO |

## 현재 확정된 원칙

1. **2-레이어 데이터 접근** — [AGENTS.md §3](../../AGENTS.md)
2. **Clean Architecture** — UI → UseCase → Port → Adapter
3. **PostGIS가 공간 연산의 SSOT** — 클라이언트 Turf.js는 UX 보조만
4. **좌표계 규칙** — 저장 4326, 연산 5179, 타일 3857
5. **캐시 계층** — Redis(Hot) → PostgreSQL(Cold) → V-World(Origin)

## 초기 데이터 플로우 스케치

```
[User]
  ↓ HTTPS
[Next.js Edge/SSR]
  ↓ Server Action
[Use Case Layer]
  ├─▶ [PostGIS Repo] ─▶ Postgres + PostGIS
  ├─▶ [V-World Client] ─▶ api.vworld.kr (cached via Redis)
  ├─▶ [Law API Client] ─▶ open.law.go.kr
  └─▶ [OpenData Client] ─▶ data.go.kr

[Claude Code 세션] ─▶ MCP (korean-land / korean-law / opendata)
  └─ 개발·운영 조회 전용, 프로덕션 경로와 분리
```

세부 다이어그램은 data-flow.md에서.
