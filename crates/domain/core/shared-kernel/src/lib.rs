//! shared-kernel — 공짱 도메인 공유 값 객체.
//!
//! `Pnu`, `Money`, `Area`, `BusinessNumber` 등 BC 간 공통 타입을 정의해요.
//! 각 값 객체는 후속 task (12-25)에서 TDD로 점진 추가됩니다.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod address;
pub mod admin_division;
pub mod area;
pub mod broker_license;
pub mod business_number;
pub mod contact_visibility;
pub mod description;
pub mod email;
pub mod geometry;
pub mod id;
pub mod ksic_code;
pub mod land_use_type;
pub mod listing_status;
pub mod listing_title;
pub mod listing_type;
pub mod money;
pub mod phone_kr;
pub mod pnu;
pub mod srid;
pub mod time;
pub mod transaction_type;
pub mod zoning;
