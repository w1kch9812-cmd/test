//! `AnalysisReport` Aggregate (optimistic locking + `R2` 시점 캐시 `snapshot`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared_kernel::id::{AnalysisReportMarker, Id, UserMarker};
use shared_kernel::pnu::Pnu;

use crate::errors::AnalysisReportError;

/// `target_pnus` 최대 개수 (응답 크기 제한).
const MAX_TARGET_PNUS: usize = 50;

/// `title` 최대 길이.
const MAX_TITLE_LEN: usize = 200;

/// 사용자가 다수 필지를 묶어 저장한 분석 리포트.
///
/// `snapshot`은 `R2` 데이터의 시점 고정 캐시(`JSONB`). 재분석 시
/// [`AnalysisReport::update_snapshot`]로 갱신하며 `version` bump.
///
/// `updated_at` 컬럼은 마이그레이션 30004 (`V003_04`) 에서 DB 에 추가되어
/// 도메인-DB 스키마가 일치해요.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisReport {
    /// 식별자 (`rpt_<26 ULID>`).
    pub id: Id<AnalysisReportMarker>,
    /// 소유자.
    pub user_id: Id<UserMarker>,
    /// 제목 (≤200자, trim 후 비어있지 않음).
    pub title: String,
    /// 분석 대상 필지 (≥1, ≤50).
    pub target_pnus: Vec<Pnu>,
    /// `R2` 데이터 시점 캐시 (`JSONB`).
    pub snapshot: serde_json::Value,
    /// 생성 시각.
    pub created_at: DateTime<Utc>,
    /// 마지막 갱신 시각 (`version` bump 시 함께 갱신).
    pub updated_at: DateTime<Utc>,
    /// Optimistic locking 버전.
    pub version: i64,
}

impl AnalysisReport {
    /// 검증 후 생성. `created_at == updated_at`, `version = 1`.
    ///
    /// # Errors
    ///
    /// - `title` 빈 (trim 후) → [`AnalysisReportError::EmptyTitle`].
    /// - `title` 200자 초과 → [`AnalysisReportError::TitleTooLong`].
    /// - `target_pnus` 빈 → [`AnalysisReportError::EmptyTargetPnus`].
    /// - `target_pnus` 50개 초과 → [`AnalysisReportError::TooManyTargetPnus`].
    #[allow(clippy::too_many_arguments)] // 의도된 풀 생성자 (clippy.toml threshold = 5)
    pub fn try_new(
        id: Id<AnalysisReportMarker>,
        user_id: Id<UserMarker>,
        title: &str,
        target_pnus: Vec<Pnu>,
        snapshot: serde_json::Value,
        now: DateTime<Utc>,
    ) -> Result<Self, AnalysisReportError> {
        let title = Self::validate_title(title)?;
        Self::validate_target_pnus(&target_pnus)?;
        Ok(Self {
            id,
            user_id,
            title,
            target_pnus,
            snapshot,
            created_at: now,
            updated_at: now,
            version: 1,
        })
    }

    /// 제목 변경 + `version` bump + `updated_at` 갱신.
    ///
    /// # Errors
    ///
    /// `try_new`과 동일한 `title` 검증.
    pub fn rename(
        &mut self,
        new_title: &str,
        at: DateTime<Utc>,
    ) -> Result<(), AnalysisReportError> {
        let new_title = Self::validate_title(new_title)?;
        self.title = new_title;
        self.version += 1;
        self.updated_at = at;
        Ok(())
    }

    /// `snapshot` 갱신 + `version` bump + `updated_at` 갱신 (재분석 시).
    pub fn update_snapshot(&mut self, new_snapshot: serde_json::Value, at: DateTime<Utc>) {
        self.snapshot = new_snapshot;
        self.version += 1;
        self.updated_at = at;
    }

