//! `sqlx::Error` → 도메인 `RepoError` 공통 매핑.
//!
//! 모든 `Pg*Repository` 가 사용하는 단일 helper. 각 도메인 crate 의 `RepoError`
//! 가 [`MapFromSqlx`] 를 구현하면 [`map_sqlx_err`] 로 변환할 수 있어요.

use sqlx::Error as SqlxError;

/// 도메인 `RepoError` 가 `sqlx::Error` 로부터 생성될 수 있음을 표시하는 trait.
///
/// 본 trait 는 본 crate (`db`) 안에서 정의되어 외부 타입에 impl 가능해요 (orphan
/// rule 우회).
pub trait MapFromSqlx: Sized {
    /// `Unique` 제약 위반 — `Conflict` 의미.
    ///
    /// `Conflict` variant 가 없는 도메인은 `Database` 로 fallback 매핑할 수 있어요
    /// (현재는 모든 도메인이 `Conflict` 를 가져요).
    fn conflict() -> Self;
    /// 일반 `DB` 에러 — 메시지만 보존 (정보 누설 방지).
    fn database(msg: String) -> Self;
}

/// `sqlx::Error` 를 도메인 `RepoError` 로 매핑.
///
/// - Unique violation → [`MapFromSqlx::conflict`]
/// - 그 외 → [`MapFromSqlx::database`] (메시지는 `e.to_string()`)
///
/// `RowNotFound` 은 `fetch_optional` 사용 시 `Ok(None)` 으로 반환되므로 본 함수에 도달
/// 하지 않아요.
///
/// `.map_err(map_sqlx_err)` 호출 ergonomic 을 위해 owned `sqlx::Error` 를 받아요.
#[must_use]
#[allow(clippy::needless_pass_by_value)]
pub fn map_sqlx_err<E: MapFromSqlx>(e: SqlxError) -> E {
    if let SqlxError::Database(ref db_err) = e {
        if db_err.is_unique_violation() {
            return E::conflict();
        }
    }
    E::database(e.to_string())
}

// User domain RepoError
impl MapFromSqlx for user_domain::repository::RepoError {
    fn conflict() -> Self {
        Self::Conflict
    }
    fn database(msg: String) -> Self {
        Self::Database(msg)
    }
}

// Listing domain RepoError
impl MapFromSqlx for listing_domain::repository::RepoError {
    fn conflict() -> Self {
        Self::Conflict
    }
    fn database(msg: String) -> Self {
        Self::Database(msg)
    }
}

// ListingPhoto domain RepoError
impl MapFromSqlx for listing_photo_domain::repository::RepoError {
    fn conflict() -> Self {
        Self::Conflict
    }
    fn database(msg: String) -> Self {
        Self::Database(msg)
    }
}

// `AuditLog` domain `RepoError` — no `Conflict` variant
impl MapFromSqlx for audit_log_domain::repository::RepoError {
    fn conflict() -> Self {
        // `audit_log` 는 immutable, `OCC` 없음. unique violation 도 `ULID` 자동
        // 생성으로 발생 안 해요. 여기 도달했다면 비정상 — `Database` 로 fallback.
        Self::Database("unexpected conflict in audit_log".to_owned())
    }
    fn database(msg: String) -> Self {
        Self::Database(msg)
    }
}

// `OutboxEvent` domain `RepoError` — no `Conflict` variant
impl MapFromSqlx for outbox_event_domain::repository::RepoError {
    fn conflict() -> Self {
        Self::Database("unexpected conflict in outbox_event".to_owned())
    }
    fn database(msg: String) -> Self {
        Self::Database(msg)
    }
}

// Pipeline domain `RepoError` — has `Conflict` variant
impl MapFromSqlx for data_pipeline_control::repository::RepoError {
    fn conflict() -> Self {
        Self::Conflict
    }
    fn database(msg: String) -> Self {
        Self::Database(msg)
    }
}

