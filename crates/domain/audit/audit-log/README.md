# audit-log-domain

`AuditLog` 도메인 (Audit BC, RDS 동적) crate에요.

## 책임

- spec § 5.3 `audit_log` 테이블 매핑하는 Aggregate 정의해요.
- **불변 (immutable) — append-only**. V002 트리거가 `UPDATE`/`DELETE`를 차단해요.
- `actor_id` `None` = 시스템 행위 (system action).
- `before_state` / `after_state` 는 `JSONB` 스냅샷.
- `correlation_id` 분산 추적용 — 한 요청의 모든 audit 묶이게 해요.
- `AuditLogRepository` trait — `insert` + 3개 read만. `save`/`update`/`delete` *없음*.

## 불변 (immutable) 보장

`AuditLog` 는 **mutation 메서드가 하나도 없어요**. `try_new` 로 생성한 뒤
어떤 필드도 바꿀 수 없어요. 이건 V002 immutable trigger 와 함께 컴파일 + 런타임
양쪽에서 audit 무결성을 보장하기 위한 *의도된 설계* 에요.

## 의존

- `shared-kernel` (`Id`, `UserMarker`, `AuditLogMarker`).
