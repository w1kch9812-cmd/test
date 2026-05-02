# lrq-domain

`ListingReviewQueue` (LRQ) 도메인 (Operations BC, RDS 동적) crate에요.

## 책임

- spec § 5.5 `listing_review_queue` 테이블 매핑하는 Aggregate 정의해요.
- **Decision-based workflow** — `decision = None` (pending) →
  `Some(Approve)` / `Some(Reject)` / `Some(RequestChanges)` (terminal).
- **12h SLA** — `submitted_at + 12h` 로 `sla_due_at` 자동 계산.
- **Optimistic concurrency** — `version` 컬럼 (`bigint`) 으로 동시 검토 충돌 차단.
- **Auto check** — 룰 기반 자동 점수 (`auto_check_score` 0-100) 와 플래그
  (`auto_check_flags`, 예: `["suspected_duplicate", "price_anomaly"]`) 를 큐 생성 시 기록.
- 검토자 (`reviewer_id`) 와 검토 메모 (`reviewer_note`) 는 결정 시점에 기록.
- `reject` / `request_changes` 는 **메모 필수** (사용자에게 사유 안내).
- `LrqRepository` trait — `find_by_id` / `find_pending` / `find_by_listing` / `save` (OCC).

## 결정 워크플로우

```text
                   decide_approve
        Pending ─────────────────►  Approved (terminal)
        (None)  ─────────────────►  Rejected (terminal)
                   decide_reject
                ─────────────────►  RequestChanges (terminal)
                   decide_request_changes
```

- 한 번 `decision` 이 `Some(_)` 으로 채워지면 **이후 모든 결정 시도는 `AlreadyDecided`** 에러.
- BVQ 와 달리 LRQ 는 `status` enum 없이 *Optional `LrqDecision`* 으로 pending/decided 구분.

## 의존

- `shared-kernel` (`Id`, `UserMarker`, `ListingMarker`, `LrqMarker`).
