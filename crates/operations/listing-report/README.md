# listing-report-domain

`ListingReport` 도메인 (Operations BC, RDS 동적) crate에요.

## 책임

- spec § 5.5 `listing_report` 테이블 매핑하는 Aggregate 정의해요.
- **4-status workflow** — `open` → `investigating` → `confirmed` / `dismissed` (terminal).
- **No OCC** — admin 신고 처리는 동시 충돌이 드물어 `version` 컬럼 없이 단순 UPDATE 사용.
- **익명 신고 허용** — `reporter_id` 가 `NULL` 이면 익명. 로그인한 사용자는 `Some(usr_…)` 으로 기록.
- **6 신고 사유** — `fake_listing` / `wrong_price` / `wrong_location` / `inappropriate_content` / `spam` / `other`.
- **handler 메모 필수** — `confirmed` / `dismissed` 로 마감할 때 `handler_note` 비어있으면 거부.
- `ListingReportRepository` trait — `find_by_id` / `find_open` / `find_by_listing` / `save`.

## 상태 워크플로우

```text
                    mark_investigating
        Open ────────────────────────────► Investigating
         │                                       │
         │ mark_confirmed / mark_dismissed       │ mark_confirmed / mark_dismissed
         ▼                                       ▼
       Confirmed / Dismissed (terminal — `resolved_at` 기록)
```

- `Open` / `Investigating` 은 non-terminal, `Confirmed` / `Dismissed` 는 terminal.
- terminal 상태에서 모든 `mark_*` 시도는 `InvalidTransition` 에러.
- `mark_confirmed` / `mark_dismissed` 는 `handler_note` 가 비어있거나 (trim 후) 2000자 초과면 거부.

## 의존

- `shared-kernel` (`Id`, `UserMarker`, `ListingMarker`, `ListingReportMarker`).
