//! `audit_log.before_state` / `after_state` 캡처 헬퍼 (SP-Obs T4).
//!
//! DB-native JSON 직렬화 (`to_jsonb(t.*)`) 로 row 를 JSON 으로 추출. `PostGIS`
//! `geometry` 컬럼은 binary `EWKB` 라 raw 출력이 비실용 — `ST_AsGeoJSON` 으로
//! 변환 후 merge.
//!
//! Pattern (`PgRepository::save`):
//!
//! ```ignore
//! let before_state = read_listing_json(&mut tx, &id).await?;  // None if INSERT
//! upsert_listing(&mut tx, listing).await?;
//! let after_state = read_listing_json(&mut tx, &id).await?;   // 항상 Some
//! let after_with_meta = merge_metadata(after_state, ctx.metadata.as_ref());
//! insert_audit_log(... before_state, after_with_meta ...).await?;
//! ```
//!
//! `__metadata__` nesting — schema 변경 없이 `after_state` JSON 의 reserved
//! key. 후속 (FU 90) 가 `audit_log.metadata jsonb` 별도 컬럼.

use serde_json::Value;
use sqlx::{Postgres, Row, Transaction};

use crate::error_map::map_sqlx_err;
use analysis_report_domain::repository::RepoError as AnalysisReportRepoError;
use bookmark_domain::repository::RepoError as BookmarkRepoError;
use listing_domain::repository::RepoError as ListingRepoError;
use listing_photo_domain::repository::RepoError as ListingPhotoRepoError;
use shared_kernel::id::{
    AnalysisReportMarker, BookmarkExternalMarker, Id, ListingMarker, ListingPhotoMarker, UserMarker,
};
use user_domain::repository::RepoError as UserRepoError;

/// `User` row → JSON. `to_jsonb(t.*)` (`PostGIS` 미사용 — 단순).
pub async fn read_user_json(
    tx: &mut Transaction<'_, Postgres>,
    id: &Id<UserMarker>,
) -> Result<Option<Value>, UserRepoError> {
    let row = sqlx::query(r#"select to_jsonb(t.*) as snap from "user" t where id = $1"#)
        .bind(id.as_str())
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_sqlx_err::<UserRepoError>)?;
    Ok(row.and_then(|r| r.try_get::<Option<Value>, _>("snap").ok().flatten()))
}

/// `Listing` row → JSON. `geom_point` 는 `ST_AsGeoJSON` 으로 변환 후 merge —
/// audit reader 가 좌표 읽을 수 있도록 (raw EWKB 비실용).
pub async fn read_listing_json(
    tx: &mut Transaction<'_, Postgres>,
    id: &Id<ListingMarker>,
) -> Result<Option<Value>, ListingRepoError> {
    // to_jsonb 에서 geom_point 제거 후 GeoJSON 형태로 다시 추가.
    let row = sqlx::query(
        r"
        select (
            to_jsonb(t.*) - 'geom_point'
            || jsonb_build_object(
                'geom_point',
                case when t.geom_point is null then null
                else ST_AsGeoJSON(t.geom_point)::jsonb end
            )
        ) as snap
        from listing t
        where id = $1
        ",
    )
    .bind(id.as_str())
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_sqlx_err::<ListingRepoError>)?;
    Ok(row.and_then(|r| r.try_get::<Option<Value>, _>("snap").ok().flatten()))
}

/// `ListingPhoto` row → JSON. `PostGIS` 미사용.
pub async fn read_listing_photo_json(
    tx: &mut Transaction<'_, Postgres>,
    id: &Id<ListingPhotoMarker>,
) -> Result<Option<Value>, ListingPhotoRepoError> {
    let row = sqlx::query("select to_jsonb(t.*) as snap from listing_photo t where id = $1")
        .bind(id.as_str())
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_sqlx_err::<ListingPhotoRepoError>)?;
    Ok(row.and_then(|r| r.try_get::<Option<Value>, _>("snap").ok().flatten()))
}

/// `BookmarkListing` row → JSON (composite PK `(user_id, listing_id)`).
pub async fn read_bookmark_listing_json(
    tx: &mut Transaction<'_, Postgres>,
    user_id: &Id<UserMarker>,
    listing_id: &Id<ListingMarker>,
) -> Result<Option<Value>, BookmarkRepoError> {
    let row = sqlx::query(
        "select to_jsonb(t.*) as snap from bookmark_listing t \
         where user_id = $1 and listing_id = $2",
    )
    .bind(user_id.as_str())
    .bind(listing_id.as_str())
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_sqlx_err::<BookmarkRepoError>)?;
    Ok(row.and_then(|r| r.try_get::<Option<Value>, _>("snap").ok().flatten()))
}

