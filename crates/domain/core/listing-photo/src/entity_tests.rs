//! `ListingPhoto` Aggregate 단위 테스트 — `entity.rs`/`errors.rs` 동작 검증.
//!
//! `entity.rs`에서 `#[path = "entity_tests.rs"] mod tests;` 형태로 포함해요.
//! 파일 자체가 테스트 모듈이므로 별도 `mod tests {}` 래퍼 없어요.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::str::FromStr;

use chrono::{DateTime, TimeZone, Utc};
use shared_kernel::id::{Id, ListingMarker, ListingPhotoMarker};

use super::{ListingPhoto, PhotoContentType, PhotoContentTypeError};
use crate::errors::ListingPhotoError;

// ── Fixtures ───────────────────────────────────────────────────────────────

fn sample_now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 5, 2, 12, 0, 0).unwrap()
}

fn later_now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 5, 3, 9, 30, 0).unwrap()
}

fn sample_id() -> Id<ListingPhotoMarker> {
    Id::<ListingPhotoMarker>::new()
}

fn sample_listing_id() -> Id<ListingMarker> {
    Id::<ListingMarker>::new()
}

fn sample_r2_key() -> String {
    "listings/lst_01HXY3NK0Z9F6S1B2C3D4E5F6G/photos/p1.jpg".to_owned()
}

/// 모든 `Some` 필드 + `Jpeg`로 happy path 빌드.
fn build_full() -> ListingPhoto {
    ListingPhoto::try_new(
        sample_id(),
        sample_listing_id(),
        sample_r2_key(),
        Some("listings/lst_01HXY3NK0Z9F6S1B2C3D4E5F6G/photos/p1_thumb.jpg".to_owned()),
        Some("정문에서 본 외관".to_owned()),
        0,
        Some(1920),
        Some(1080),
        Some(524_288),
        PhotoContentType::Jpeg,
        sample_now(),
    )
    .expect("full happy path valid")
}

// ── try_new happy paths ────────────────────────────────────────────────────

#[test]
fn try_new_full_fields_succeeds() {
    let photo = build_full();
    assert_eq!(photo.r2_key, sample_r2_key());
    assert_eq!(photo.thumbnail_r2_key.as_deref(), Some("listings/lst_01HXY3NK0Z9F6S1B2C3D4E5F6G/photos/p1_thumb.jpg"));
    assert_eq!(photo.caption.as_deref(), Some("정문에서 본 외관"));
    assert_eq!(photo.display_order, 0);
    assert_eq!(photo.width_px, Some(1920));
    assert_eq!(photo.height_px, Some(1080));
    assert_eq!(photo.file_size_bytes, Some(524_288));
    assert_eq!(photo.content_type, PhotoContentType::Jpeg);
    assert_eq!(photo.uploaded_at, sample_now());
    assert!(photo.deleted_at.is_none());
}

#[test]
fn try_new_all_optionals_none_succeeds() {
    let photo = ListingPhoto::try_new(
        sample_id(),
        sample_listing_id(),
        sample_r2_key(),
        None,
        None,
        5,
        None,
        None,
        None,
        PhotoContentType::Png,
        sample_now(),
    )
    .expect("optionals None is valid");
    assert!(photo.thumbnail_r2_key.is_none());
    assert!(photo.caption.is_none());
    assert!(photo.width_px.is_none());
    assert!(photo.height_px.is_none());
    assert!(photo.file_size_bytes.is_none());
    assert_eq!(photo.display_order, 5);
}

#[test]
fn try_new_all_three_content_types_succeed() {
    for ct in [
        PhotoContentType::Jpeg,
        PhotoContentType::Png,
        PhotoContentType::Webp,
    ] {
        let photo = ListingPhoto::try_new(
            sample_id(),
            sample_listing_id(),
            sample_r2_key(),
            None,
            None,
            0,
            None,
            None,
            None,
            ct,
            sample_now(),
        )
        .expect("each content type valid");
        assert_eq!(photo.content_type, ct);
    }
}

#[test]
fn try_new_zero_display_order_accepted() {
    let photo = ListingPhoto::try_new(
        sample_id(),
        sample_listing_id(),
        sample_r2_key(),
        None,
        None,
        0,
        None,
        None,
        None,
        PhotoContentType::Jpeg,
        sample_now(),
    )
    .expect("0 is non-negative");
    assert_eq!(photo.display_order, 0);
}

#[test]
fn try_new_caption_exactly_200_chars_accepted() {
    let caption: String = "가".repeat(200);
    let photo = ListingPhoto::try_new(
        sample_id(),
        sample_listing_id(),
        sample_r2_key(),
        None,
        Some(caption.clone()),
        0,
        None,
        None,
        None,
        PhotoContentType::Webp,
        sample_now(),
    )
    .expect("200 chars is the boundary");
    assert_eq!(photo.caption.as_deref(), Some(caption.as_str()));
}

// ── try_new errors ─────────────────────────────────────────────────────────

