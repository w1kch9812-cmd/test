# business-verification-queue-domain

`BusinessVerificationQueue` (BVQ) 도메인 (Operations BC, RDS 동적) crate에요.

## 책임

- spec § 5.5 `business_verification_queue` 테이블 매핑하는 Aggregate 정의해요.
- **4-status workflow** — `pending` → `approved`/`rejected`/`needs_more_info`,
  `needs_more_info` → `pending` (재제출).
- **24h SLA** — `submitted_at + 24h` 로 `sla_due_at` 자동 계산.
- **Optimistic concurrency** — `version` 컬럼 (`bigint`) 으로 동시 검토 충돌 차단.
- 검토자 (`reviewer_id`) 와 검토 메모 (`reviewer_note`) 는 상태 전이 시점에 기록.
- `reject` / `request_more_info` 는 **메모 필수** (사용자에게 사유 안내).
- `BvqRepository` trait — `find_by_id` / `find_pending` / `find_by_user` / `save` (OCC).

## 상태 머신

```text
                    approve
        Pending  ─────────────►  Approved (terminal)
           │  ─────────────►     Rejected (terminal)
           │     reject
           │  ─────────────►     NeedsMoreInfo
           │     request_more_info
           ▲                          │
           └──────────────────────────┘
                resubmit
```

- `Approved` / `Rejected` 는 terminal — 어떤 전이도 허용 안 해요.
- `resubmit` 은 `submitted_documents` 를 새 R2 키로 교체하고 reviewer 필드를 초기화해요.

## 의존

- `shared-kernel` (`Id`, `UserMarker`, `BvqMarker`, `BusinessNumber`).
