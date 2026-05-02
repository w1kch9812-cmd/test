# operations-meta-domain

`FeaturedContent` + `SystemAlert` 도메인 합본 crate에요 (Operations BC, RDS 동적).

## 책임

- spec § 5.5 `featured_content` + `system_alert` 두 테이블 매핑하는 두 Aggregate 정의해요.
- **No OCC** — 두 Aggregate 모두 `version` 컬럼 없이 단순 UPDATE 사용.
- 단일 `OperationsMetaRepository` trait — 두 Aggregate 모두 Operations BC 의 *meta* 테이블이라 묶었어요 (워크플로우 X).

## FeaturedContent (홈페이지 추천/광고/스폰서)

- ID prefix **`fea`** — spec inline 은 `fc_` 로 적혀있지만 본 프로젝트 30자 ID 불변식 (3-char prefix) 충족 위해 `fea` 사용. Spec FU 11 에서 reconcile 예정.
- `target_kind` 3값 — `listing` / `industrial_complex` / `manufacturer`.
- `feature_kind` 4값 — `homepage_featured` / `search_top` / `sponsored_marker` / `newsletter`.
- **V003_03 invariant** — `ends_at > starts_at` (DB CHECK 동시).
- `is_active_at(t)` — `starts_at <= t < ends_at` 인지 검사.
- `record_impression` / `record_click` — saturating 카운터 (race 허용).

## SystemAlert (시스템 알림)

- ID prefix **`sal`** — spec inline 일치.
- `severity` 4값 — `info` / `warning` / `error` / `critical`. `is_actionable()` = `Error|Critical`.
- `acknowledge(by, at)` — 1회만 (재호출 시 `AlreadyAcknowledged`).
- `resolve(at)` — 1회만 (재호출 시 `AlreadyResolved`). 사전 acknowledge 불필요 (auto-resolved 가능).
- `metadata` JSONB — 호출자가 자유롭게 채움.

## 의존

- `shared-kernel` (`Id`, `UserMarker`, `FeaturedContentMarker`, `SystemAlertMarker`).
