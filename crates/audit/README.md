# crates/audit

Immutable Audit Log — 모든 데이터 변경·인증 시도·외부 호출의 영구 기록.

## 책임
- audit_log 테이블 쓰기
- S3 Object Lock (immutable, retention 7년)
- 자동 마스킹 (PII 필드)
- audit 이벤트 스키마 (도메인별)
- 감사 조회 (관리자 전용)

## 의존
- `crates/db` — audit_log 테이블
- `crates/observability` — tracing
- AWS S3 (Object Lock 설정)

## 정책
- 모든 도메인 *write* 작업 = 자동 audit log (Repository wrapper)
- 모든 *인증* 시도/성공/실패 = audit
- 모든 *외부 API 호출* = audit (success/failure 둘 다)
- 모든 *PII 접근* = audit
- audit log는 *수정·삭제 불가* (Postgres + S3 둘 다)
- retention: 도메인 5년, 인증/PII 7년 (PIPA 요구)

→ @docs/security/README.md, → @docs/compliance/audit-log-immutable.md
