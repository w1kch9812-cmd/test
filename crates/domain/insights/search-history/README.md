# search-history-domain

`SearchHistory` 도메인 (Insights BC, RDS 동적) crate에요.

## 책임

- spec § 5.2 `search_history` 테이블 매핑하는 Aggregate 정의해요.
- 사용자 검색 이력 append-mostly 기록 (매 검색마다 1 row INSERT).
- `BRIN` 인덱스 (`created_at`) 기반 시간순 조회 가정.
- `PIPA` 가명화 — `created_at` 90일 경과 시 `user_id` → `NULL`.
- 1년 retention — 더 오래된 row는 워커가 `DELETE`.
- `SearchHistoryRepository` trait — 구현체는 sub-project 5에서 추가.

## 의존

- `shared-kernel` (`Id`, `UserMarker`, `SearchHistoryMarker`).