// `AdminAction` domain `RepoError` — no `Conflict` variant
impl MapFromSqlx for admin_action_domain::repository::RepoError {
    fn conflict() -> Self {
        Self::Database("unexpected conflict in admin_action".to_owned())
    }
    fn database(msg: String) -> Self {
        Self::Database(msg)
    }
}

// `BVQ` domain `RepoError` — has `Conflict` variant
impl MapFromSqlx for business_verification_queue_domain::repository::RepoError {
    fn conflict() -> Self {
        Self::Conflict
    }
    fn database(msg: String) -> Self {
        Self::Database(msg)
    }
}

// `LRQ` domain `RepoError` — has `Conflict` variant
impl MapFromSqlx for lrq_domain::repository::RepoError {
    fn conflict() -> Self {
        Self::Conflict
    }
    fn database(msg: String) -> Self {
        Self::Database(msg)
    }
}

// `ListingReport` domain `RepoError` — no `Conflict` variant
impl MapFromSqlx for listing_report_domain::repository::RepoError {
    fn conflict() -> Self {
        Self::Database("unexpected conflict in listing_report".to_owned())
    }
    fn database(msg: String) -> Self {
        Self::Database(msg)
    }
}

// `OperationsMeta` domain `RepoError` — no `Conflict` variant
impl MapFromSqlx for operations_meta_domain::repository::RepoError {
    fn conflict() -> Self {
        Self::Database("unexpected conflict in operations_meta".to_owned())
    }
    fn database(msg: String) -> Self {
        Self::Database(msg)
    }
}

// `Bookmark` domain `RepoError` — no `Conflict` variant (UPSERT 패턴)
impl MapFromSqlx for bookmark_domain::repository::RepoError {
    fn conflict() -> Self {
        // unique violation 은 UPSERT 로 처리되므로 정상 흐름엔 도달 안 함.
        Self::Database("unexpected conflict in bookmark".to_owned())
    }
    fn database(msg: String) -> Self {
        Self::Database(msg)
    }
}

// `SearchHistory` domain `RepoError` — no `Conflict` variant (insert-only)
impl MapFromSqlx for search_history_domain::repository::RepoError {
    fn conflict() -> Self {
        Self::Database("unexpected conflict in search_history".to_owned())
    }
    fn database(msg: String) -> Self {
        Self::Database(msg)
    }
}

// `AnalysisReport` domain `RepoError` — has `Conflict` variant (OCC)
impl MapFromSqlx for analysis_report_domain::repository::RepoError {
    fn conflict() -> Self {
        Self::Conflict
    }
    fn database(msg: String) -> Self {
        Self::Database(msg)
    }
}

// `Notification` domain `RepoError` — no `Conflict` variant (mark_read 멱등)
impl MapFromSqlx for notification_domain::repository::RepoError {
    fn conflict() -> Self {
        Self::Database("unexpected conflict in notification".to_owned())
    }
    fn database(msg: String) -> Self {
        Self::Database(msg)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

    use super::*;

    /// `sqlx::Error::Io` 변종으로 `database()` 분기 검증 (unique violation 분기는 통합
    /// 테스트에서 진짜 `DB` 로 검증 — 본 함수에서 `DatabaseError` mock 을 만들 수 없음).
    #[test]
    fn io_error_maps_to_database() {
        let io = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "test");
        let e = SqlxError::Io(io);
        let err: user_domain::repository::RepoError = map_sqlx_err(e);
        match err {
            user_domain::repository::RepoError::Database(s) => {
                assert!(s.contains("test") || s.contains("ConnectionRefused"));
            }
            _ => panic!("expected Database variant"),
        }
    }

    #[test]
    fn protocol_error_maps_to_database_for_listing() {
        let e = SqlxError::Protocol("bad protocol".into());
        let err: listing_domain::repository::RepoError = map_sqlx_err(e);
        assert!(matches!(
            err,
            listing_domain::repository::RepoError::Database(_)
        ));
    }
}
