//! V-World 레이어별 property 파서.
//!
//! 한 모듈 = 한 V-World 레이어. envelope (`crate::envelope`)이 features 배열만
//! 추출 후, 각 레이어 파서가 자기 properties → 도메인 entity 변환을 책임.
//!
//! 분리 이유 — V-World 가 새 레이어를 추가하거나 한 레이어 schema 가 변해도
//! envelope·geometry 코드는 무관. drift는 자기 모듈에 격리.
//!
//! 현재 지원:
//! - [`parcel_boundary`] — `LP_PA_CBND_BUBUN` (연속지적도, PNU 기반 단일 필지)
//!
//! 향후 (별도 spec):
//! - `use_zone` — `LT_C_UQ111` (도시지역 용도지역, spatial intersect)
//! - `building` — `LT_C_*` 건축물 레이어 등

pub mod parcel_boundary;
