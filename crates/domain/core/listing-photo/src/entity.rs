//! `ListingPhoto` Aggregate (spec § 5.1, 12 필드).

// `ListingPhoto`/`PhotoContentType` 처럼 모듈명 반복은 의도된 공개 API 형태.
#![allow(clippy::module_name_repetitions)]

use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared_kernel::id::{Id, ListingMarker, ListingPhotoMarker};

use crate::errors::ListingPhotoError;

/// 사진 `MIME` content-type (3값).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PhotoContentType {
    /// `image/jpeg`.
    Jpeg,
    /// `image/png`.
    Png,
    /// `image/webp`.
    Webp,
}

impl PhotoContentType {
    /// `MIME` 문자열 반환 (`DB varchar(50)` 매핑).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
            Self::Webp => "image/webp",
        }
    }
}

/// `PhotoContentType` 파싱 에러.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PhotoContentTypeError {
    /// 미지원 `MIME`.
    #[error("unsupported photo content_type: '{0}' (expected: image/jpeg, image/png, image/webp)")]
    Unsupported(String),
}

impl fmt::Display for PhotoContentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for PhotoContentType {
    type Err = PhotoContentTypeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "image/jpeg" => Ok(Self::Jpeg),
            "image/png" => Ok(Self::Png),
            "image/webp" => Ok(Self::Webp),
            other => Err(PhotoContentTypeError::Unsupported(other.to_owned())),
        }
    }
}

/// `ListingPhoto` Aggregate.
///
/// spec § 5.1 `listing_photo` 테이블 12 필드 1:1 매핑이에요.
/// `version` 컬럼은 *없어요* — 사진은 append/soft-delete 흐름이라
/// 동시 관리자 수정 충돌이 없어요.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListingPhoto {
    /// `lph_<26 ULID>` 형식 ID.
    pub id: Id<ListingPhotoMarker>,
    /// 소속 매물 (`FK` + `ON DELETE CASCADE`).
    pub listing_id: Id<ListingMarker>,
    /// `R2` 객체 키 (예: `'listings/lst_01HXY/photos/p1.jpg'`).
    pub r2_key: String,
    /// 썸네일 `R2` 키 (선택).
    pub thumbnail_r2_key: Option<String>,
    /// 캡션 (≤200자, 선택).
    pub caption: Option<String>,
    /// 표시 순서 (≥0).
    pub display_order: i32,
    /// 너비 px (선택, 메타데이터).
    pub width_px: Option<i32>,
    /// 높이 px (선택, 메타데이터).
    pub height_px: Option<i32>,
    /// 파일 크기 bytes (선택).
    pub file_size_bytes: Option<i64>,
    /// `MIME` content-type.
    pub content_type: PhotoContentType,
    /// 업로드 시각.
    pub uploaded_at: DateTime<Utc>,
    /// Soft-delete 시각. `None`이면 활성.
    pub deleted_at: Option<DateTime<Utc>>,
}

impl ListingPhoto {
    /// 검증 후 새 `ListingPhoto` 생성. `deleted_at = None`,
    /// `uploaded_at = now`로 설정해요.
    ///
    /// # Errors
    ///
    /// - `r2_key`가 trim 후 빈 문자열 → [`ListingPhotoError::EmptyR2Key`].
    /// - `display_order < 0` → [`ListingPhotoError::NegativeDisplayOrder`].
    /// - `caption` 200자 초과 → [`ListingPhotoError::CaptionTooLong`].
    #[allow(clippy::too_many_arguments)] // 의도된 풀 생성자 (12 필드 매핑).
    pub fn try_new(
        id: Id<ListingPhotoMarker>,
        listing_id: Id<ListingMarker>,
        r2_key: String,
        thumbnail_r2_key: Option<String>,
        caption: Option<String>,
        display_order: i32,
        width_px: Option<i32>,
        height_px: Option<i32>,
        file_size_bytes: Option<i64>,
        content_type: PhotoContentType,
        now: DateTime<Utc>,
    ) -> Result<Self, ListingPhotoError> {
        let r2_key = r2_key.trim().to_owned();
        if r2_key.is_empty() {
            return Err(ListingPhotoError::EmptyR2Key);
        }
        if display_order < 0 {
            return Err(ListingPhotoError::NegativeDisplayOrder {
                actual: display_order,
            });
        }
        if let Some(ref c) = caption {
            let len = c.chars().count();
            if len > 200 {
                return Err(ListingPhotoError::CaptionTooLong { actual: len });
            }
        }

        Ok(Self {
            id,
            listing_id,
            r2_key,
            thumbnail_r2_key,
            caption,
            display_order,
            width_px,
            height_px,
            file_size_bytes,
            content_type,
            uploaded_at: now,
            deleted_at: None,
        })
    }

    /// Soft-delete. `deleted_at`을 설정해요. 이미 삭제된 경우
    /// 무시 (idempotent — 첫 삭제 시각 보존).
    pub const fn soft_delete(&mut self, at: DateTime<Utc>) {
        if self.deleted_at.is_none() {
            self.deleted_at = Some(at);
        }
    }

    /// `display_order` 변경. 음수 거부.
    ///
    /// # Errors
    ///
    /// `new_order < 0` → [`ListingPhotoError::NegativeDisplayOrder`].
    pub const fn reorder(&mut self, new_order: i32) -> Result<(), ListingPhotoError> {
        if new_order < 0 {
            return Err(ListingPhotoError::NegativeDisplayOrder { actual: new_order });
        }
        self.display_order = new_order;
        Ok(())
    }

    /// 활성 여부 (soft-delete 안 됨).
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.deleted_at.is_none()
    }
}

#[cfg(test)]
#[path = "entity_tests.rs"]
mod tests;
