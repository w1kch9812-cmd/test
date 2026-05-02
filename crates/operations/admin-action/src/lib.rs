//! `AdminAction` 도메인 (Operations BC, RDS 동적).
//!
//! 어드민 운영 액션의 감사 로그. **append-only** — 한 번 기록되면
//! `UPDATE`/`DELETE` 가 불가능하도록 도메인 모델 자체에서 mutation 메서드를
//! 노출하지 않아요 (`AuditLog` 와 같은 설계).
//!
//! `AdminAction` Aggregate 는 *mutation 메서드를 노출하지 않아요* — `try_new` 로
//! 생성 후 어떤 필드도 바꿀 수 없어요 (의도된 설계).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod entity;
pub mod errors;
pub mod repository;
