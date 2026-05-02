//! `SearchHistory` Aggregate (append-mostly + `PIPA` pseudonymize).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared_kernel::id::{Id, SearchHistoryMarker, UserMarker};

use crate::errors::SearchHistoryError;

/// 사용자 검색 이력 1건. append-mostly (매 검색마다 INSERT).
///
/// `user_id`는 비로그인 검색 또는 `PIPA` 가명화 후 `None`이에요.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchHistory {
    /// 식별자 (`srh_<26 ULID>`).
    pub id: Id<SearchHistoryMarker>,
    /// 사용자 ID. `None` = 비로그인 또는 가명화 후.
    pub user_id: Option<Id<UserMarker>>,
    /// 검색어 (≤500자, 비어있지 않음).
    pub query: String,
    /// 필터 (`jsonb`).
    pub filters: serde_json::Value,
    /// 결과 row 수.
    pub result_count: u32,
    /// 추적 ID (≤30자, 비어있지 않음).
    pub correlation_id: String,
    /// 생성 시각.
    pub created_at: DateTime<Utc>,
}

impl SearchHistory {
    /// 검증 후 생성.
    ///
    /// # Errors
    ///
    /// - `query` 빈 → [`SearchHistoryError::EmptyQuery`].
    /// - `query` 500자 초과 → [`SearchHistoryError::QueryTooLong`].
    /// - `correlation_id` 빈 → [`SearchHistoryError::EmptyCorrelationId`].
    /// - `correlation_id` 30자 초과 → [`SearchHistoryError::CorrelationIdTooLong`].
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        id: Id<SearchHistoryMarker>,
        user_id: Option<Id<UserMarker>>,
        query: &str,
        filters: serde_json::Value,
        result_count: u32,
        correlation_id: &str,
        now: DateTime<Utc>,
    ) -> Result<Self, SearchHistoryError> {
        let query = query.trim().to_owned();
        if query.is_empty() {
            return Err(SearchHistoryError::EmptyQuery);
        }
        if query.chars().count() > 500 {
            return Err(SearchHistoryError::QueryTooLong {
                actual: query.chars().count(),
            });
        }
        let correlation_id = correlation_id.trim().to_owned();
        if correlation_id.is_empty() {
            return Err(SearchHistoryError::EmptyCorrelationId);
        }
        if correlation_id.chars().count() > 30 {
            return Err(SearchHistoryError::CorrelationIdTooLong {
                actual: correlation_id.chars().count(),
            });
        }
        Ok(Self {
            id,
            user_id,
            query,
            filters,
            result_count,
            correlation_id,
            created_at: now,
        })
    }

    /// `PIPA` 가명화 — `user_id`를 `None`으로 설정해요.
    ///
    /// 90일 retention 워커가 호출. 사용자 mutation이 아닌 운영 op라 version bump 없어요.
    pub fn pseudonymize(&mut self) {
        self.user_id = None;
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;

    fn sample_filters() -> serde_json::Value {
        serde_json::json!({"region": "성남시", "min_price": 100_000_000})
    }

    #[test]
    fn happy_path_logged_in() {
        let sh = SearchHistory::try_new(
            Id::new(),
            Some(Id::new()),
            "성남 지식산업센터",
            sample_filters(),
            42,
            "req_01HXY3NK0Z9F6S1B2C",
            Utc::now(),
        )
        .expect("valid");
        assert!(sh.user_id.is_some());
        assert_eq!(sh.result_count, 42);
        assert_eq!(sh.query, "성남 지식산업센터");
    }

    #[test]
    fn happy_path_anonymous() {
        let sh = SearchHistory::try_new(
            Id::new(),
            None,
            "공장 임대",
            serde_json::json!({}),
            0,
            "req_anon_001",
            Utc::now(),
        )
        .expect("valid");
        assert!(sh.user_id.is_none());
    }

    #[test]
    fn rejects_empty_query() {
        let err = SearchHistory::try_new(
            Id::new(),
            Some(Id::new()),
            "",
            sample_filters(),
            0,
            "req_001",
            Utc::now(),
        )
        .unwrap_err();
        assert!(matches!(err, SearchHistoryError::EmptyQuery));
    }

    #[test]
    fn rejects_whitespace_only_query() {
        let err = SearchHistory::try_new(
            Id::new(),
            Some(Id::new()),
            "    ",
            sample_filters(),
            0,
            "req_001",
            Utc::now(),
        )
        .unwrap_err();
        assert!(matches!(err, SearchHistoryError::EmptyQuery));
    }

    #[test]
    fn rejects_query_over_500_chars() {
        let long = "X".repeat(501);
        let err = SearchHistory::try_new(
            Id::new(),
            Some(Id::new()),
            &long,
            sample_filters(),
            0,
            "req_001",
            Utc::now(),
        )
        .unwrap_err();
        assert!(matches!(
            err,
            SearchHistoryError::QueryTooLong { actual: 501 }
        ));
    }

    #[test]
    fn accepts_query_exactly_500_chars() {
        let exactly = "X".repeat(500);
        let sh = SearchHistory::try_new(
            Id::new(),
            None,
            &exactly,
            sample_filters(),
            0,
            "req_001",
            Utc::now(),
        )
        .expect("500 ok");
        assert_eq!(sh.query.chars().count(), 500);
    }

    #[test]
    fn rejects_empty_correlation_id() {
        let err =
            SearchHistory::try_new(Id::new(), None, "공장", sample_filters(), 0, "", Utc::now())
                .unwrap_err();
        assert!(matches!(err, SearchHistoryError::EmptyCorrelationId));
    }

    #[test]
    fn rejects_correlation_id_over_30_chars() {
        let long = "X".repeat(31);
        let err = SearchHistory::try_new(
            Id::new(),
            None,
            "공장",
            sample_filters(),
            0,
            &long,
            Utc::now(),
        )
        .unwrap_err();
        assert!(matches!(
            err,
            SearchHistoryError::CorrelationIdTooLong { actual: 31 }
        ));
    }

    #[test]
    fn pseudonymize_clears_user_id() {
        let mut sh = SearchHistory::try_new(
            Id::new(),
            Some(Id::new()),
            "성남 지식산업센터",
            sample_filters(),
            42,
            "req_001",
            Utc::now(),
        )
        .expect("valid");
        assert!(sh.user_id.is_some());
        sh.pseudonymize();
        assert!(sh.user_id.is_none());
    }

    #[test]
    fn pseudonymize_preserves_other_fields() {
        let original = SearchHistory::try_new(
            Id::new(),
            Some(Id::new()),
            "공장 임대",
            sample_filters(),
            7,
            "req_42",
            Utc::now(),
        )
        .expect("valid");
        let mut copy = original.clone();
        copy.pseudonymize();
        assert_eq!(copy.id, original.id);
        assert_eq!(copy.query, original.query);
        assert_eq!(copy.filters, original.filters);
        assert_eq!(copy.result_count, original.result_count);
        assert_eq!(copy.correlation_id, original.correlation_id);
        assert_eq!(copy.created_at, original.created_at);
    }

    #[test]
    fn pseudonymize_idempotent_on_anonymous() {
        let mut sh = SearchHistory::try_new(
            Id::new(),
            None,
            "공장",
            sample_filters(),
            0,
            "req_001",
            Utc::now(),
        )
        .expect("valid");
        sh.pseudonymize();
        assert!(sh.user_id.is_none());
        sh.pseudonymize();
        assert!(sh.user_id.is_none());
    }

    #[test]
    fn serde_roundtrip() {
        let sh = SearchHistory::try_new(
            Id::new(),
            Some(Id::new()),
            "성남 지식산업센터",
            sample_filters(),
            42,
            "req_01HXY3NK0Z9F6S1",
            Utc::now(),
        )
        .expect("valid");
        let json = serde_json::to_string(&sh).expect("serialize");
        let back: SearchHistory = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(sh, back);
    }
}