    fn validate_title(title: &str) -> Result<String, AnalysisReportError> {
        let trimmed = title.trim().to_owned();
        if trimmed.is_empty() {
            return Err(AnalysisReportError::EmptyTitle);
        }
        let len = trimmed.chars().count();
        if len > MAX_TITLE_LEN {
            return Err(AnalysisReportError::TitleTooLong { actual: len });
        }
        Ok(trimmed)
    }

    const fn validate_target_pnus(pnus: &[Pnu]) -> Result<(), AnalysisReportError> {
        if pnus.is_empty() {
            return Err(AnalysisReportError::EmptyTargetPnus);
        }
        if pnus.len() > MAX_TARGET_PNUS {
            return Err(AnalysisReportError::TooManyTargetPnus { actual: pnus.len() });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;

    fn sample_pnu() -> Pnu {
        Pnu::try_new("1111010100100010000").expect("valid PNU")
    }

    fn sample_pnu_n(n: u32) -> Pnu {
        // 본번 4자리 영역에 n을 박는 19자리 PNU 생성.
        let s = format!("11110101001{:04}0000", n % 10_000);
        Pnu::try_new(&s).expect("valid PNU")
    }

    fn sample_snapshot() -> serde_json::Value {
        serde_json::json!({"jiga_avg": 1_500_000, "buildings": 3})
    }

    #[test]
    fn happy_path_single_pnu() {
        let now = Utc::now();
        let r = AnalysisReport::try_new(
            Id::new(),
            Id::new(),
            "성남 후보지 분석",
            vec![sample_pnu()],
            sample_snapshot(),
            now,
        )
        .expect("valid");
        assert_eq!(r.title, "성남 후보지 분석");
        assert_eq!(r.target_pnus.len(), 1);
        assert_eq!(r.version, 1);
        assert_eq!(r.created_at, r.updated_at);
    }

    #[test]
    fn happy_path_50_pnus() {
        let pnus: Vec<Pnu> = (0..50).map(sample_pnu_n).collect();
        let r = AnalysisReport::try_new(
            Id::new(),
            Id::new(),
            "50개 필지 비교",
            pnus,
            sample_snapshot(),
            Utc::now(),
        )
        .expect("valid");
        assert_eq!(r.target_pnus.len(), 50);
    }

    #[test]
    fn rejects_empty_title() {
        let err = AnalysisReport::try_new(
            Id::new(),
            Id::new(),
            "",
            vec![sample_pnu()],
            sample_snapshot(),
            Utc::now(),
        )
        .unwrap_err();
        assert_eq!(err, AnalysisReportError::EmptyTitle);
    }

    #[test]
    fn rejects_whitespace_only_title() {
        let err = AnalysisReport::try_new(
            Id::new(),
            Id::new(),
            "    ",
            vec![sample_pnu()],
            sample_snapshot(),
            Utc::now(),
        )
        .unwrap_err();
        assert_eq!(err, AnalysisReportError::EmptyTitle);
    }

    #[test]
    fn title_is_trimmed() {
        let r = AnalysisReport::try_new(
            Id::new(),
            Id::new(),
            "   리포트   ",
            vec![sample_pnu()],
            sample_snapshot(),
            Utc::now(),
        )
        .expect("valid");
        assert_eq!(r.title, "리포트");
    }

    #[test]
    fn rejects_title_over_200_chars() {
        let long = "X".repeat(201);
        let err = AnalysisReport::try_new(
            Id::new(),
            Id::new(),
            &long,
            vec![sample_pnu()],
            sample_snapshot(),
            Utc::now(),
        )
        .unwrap_err();
        assert_eq!(err, AnalysisReportError::TitleTooLong { actual: 201 });
    }

    #[test]
    fn accepts_title_exactly_200_chars() {
        let exactly = "X".repeat(200);
        let r = AnalysisReport::try_new(
            Id::new(),
            Id::new(),
            &exactly,
            vec![sample_pnu()],
            sample_snapshot(),
            Utc::now(),
        )
        .expect("200 ok");
        assert_eq!(r.title.chars().count(), 200);
    }

    #[test]
    fn rejects_empty_target_pnus() {
        let err = AnalysisReport::try_new(
            Id::new(),
            Id::new(),
            "리포트",
            vec![],
            sample_snapshot(),
            Utc::now(),
        )
        .unwrap_err();
        assert_eq!(err, AnalysisReportError::EmptyTargetPnus);
    }

    #[test]
    fn rejects_target_pnus_over_50() {
        let pnus: Vec<Pnu> = (0..51).map(sample_pnu_n).collect();
        let err = AnalysisReport::try_new(
            Id::new(),
            Id::new(),
            "리포트",
            pnus,
            sample_snapshot(),
            Utc::now(),
        )
        .unwrap_err();
        assert_eq!(err, AnalysisReportError::TooManyTargetPnus { actual: 51 });
    }

    #[test]
    fn rename_bumps_version_and_updates_updated_at() {
        let t0 = Utc::now();
        let mut r = AnalysisReport::try_new(
            Id::new(),
            Id::new(),
            "원래 제목",
            vec![sample_pnu()],
            sample_snapshot(),
            t0,
        )
        .expect("valid");
        let t1 = t0 + chrono::Duration::seconds(60);
        r.rename("새 제목", t1).expect("valid rename");
        assert_eq!(r.title, "새 제목");
        assert_eq!(r.version, 2);
        assert_eq!(r.updated_at, t1);
        assert_eq!(r.created_at, t0);
    }

    #[test]
    fn rename_rejects_invalid_title() {
        let mut r = AnalysisReport::try_new(
            Id::new(),
            Id::new(),
            "원래 제목",
            vec![sample_pnu()],
            sample_snapshot(),
            Utc::now(),
        )
        .expect("valid");
        let before_version = r.version;
        let before_title = r.title.clone();
        let err = r.rename("   ", Utc::now()).unwrap_err();
        assert_eq!(err, AnalysisReportError::EmptyTitle);
        // 실패 시 mutation 없음 보장.
        assert_eq!(r.version, before_version);
        assert_eq!(r.title, before_title);
    }

    #[test]
    fn rename_rejects_too_long_title() {
        let mut r = AnalysisReport::try_new(
            Id::new(),
            Id::new(),
            "원래 제목",
            vec![sample_pnu()],
            sample_snapshot(),
            Utc::now(),
        )
        .expect("valid");
        let long = "X".repeat(201);
        let err = r.rename(&long, Utc::now()).unwrap_err();
        assert_eq!(err, AnalysisReportError::TitleTooLong { actual: 201 });
    }

    #[test]
    fn update_snapshot_bumps_version_and_updates_updated_at() {
        let t0 = Utc::now();
        let mut r = AnalysisReport::try_new(
            Id::new(),
            Id::new(),
            "리포트",
            vec![sample_pnu()],
            serde_json::json!({"v": 1}),
            t0,
        )
        .expect("valid");
        let t1 = t0 + chrono::Duration::seconds(120);
        r.update_snapshot(serde_json::json!({"v": 2}), t1);
        assert_eq!(r.snapshot, serde_json::json!({"v": 2}));
        assert_eq!(r.version, 2);
        assert_eq!(r.updated_at, t1);
        assert_eq!(r.created_at, t0);
    }

    #[test]
    fn initial_values_created_eq_updated_and_version_one() {
        let now = Utc::now();
        let r = AnalysisReport::try_new(
            Id::new(),
            Id::new(),
            "리포트",
            vec![sample_pnu()],
            sample_snapshot(),
            now,
        )
        .expect("valid");
        assert_eq!(r.version, 1);
        assert_eq!(r.created_at, now);
        assert_eq!(r.updated_at, now);
    }

    #[test]
    fn serde_roundtrip() {
        let r = AnalysisReport::try_new(
            Id::new(),
            Id::new(),
            "리포트",
            vec![sample_pnu(), sample_pnu_n(2)],
            sample_snapshot(),
            Utc::now(),
        )
        .expect("valid");
        let json = serde_json::to_string(&r).expect("serialize");
        let back: AnalysisReport = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(r, back);
    }
}
