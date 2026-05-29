use chrono::{DateTime, Utc};
use operations_meta_domain::alert::{SystemAlert, SystemAlertSeverity};
use operations_meta_domain::featured::{
    FeaturedContent, FeaturedContentFeatureKind, FeaturedContentTargetKind,
};
use operations_meta_domain::repository::RepoError;
use shared_kernel::id::{FeaturedContentMarker, Id, SystemAlertMarker, UserMarker};
use sqlx::postgres::PgRow;
use sqlx::Row;

/// `select` 절에서 모든 `featured_content` 컬럼을 일관되게 가져오기 위한 상수.
pub(super) const FC_COLUMNS: &str = "id, target_kind, target_id, feature_kind, weight, \
    starts_at, ends_at, purchased_by, impression_count, click_count, created_at";

/// `select` 절에서 모든 `system_alert` 컬럼을 일관되게 가져오기 위한 상수.
pub(super) const SA_COLUMNS: &str = "id, severity, source, title, detail, metadata, \
    acknowledged_at, acknowledged_by, resolved_at, created_at";

fn parse_target_kind(s: &str) -> Result<FeaturedContentTargetKind, RepoError> {
    FeaturedContentTargetKind::from_db_str(s)
        .ok_or_else(|| RepoError::Database(format!("unexpected target_kind: {s}")))
}

fn parse_feature_kind(s: &str) -> Result<FeaturedContentFeatureKind, RepoError> {
    FeaturedContentFeatureKind::from_db_str(s)
        .ok_or_else(|| RepoError::Database(format!("unexpected feature_kind: {s}")))
}

fn parse_severity(s: &str) -> Result<SystemAlertSeverity, RepoError> {
    SystemAlertSeverity::from_db_str(s)
        .ok_or_else(|| RepoError::Database(format!("unexpected severity: {s}")))
}

/// `PgRow` → [`FeaturedContent`] 변환. 11 컬럼 round-trip (`version` 없음).
pub(super) fn row_to_featured(row: &PgRow) -> Result<FeaturedContent, RepoError> {
    let id_str: String = row
        .try_get("id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let target_kind_str: String = row
        .try_get("target_kind")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let target_id: String = row
        .try_get("target_id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let feature_kind_str: String = row
        .try_get("feature_kind")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let weight: i32 = row
        .try_get("weight")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let starts_at: DateTime<Utc> = row
        .try_get("starts_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let ends_at: DateTime<Utc> = row
        .try_get("ends_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let purchased_by_str: Option<String> = row
        .try_get("purchased_by")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let impression_count: i64 = row
        .try_get("impression_count")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let click_count: i64 = row
        .try_get("click_count")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let created_at: DateTime<Utc> = row
        .try_get("created_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;

    let id = Id::<FeaturedContentMarker>::try_from_str(id_str.trim())
        .map_err(|e| RepoError::Database(format!("malformed featured_content id: {e}")))?;
    let target_kind = parse_target_kind(&target_kind_str)?;
    let feature_kind = parse_feature_kind(&feature_kind_str)?;
    let purchased_by = purchased_by_str
        .map(|s| {
            Id::<UserMarker>::try_from_str(s.trim())
                .map_err(|e| RepoError::Database(format!("malformed purchased_by: {e}")))
        })
        .transpose()?;

    Ok(FeaturedContent {
        id,
        target_kind,
        target_id,
        feature_kind,
        weight,
        starts_at,
        ends_at,
        purchased_by,
        impression_count,
        click_count,
        created_at,
    })
}

/// `PgRow` → [`SystemAlert`] 변환. 10 컬럼 round-trip (`version` 없음).
pub(super) fn row_to_alert(row: &PgRow) -> Result<SystemAlert, RepoError> {
    let id_str: String = row
        .try_get("id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let severity_str: String = row
        .try_get("severity")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let source: String = row
        .try_get("source")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let title: String = row
        .try_get("title")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let detail: Option<String> = row
        .try_get("detail")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let metadata: serde_json::Value = row
        .try_get("metadata")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let acknowledged_at: Option<DateTime<Utc>> = row
        .try_get("acknowledged_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let acknowledged_by_str: Option<String> = row
        .try_get("acknowledged_by")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let resolved_at: Option<DateTime<Utc>> = row
        .try_get("resolved_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let created_at: DateTime<Utc> = row
        .try_get("created_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;

    let id = Id::<SystemAlertMarker>::try_from_str(id_str.trim())
        .map_err(|e| RepoError::Database(format!("malformed system_alert id: {e}")))?;
    let severity = parse_severity(&severity_str)?;
    let acknowledged_by = acknowledged_by_str
        .map(|s| {
            Id::<UserMarker>::try_from_str(s.trim())
                .map_err(|e| RepoError::Database(format!("malformed acknowledged_by: {e}")))
        })
        .transpose()?;

    Ok(SystemAlert {
        id,
        severity,
        source,
        title,
        detail,
        metadata,
        acknowledged_at,
        acknowledged_by,
        resolved_at,
        created_at,
    })
}
