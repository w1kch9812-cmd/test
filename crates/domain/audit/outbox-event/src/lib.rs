//! `OutboxEvent` 도메인 (System BC, RDS 동적).
//!
//! Transactional outbox 패턴의 application-side 컴포넌트에요. `Aggregate` 도메인 메서드가
//! `DomainEvent` 를 emit 하면, application handler 가 `OutboxEvent::from_domain` 으로
//! wrap 해 *같은 트랜잭션* 안에서 `OutboxRepository::save` 로 INSERT 해요. Publisher
//! 워커 (sub-project 4) 가 미발행 row 를 polling 해 외부 시스템에 발행 후
//! `mark_published` 를 호출해요.
//!
//! - ID: `evt_<26-char ULID>` (spec § 5.3 inline comment 준수)
//! - `mark_published` 는 idempotent — 이미 발행된 경우 변경 없음.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod entity;
pub mod errors;
pub mod repository;
