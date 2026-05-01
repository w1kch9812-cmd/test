# apps/admin-web

관리자/운영자 대시보드 (Next.js 16). 매물 검수, 사용자 관리, 데이터 ETL 모니터링.

## 의존
- `@gongzzang/api-client`, `@gongzzang/ui-web`, `@gongzzang/shared`, `@gongzzang/tsconfig`
- 추가 권한 미들웨어 (RBAC: ADMIN/OPERATOR)

## 정책
- LLM/MCP 의존성 금지
- 운영자 전용 (별도 도메인 또는 IP 제한)
- 모든 액션 audit log 기록

## 화면 (sub-project 6+)
- 매물 검수 큐
- 사용자 관리 (역할 부여, 사업자 검증)
- ETL 작업 상태 (V-World 동기화 등)
- V-World 쿼터 모니터링
- 신고된 매물 처리
- 시스템 헬스 (관측 대시보드 링크)
