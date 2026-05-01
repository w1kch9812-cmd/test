# packages/shared

공용 React 훅 + 유틸 + 타입 (Next.js 앱들이 공유).

## 의존
- `react`, `@gongzzang/tsconfig`

## 제공 (sub-project 6+)
- **Hooks**: useDebounce, useLocalStorage, useMediaQuery, useIntersectionObserver
- **Utils**: formatKrw, formatArea (㎡↔평), formatDate (KST), maskPii
- **Types**: 환경 변수 타입 안전 접근 (`env.ts`)

## 정책
- 외부 의존 최소
- 모든 함수는 *순수* (side effect 없음)
- React 훅은 `use*` 접두사
- 한국 특화 포맷터: 한국식 천 단위 (8억 5,000만원), KST 시간

## 금지
- 비즈니스 로직 (Rust로 위임)
- 외부 API 호출 (api-client만)
- DOM 직접 조작 (React 표준 사용)
