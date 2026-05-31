use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

use super::error::AnchorImporterError;
use super::MAX_FAILURE_REASON_LEN;

pub fn parse_rfc3339_utc(value: &str) -> Result<DateTime<Utc>, AnchorImporterError> {
    Ok(DateTime::parse_from_rfc3339(value)?.with_timezone(&Utc))
}

pub fn truncate_failure_reason(reason: &str) -> String {
    reason.chars().take(MAX_FAILURE_REASON_LEN).collect()
}

pub fn verify_size_bytes(
    actual: usize,
    expected: u64,
    label: &'static str,
    object_key: &str,
) -> Result<(), AnchorImporterError> {
    if u64::try_from(actual).map_err(|_| AnchorImporterError::ArtifactObjectSizeOverflow)?
        == expected
    {
        return Ok(());
    }

    Err(AnchorImporterError::SizeMismatch {
        label,
        object_key: object_key.to_owned(),
        expected,
        actual,
    })
}

pub fn verify_sha256(
    bytes: &[u8],
    expected: &str,
    label: &'static str,
) -> Result<(), AnchorImporterError> {
    let digest = Sha256::digest(bytes);
    let actual = digest
        .iter()
        .fold(String::with_capacity(64), |mut output, byte| {
            use std::fmt::Write as _;
            let _ = write!(&mut output, "{byte:02x}");
            output
        });
    if actual == expected {
        return Ok(());
    }

    Err(AnchorImporterError::ChecksumMismatch {
        label,
        expected: expected.to_owned(),
        actual,
    })
}
