//! shared-kernel — 공짱 도메인 공유 값 객체.
//!
//! `Pnu`, `Money`, `Area`, `BusinessNumber` 등 BC 간 공통 타입을 정의해요.
//! 각 값 객체는 후속 task (12-25)에서 TDD로 점진 추가됩니다.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod id;
