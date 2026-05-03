# ADR-0012: 파이프라인 시각화 — React Flow (xyflow)

| | |
|---|---|
| 작성일 | 2026-05-03 |
| 상태 | Accepted |
| 결정자 | 운영자 |

## 컨텍스트

sub-project 2 brainstorming D19에서 *시각화 = Grafana embed (서비스 맵·트레이스·메트릭) + 자체 UI (파이프라인 단계 진행)* 두 트랙으로 분리하기로 결정했어요. 이 ADR은 그중 **자체 UI — 파이프라인 단계 진행 노드 그래프** 렌더 라이브러리를 결정합니다.

요구사항:
- `pipeline_run.steps` JSONB 배열을 노드 그래프로 렌더 (각 step = 노드, `order`로 순서)
- 각 노드 안에 한국어 라벨, 상태 배지(`pending|running|success|failed|skipped`), 진행률 바, 메트릭 카운터 표시
- 어드민 UI(`apps/admin-web/.../data-pipeline-control/`)에 임베드, Tailwind/shadcn 디자인 시스템과 일관성 유지
- 라이브 진행 갱신(transport는 별도 결정)
- 현재는 `order` 기반 단일 흐름, 향후 분기/병렬 가능성

제약:
- ADR-0003 — Next.js 16 + React 19 호환
- 메인 시스템(MCP/LLM 의존성 0) 규칙은 UI 라이브러리에는 무관

## 결정

**React Flow** (`@xyflow/react`, MIT) 채택.

- xyflow 회사(webkid GmbH, Berlin)가 2019년부터 풀타임 유지보수
- DOM 기반 — 노드를 React 컴포넌트로 직접 작성 → Tailwind/shadcn 그대로 사용 가능
- React Flow Pro(유료 Slack 지원·고급 예제)는 **불필요**, MIT 무료 부분으로 충분
- 레이아웃 알고리즘(dagre / elkjs / 수동)은 **본 ADR 범위 외** — 현재 단일 흐름은 수동 위치, 분기 도입 시 별도 결정

## 대안

- **Cytoscape.js** (Canvas, MIT, 14종 레이아웃 내장): 노드를 Canvas에 직접 그려야 해서 Tailwind/shadcn 컴포넌트를 노드 안에 넣기 어려움. 디자인 시스템 일관성 저해. 기각.
- **G6 (AntV)** (Canvas, MIT): 위와 동일한 Canvas 한계. React 통합도 약함. 기각.
- **Sigma.js** (WebGL, MIT): 1만+ 노드 대규모 그래프 전용. 우리 케이스(5~15 노드)엔 오버킬, 노드 커스터마이징도 매우 제한적. 기각.
- **Reaflow** (SVG, Apache-2.0): React 컴포넌트 노드 지원하지만 유지보수 정체, 커뮤니티 규모 작음. 기각.
- **Mermaid** (텍스트→SVG, MIT): 정적 다이어그램용. 라이브 진행률·노드 인터랙션 불가. 기각.
- **D3 직접 구현** (BSD, 무한 커스텀): 시간 비용 큼, 우리가 필요한 기능(zoom/pan/selection)을 처음부터 작성해야 함. 기각.

## 결과

- **긍정**:
  - 노드 안에 한국어 UI·상태 배지·shadcn 컴포넌트를 React로 그대로 작성 → 디자인 시스템 일관성
  - React 19 + Next.js 16 호환 검증됨
  - xyflow 회사가 풀타임 운영, 2019년부터 안정적 유지보수
  - MIT 라이선스 — 비용 0
  - zoom/pan/minimap/selection 등 인터랙션 기본 제공
  - 레이아웃 엔진 교체 가능 (수동 → dagre → elkjs)
- **부정**:
  - Next.js App Router에서 `'use client'` 필수 (SSR 안 됨) — 어드민은 어차피 클라이언트 인터랙션 위주라 영향 없음
  - 1000+ 노드 시 DOM 렌더 성능 한계 — 우리 케이스엔 무관
  - 번들 크기 ~100KB (gzipped) 추가
- **영향 영역**:
  - `apps/admin-web/.../data-pipeline-control/` — 파이프라인 schedule + run 화면
  - `package.json`(admin-web) — `@xyflow/react` 의존성 추가
  - 향후 다른 노드 그래프 시각화 필요 시(예: BC 의존성 그래프, 데이터 lineage) 같은 라이브러리 재사용

## 재검토 트리거

- 노드 수 1,000+ 또는 페이지 렌더 지연 발생 시 → Sigma.js (WebGL) 또는 Cytoscape Canvas 모드로 전환 검토
- xyflow 회사·라이브러리 유지보수 정체 (12개월+ 릴리스 없음) 시 → 대안 재평가
- DAG 분기/병렬 도입 후 dagre/elkjs 통합이 React Flow와 충돌 시 → 별도 ADR
- React Flow Pro 기능이 필수가 되는 요구사항 발생 시 → 비용 vs 자체 구현 평가

## 참조

- → @docs/superpowers/specs/2026-05-02-sub-project-2-db-core-domain-design.md (D19 결정, § 5.4 `pipeline_run.steps` 스키마)
- → @docs/database/er-diagram-v001.md (line 100 — "admin UI 노드 그래프")
- → @docs/adr/0003-frontend-nextjs-react19.md (프론트엔드 스택)
- → @docs/adr/0008-observability-grafana-otel-sentry.md (Grafana embed 트랙)
- React Flow: https://reactflow.dev/
- xyflow: https://xyflow.com/
- License: MIT (https://github.com/xyflow/xyflow/blob/main/LICENSE)
