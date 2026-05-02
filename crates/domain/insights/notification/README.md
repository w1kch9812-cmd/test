# notification-domain

`Notification` 도메인 (Insights BC, RDS 동적) crate에요.

## 책임

- spec § 5.2 `notification` 테이블 매핑하는 Aggregate 정의해요.
- 사용자 알림 append-mostly (이벤트 발생 시 1 row INSERT).
- `kind`: ≤50자 식별 문자열 (예: `bookmark_listing_changed`, `auction_deadline_approaching`).
- `payload`: 이벤트 컨텍스트 (`JSONB`).
- `mark_read` 멱등 — 이미 읽은 알림 재호출 시 `read_at` 보존.
- 365일 retention — 워커가 더 오래된 row를 `DELETE`.
- `NotificationRepository` trait — 구현체는 sub-project 5에서 추가.

## 의존

- `shared-kernel` (`Id`, `UserMarker`, `NotificationMarker`).