#[test]
fn try_new_empty_r2_key_rejected() {
    let err = ListingPhoto::try_new(
        sample_id(),
        sample_listing_id(),
        String::new(),
        None,
        None,
        0,
        None,
        None,
        None,
        PhotoContentType::Jpeg,
        sample_now(),
    )
    .unwrap_err();
    assert_eq!(err, ListingPhotoError::EmptyR2Key);
}

#[test]
fn try_new_whitespace_r2_key_rejected_via_trim() {
    let err = ListingPhoto::try_new(
        sample_id(),
        sample_listing_id(),
        "   \t\n  ".to_owned(),
        None,
        None,
        0,
        None,
        None,
        None,
        PhotoContentType::Jpeg,
        sample_now(),
    )
    .unwrap_err();
    assert_eq!(err, ListingPhotoError::EmptyR2Key);
}

#[test]
fn try_new_negative_display_order_rejected() {
    let err = ListingPhoto::try_new(
        sample_id(),
        sample_listing_id(),
        sample_r2_key(),
        None,
        None,
        -1,
        None,
        None,
        None,
        PhotoContentType::Jpeg,
        sample_now(),
    )
    .unwrap_err();
    assert_eq!(err, ListingPhotoError::NegativeDisplayOrder { actual: -1 });
}

#[test]
fn try_new_caption_201_chars_rejected() {
    let caption: String = "가".repeat(201);
    let err = ListingPhoto::try_new(
        sample_id(),
        sample_listing_id(),
        sample_r2_key(),
        None,
        Some(caption),
        0,
        None,
        None,
        None,
        PhotoContentType::Jpeg,
        sample_now(),
    )
    .unwrap_err();
    assert_eq!(err, ListingPhotoError::CaptionTooLong { actual: 201 });
}

// ── soft_delete + is_active ────────────────────────────────────────────────

#[test]
fn soft_delete_sets_deleted_at_and_marks_inactive() {
    let mut photo = build_full();
    assert!(photo.is_active());
    photo.soft_delete(later_now());
    assert_eq!(photo.deleted_at, Some(later_now()));
    assert!(!photo.is_active());
}

#[test]
fn soft_delete_idempotent_preserves_first_timestamp() {
    let mut photo = build_full();
    let first = later_now();
    let second = Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).unwrap();
    photo.soft_delete(first);
    photo.soft_delete(second);
    assert_eq!(photo.deleted_at, Some(first));
}

#[test]
fn soft_delete_preserves_uploaded_at() {
    let mut photo = build_full();
    let original_uploaded = photo.uploaded_at;
    photo.soft_delete(later_now());
    assert_eq!(photo.uploaded_at, original_uploaded);
}

// ── reorder ────────────────────────────────────────────────────────────────

#[test]
fn reorder_happy_path_updates_display_order() {
    let mut photo = build_full();
    photo.reorder(7).expect("non-negative");
    assert_eq!(photo.display_order, 7);
}

#[test]
fn reorder_negative_rejected() {
    let mut photo = build_full();
    let err = photo.reorder(-3).unwrap_err();
    assert_eq!(err, ListingPhotoError::NegativeDisplayOrder { actual: -3 });
    // 원본 값 유지.
    assert_eq!(photo.display_order, 0);
}

// ── PhotoContentType ───────────────────────────────────────────────────────

#[test]
fn content_type_as_str_matches_db_strings() {
    assert_eq!(PhotoContentType::Jpeg.as_str(), "image/jpeg");
    assert_eq!(PhotoContentType::Png.as_str(), "image/png");
    assert_eq!(PhotoContentType::Webp.as_str(), "image/webp");
}

#[test]
fn content_type_display_renders_mime() {
    assert_eq!(format!("{}", PhotoContentType::Jpeg), "image/jpeg");
    assert_eq!(format!("{}", PhotoContentType::Png), "image/png");
    assert_eq!(format!("{}", PhotoContentType::Webp), "image/webp");
}

#[test]
fn content_type_from_str_roundtrip() {
    for ct in [
        PhotoContentType::Jpeg,
        PhotoContentType::Png,
        PhotoContentType::Webp,
    ] {
        let parsed = PhotoContentType::from_str(ct.as_str()).expect("valid mime");
        assert_eq!(parsed, ct);
    }
}

#[test]
fn content_type_from_str_rejects_unsupported() {
    let err = PhotoContentType::from_str("image/gif").unwrap_err();
    assert_eq!(err, PhotoContentTypeError::Unsupported("image/gif".to_owned()));
}

#[test]
fn content_type_serde_roundtrip() {
    let ct = PhotoContentType::Webp;
    let json = serde_json::to_string(&ct).expect("serialize");
    assert_eq!(json, "\"Webp\"");
    let back: PhotoContentType = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back, ct);
}

// ── ListingPhoto serde ─────────────────────────────────────────────────────

#[test]
fn listing_photo_serde_json_roundtrip() {
    let photo = build_full();
    let json = serde_json::to_string(&photo).expect("serialize");
    let back: ListingPhoto = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back, photo);
}