/// `BookmarkExternal` row → JSON (단일 PK).
pub async fn read_bookmark_external_json(
    tx: &mut Transaction<'_, Postgres>,
    id: &Id<BookmarkExternalMarker>,
) -> Result<Option<Value>, BookmarkRepoError> {
    let row = sqlx::query("select to_jsonb(t.*) as snap from bookmark_external t where id = $1")
        .bind(id.as_str())
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_sqlx_err::<BookmarkRepoError>)?;
    Ok(row.and_then(|r| r.try_get::<Option<Value>, _>("snap").ok().flatten()))
}

/// `AnalysisReport` row → JSON. `target_pnus char(19)[]` 은 `to_jsonb` 가 array
/// 로 안전하게 처리.
pub async fn read_analysis_report_json(
    tx: &mut Transaction<'_, Postgres>,
    id: &Id<AnalysisReportMarker>,
) -> Result<Option<Value>, AnalysisReportRepoError> {
    let row = sqlx::query("select to_jsonb(t.*) as snap from analysis_report t where id = $1")
        .bind(id.as_str())
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_sqlx_err::<AnalysisReportRepoError>)?;
    Ok(row.and_then(|r| r.try_get::<Option<Value>, _>("snap").ok().flatten()))
}

/// `ctx.metadata` 를 `after_state` JSON 의 `__metadata__` reserved key 로 merge.
///
/// - `after_state` 가 `Some(Object)` + `metadata` 가 `Some` → 객체 merge
/// - `after_state` 가 `Some(non-Object)` (희박) + metadata Some → wrapper 객체
/// - `metadata` 가 `None` → `after_state` 그대로
/// - `after_state` 가 `None` → metadata 만 wrapping (DELETE 시나리오)
///
/// FU 90 (별도 `metadata jsonb` 컬럼 마이그) 가 본 nesting 을 대체. 1차 = schema
/// 변경 0.
#[must_use]
pub fn merge_metadata(after_state: Option<Value>, metadata: Option<&Value>) -> Option<Value> {
    match (after_state, metadata) {
        (Some(Value::Object(mut obj)), Some(meta)) => {
            obj.insert("__metadata__".to_owned(), meta.clone());
            Some(Value::Object(obj))
        }
        (Some(s), Some(meta)) => Some(serde_json::json!({"__state__": s, "__metadata__": meta})),
        (s, None) => s,
        (None, Some(meta)) => Some(serde_json::json!({"__metadata__": meta})),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;
    use serde_json::json;

    #[test]
    fn merge_metadata_into_object_state() {
        let state = json!({"id": "lst_x", "title": "test"});
        let meta = json!({"reason": "edit"});
        let merged = merge_metadata(Some(state), Some(&meta)).expect("some");
        assert_eq!(merged["id"], "lst_x");
        assert_eq!(merged["title"], "test");
        assert_eq!(merged["__metadata__"]["reason"], "edit");
    }

    #[test]
    fn merge_metadata_when_state_is_none() {
        let meta = json!({"action": "delete"});
        let merged = merge_metadata(None, Some(&meta)).expect("some");
        assert_eq!(merged["__metadata__"]["action"], "delete");
    }

    #[test]
    fn merge_metadata_returns_state_when_metadata_none() {
        let state = json!({"id": "x"});
        let merged = merge_metadata(Some(state.clone()), None).expect("some");
        assert_eq!(merged, state);
    }

    #[test]
    fn merge_metadata_returns_none_when_both_none() {
        assert!(merge_metadata(None, None).is_none());
    }

    #[test]
    fn merge_metadata_wraps_non_object_state() {
        // 비현실적이지만 — array / scalar after_state 도 안전하게 wrap.
        let state = json!([1, 2, 3]);
        let meta = json!({"x": 1});
        let merged = merge_metadata(Some(state), Some(&meta)).expect("some");
        assert_eq!(merged["__state__"], json!([1, 2, 3]));
        assert_eq!(merged["__metadata__"]["x"], 1);
    }
}
