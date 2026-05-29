# korean-land-mcp (에이전트 경로)

- 소스: https://github.com/UrbanWatcherKr/korean-land-mcp
- 라이선스: MIT
- 역할: **V-World API를 MCP로 래핑** (AI 에이전트의 개발 탐색용)
- 프로덕션 사용 금지 — [AGENTS.md §3](../../AGENTS.md)
- Catalog facts and source ingestion remain Platform Core-owned. This MCP is
  not a Gongzzang runtime data source, admin lookup backend, or SSOT.

## 제공 도구 (7 + discover)

| 도구 | 기능 |
|------|------|
| `resolve_parcel` | 주소/PNU/지번을 좌표·행정구역으로 표준화 |
| `get_zoning` | 용도지역 (도시/관리/농림/자연환경) + 8개 세부 카테고리 |
| `get_district_plan` | 지구단위계획, 개발제한구역 |
| `get_urban_facility` | 도시계획시설 9종 + 거리 필터링 |
| `get_other_law_designations` | 42개 법적지정 (농업/임업/산업단지/환경 등) |
| `get_land_attributes` | 지목 28종, 공시가격, 건축물 데이터 |
| `analyze_parcel` | 위 6개를 병렬 실행 + 워크플로우 힌트 |
| `discover_tools` | 자연어로 도구 탐색 |

## 특이 로직

- **국토계획법 §76(5) 우선위임 자동 감지**: 용도지역 간 충돌 시 자동 플래그
- **Honest failure**: V-World 5xx 노출, Mock 금지
- **Point-based**: 필지 중심점 기준 → 경계 걸친 케이스 한계 있음
- **행정조례는 미포함** → korean-law-mcp 병용

## 설치

프로젝트에서는 `.mcp.json` 을 통해 자동 구성 예정.

수동 설치:
```bash
git clone https://github.com/UrbanWatcherKr/korean-land-mcp .mcp-local/korean-land-mcp
cd .mcp-local/korean-land-mcp
npm install && npm run build
```

`.env`:
```
VWORLD_API_KEY=...
VWORLD_DOMAIN=localhost
```

## 사용 예시 (Claude Code)

```
@korean-land 서울 강남구 테헤란로 123 용도지역 알려줘
@korean-land analyze_parcel 로 이 필지 전체 분석해줘
```

## 법규 해석이 필요할 때

`analyze_parcel` 결과 → [korean-law-mcp](./korean-law-mcp.md) 로 `verify_citations` → 최종 결론.

## 한계

- V-World 의존 (V-World 다운 시 전체 먹통)
- 응답 속도: V-World 레이턴시 + MCP 오버헤드
- 동시성: MCP 인스턴스당 직렬
