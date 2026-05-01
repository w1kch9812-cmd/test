# @gongzzang/ui

공통 UI 컴포넌트 + 디자인 토큰.

## 도구 (계획)

- React 19
- TailwindCSS v4
- shadcn/ui 베이스
- Radix UI primitives
- Framer Motion (선택, 애니메이션)
- MapLibre GL JS (지도 컴포넌트)

## 컴포넌트 (계획)

```
src/
├── primitives/         ← Button, Input, Dialog, ...
├── layout/             ← Header, Sidebar, ...
├── data/               ← DataTable, Filter, ...
├── map/                ← MapView, MarkerCluster, ...
├── chart/              ← BarChart, ...
└── tokens/             ← color, spacing, motion
```

## 정책

- 한국어 해요체 기본 ([AGENTS.md §9](../../AGENTS.md))
- 다크모드 동시 지원
- WCAG 2.1 AA 접근성
- 출처 표기 컴포넌트 (`<DataSourceCredit />`) 의무 사용
