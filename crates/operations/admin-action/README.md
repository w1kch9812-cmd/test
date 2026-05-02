# admin-action-domain

`AdminAction` 도메인 (Operations BC, RDS 동적) crate에요.

## 책임

- spec § 5.5 `admin_action` 테이블 매핑하는 Aggregate 정의해요.
- **불변 (immutable) — append-only**. 어드민 액션은 한 번 기록되면 수정/삭제 불가에요.
- `target_kind` / `target_id` 는 *둘 다 Some 또는 둘 다 None* (도메인 invariant).
- `correlation_id` 분산 추적용 — 한 어드민 작업의 모든 액션이 묶여요.
- `AdminActionRepository` trait — `insert` + 3개 read만. `save`/`update`/`delete` *없음*.

## 불변 (immutable) 보장

`AdminAction` 은 **mutation 메서드가 하나도 없어요**. `try_new` 로 생성한 뒤
어떤 필드도 바꿀 수 없어요. 이건 `AuditLog` 와 같은 의도된 설계로, 어드민
운영 액션의 추적성과 무결성을 컴파일 시점에 보장해요.

## 의존

- `shared-kernel` (`Id`, `UserMarker`, `AdminActionMarker`).
