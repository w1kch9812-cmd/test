# apps/admin

운영자/관리자 대시보드.

- 프레임워크: Next.js 16
- 사용자: 내부 운영자, 데이터 큐레이터
- 권한: RBAC (MASTER_ADMIN, OPERATOR)
- 의존: `@gongzzang/core`, `@gongzzang/data-clients`, `@gongzzang/ui`

## 기능 (계획)

- 공공데이터 ETL 작업 모니터링
- V-World 쿼터 사용량 조회
- 사용자 관리, 권한 부여
- 콘텐츠 큐레이션 (특정 지역 분석 결과 정리)
- 감사 로그 조회

## MCP 사용 정책

이 앱은 옵션 A 메인 시스템에 포함됨. **MCP/LLM import 금지**.
관리자 본인이 Claude Code로 도메인 탐색하는 건 별개.
