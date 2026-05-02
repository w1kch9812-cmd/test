# outbox-event-domain

`OutboxEvent` 도메인 (System BC, RDS 동적) crate에요.

## 책임

- spec § 5.3 `outbox_event` 테이블 매핑하는 Aggregate 정의해요.
- `Aggregate` 도메인 메서드가 `DomainEvent` 를 emit 하면, application layer 에서
  `OutboxEvent::from_domain` 으로 wrap 해 *Aggregate save 와 같은 트랜잭션* 에서
  `OutboxRepository::save` 로 INSERT 해요 (transactional outbox 패턴).
- Publisher 워커 (sub-project 4) 가 미발행 row 를 polling 해 외부 시스템 발행 후
  `mark_published` 호출.
- `mark_published` 는 *idempotent* — 이미 발행된 경우 변경 없음.

## ID

`evt_<26-char ULID>` (총 30자) — spec § 5.3 inline comment (`evt_...`) 준수.

## 의존

- `shared-kernel` (`Id`, `OutboxEventMarker`, `domain_event::DomainEvent`).
