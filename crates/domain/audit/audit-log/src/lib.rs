//! `AuditLog` 도메인 (Audit BC, RDS 동적).
//!
//! 모든 사용자/시스템 행위의 감사 로그. **append-only** — V002 immutable trigger 가
//! `UPDATE`/`DELETE` 를 DB 레벨에서 차단해요.
//!
//! `AuditLog` Aggregate 는 *mutation 메서드를 노출하지 않아요* — `try_new` 로 생성 후
//! 어떤 필드도 바꿀 수 없어요 (의도된 설계).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod entity;
pub mod errors;
pub mod repository;
