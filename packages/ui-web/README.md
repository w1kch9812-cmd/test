# packages/ui-web

공통 UI 컴포넌트 + 디자인 토큰 (shadcn/ui + Radix UI + Tailwind v4).

## 의존
- `react`, `react-dom` (peer)
- `@radix-ui/*`
- `tailwindcss`, `tailwind-variants`, `class-variance-authority`
- `lucide-react`, `@phosphor-icons/react`
- `@gongzzang/tsconfig`

## 제공 (sub-project 6+)
- **Primitives**: Button, Input, Select, Checkbox, Slider, Popover, Modal, Tabs, DatePicker
- **Layout**: Header, Sidebar, Container
- **Data**: DataTable, Filter, Pagination
- **Tokens**: color (oklch), spacing, motion, z-index

## 정책
- 모든 컴포넌트 키보드 접근 가능 (WCAG 2.2 AA)
- 다크모드 동시 지원 (Tailwind dark:)
- 한국어 해요체 기본
- 출처 표기 컴포넌트 (`<DataSourceCredit />`) — 공공데이터 표시 의무
- Storybook (Phase 3+) + Lost Pixel 시각 회귀

→ @docs/frontend/shadcn-radix.md
