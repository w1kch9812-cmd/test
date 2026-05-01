# apps/worker

배치/크론 작업 전담.

- 런타임: Node.js (단독 프로세스, 또는 AWS Lambda / ECS)
- 트리거: 스케줄러 (cron, EventBridge)
- 의존: `@gongzzang/core`, `@gongzzang/data-clients`, `@gongzzang/db`

## 작업 (계획)

| 작업 | 주기 | 내용 |
|------|------|------|
| `vworld-cache-refresh` | 일일 03:00 | 인기 필지 V-World 데이터 재갱신 |
| `realprice-ingest` | 일일 04:00 | data.go.kr 실거래가 신규분 적재 |
| `building-register-sync` | 주간 일요일 02:00 | 건축물대장 변동분 동기화 |
| `cache-expire-sweep` | 시간당 | 만료 캐시 정리 |
| `audit-log-archive` | 월간 1일 | 감사 로그 S3 아카이브 |

## 정책

- 사용자 트래픽 경로와 분리 (DB는 같은 거 사용)
- V-World 쿼터 영향 큰 작업은 [AGENTS.md §6](../../AGENTS.md) 사용자 확인 필수
