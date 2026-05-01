# apps/web

메인 사용자용 데이터 플랫폼 (옵션 A).

- 프레임워크: Next.js 16 (App Router)
- 사용자: 일반 사용자, 인증된 회원
- 의존: `@gongzzang/core`, `@gongzzang/data-clients`, `@gongzzang/ui`, `@gongzzang/db`
- LLM/MCP 의존성: **금지** ([AGENTS.md §3](../../AGENTS.md))

## 화면 (계획)

- `/` 랜딩
- `/map` 지도 + 필지 검색
- `/parcel/[pnu]` 필지 상세 (용도지역, 지구단위계획, 도시계획시설)
- `/law` 법령 검색·열람
- `/dashboard` 사용자 대시보드 (저장한 필지)

## 부트스트랩 (TODO)

```bash
# 다음 작업 세션에 진행
pnpm dlx create-next-app@latest apps/web --typescript --tailwind --app --src-dir
```
