//! PMTiles v3 spec — magic + version 검증 + header skeleton.
//!
//! 본 모듈 1차 = magic byte + version 검증만. 완전한 directory + tile_at
//! 디코드는 [FU 60] (ETL 빌더 + production fixture) 와 함께 구현. 1차 reader
//! (R2ParcelReader / R2BuildingReader) 가 honest failure
//! `Err(Fetch("PMTiles directory decode pending FU 60"))` 반환 — V-World 합성
//! fallback 으로 운영.
//!
//! v3 spec 참고:
//! - magic: `PMTiles` ASCII 7 byte
//! - version: 1 byte (`3`)
//! - 그 이후 117 byte = root_offset, root_length, leaf_dirs_offset,
//!   leaf_dirs_length, tile_data_offset, tile_data_length, ... (varint /
//!   little-endian u64)
//!
//! [FU 60]: docs/superpowers/specs/2026-05-06-sub-project-4-iii-e-r2-pmtiles-design.md § 7

#![allow(clippy::module_name_repetitions, clippy::doc_markdown)]

use bytes::Bytes;

use crate::error::ParseError;

/// PMTiles v3 magic prefix — `PMTiles` ASCII.
const PMTILES_MAGIC: [u8; 7] = *b"PMTiles";

/// 본 1차 spec 에서 지원하는 유일한 PMTiles version.
const SUPPORTED_VERSION: u8 = 3;

/// 검증된 PMTiles 헤더 (magic + version + 미파싱 raw bytes).
///
/// 1차는 magic + version 통과만 검증 — 본 PMTiles 가 reader 가 다룰 수 있는
/// 형식인지 확인용. directory + tile data 디코드는 FU 60 과 함께.
#[derive(Debug, Clone)]
pub struct PmtilesHeader {
    /// Spec version (`3` 만 지원).
    pub version: u8,
    /// 전체 파일 raw bytes (caller 가 보유, FU 60 의 directory 디코드 시 reuse).
    pub raw: Bytes,
}

/// PMTiles raw bytes → magic + version 검증.
///
/// 통과 시 [`PmtilesHeader`] 반환. directory / tile_at 디코드는 미구현 (FU 60).
///
/// # Errors
///
/// - magic 7 byte 불일치 → [`ParseError::MagicMismatch`]
/// - version `3` 외 → [`ParseError::UnsupportedVersion`]
/// - 8 byte 미만 → [`ParseError::Malformed`]
pub fn parse_header(raw: &Bytes) -> Result<PmtilesHeader, ParseError> {
    if raw.len() < 8 {
        return Err(ParseError::Malformed(format!(
            "PMTiles 가 너무 작음: {} byte (8 byte 헤더 필요)",
            raw.len()
        )));
    }

    let magic = &raw[0..7];
    if magic != PMTILES_MAGIC {
        let mut got = [0u8; 7];
        got.copy_from_slice(magic);
        return Err(ParseError::MagicMismatch { got });
    }

    let version = raw[7];
    if version != SUPPORTED_VERSION {
        return Err(ParseError::UnsupportedVersion(version));
    }

    Ok(PmtilesHeader {
        version,
        raw: raw.clone(),
    })
}

/// FU 60 까지 honest failure marker.
///
/// 1차 reader (Parcel / Building) 가 호출 시 `Err(Fetch(...))` 로 매핑돼
/// 도메인 layer 가 V-World 합성 fallback 으로 진행.
#[must_use]
pub const fn tile_at_pending_message() -> &'static str {
    "PMTiles tile_at decode pending FU 60 (ETL 빌더 + production fixture)"
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    fn valid_header_prefix() -> Bytes {
        let mut v = Vec::with_capacity(127);
        v.extend_from_slice(b"PMTiles"); // 7 byte magic
        v.push(3); // version
                   // 나머지 119 byte 0 — header zone full size 는 127. 1차 파서는 8 byte 만 검증.
        v.resize(127, 0);
        Bytes::from(v)
    }

    #[test]
    fn parse_valid_v3_header_succeeds() {
        let h = parse_header(&valid_header_prefix()).expect("valid");
        assert_eq!(h.version, 3);
    }

    #[test]
    fn parse_too_short_returns_malformed() {
        let bytes = Bytes::from(b"PMTi".to_vec());
        let err = parse_header(&bytes).unwrap_err();
        assert!(matches!(err, ParseError::Malformed(_)));
    }

    #[test]
    fn parse_wrong_magic_returns_mismatch() {
        // 8 byte 라야 length check 통과 후 magic 검사로 진입.
        let bytes = Bytes::from(b"NOTPMT_X".to_vec());
        let err = parse_header(&bytes).unwrap_err();
        assert!(matches!(err, ParseError::MagicMismatch { .. }));
    }

    #[test]
    fn parse_unsupported_version_returns_error() {
        let mut v = b"PMTiles".to_vec();
        v.push(2); // v2 = unsupported
        let bytes = Bytes::from(v);
        let err = parse_header(&bytes).unwrap_err();
        assert!(matches!(err, ParseError::UnsupportedVersion(2)));
    }

    #[test]
    fn tile_at_pending_message_mentions_fu_60() {
        assert!(tile_at_pending_message().contains("FU 60"));
    }
}
