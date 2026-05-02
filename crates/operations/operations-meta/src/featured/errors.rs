//! `FeaturedContent` 도메인 에러.

use thiserror::Error;

/// `FeaturedContent` Aggregate 검증 에러.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum FeaturedContentError {
    /// `target_id` 가 비어있음 (trim 후).
    #[error("target_id cannot be empty")]
    EmptyTargetId,
    /// `target_id` 가 50자 초과 (DB `varchar(50)` 한계).
    #[error("target_id exceeds 50 chars (got {actual})")]
    TargetIdTooLong {
        /// 실제 길이.
        actual: usize,
    },
    /// `weight` 가 음수.
    #[error("weight must be >= 0 (got {actual})")]
    NegativeWeight {
        /// 실제 weight.
        actual: i32,
    },
    /// `ends_at <= starts_at` — V003_03 invariant 위반.
    #[error("ends_at must be strictly after starts_at")]
    InvalidTimeBound,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_target_id_message() {
        assert_eq!(
            FeaturedContentError::EmptyTargetId.to_string(),
            "target_id cannot be empty"
        );
    }

    #[test]
    fn target_id_too_long_message() {
        assert_eq!(
            FeaturedContentError::TargetIdTooLong { actual: 60 }.to_string(),
            "target_id exceeds 50 chars (got 60)"
        );
    }

    #[test]
    fn negative_weight_message() {
        assert_eq!(
            FeaturedContentError::NegativeWeight { actual: -1 }.to_string(),
            "weight must be >= 0 (got -1)"
        );
    }

    #[test]
    fn invalid_time_bound_message() {
        assert_eq!(
            FeaturedContentError::InvalidTimeBound.to_string(),
            "ends_at must be strictly after starts_at"
        );
    }

    #[test]
    fn equality_holds_per_variant() {
        assert_eq!(
            FeaturedContentError::EmptyTargetId,
            FeaturedContentError::EmptyTargetId
        );
        assert_eq!(
            FeaturedContentError::TargetIdTooLong { actual: 51 },
            FeaturedContentError::TargetIdTooLong { actual: 51 }
        );
        assert_ne!(
            FeaturedContentError::TargetIdTooLong { actual: 51 },
            FeaturedContentError::TargetIdTooLong { actual: 52 }
        );
    }
}
