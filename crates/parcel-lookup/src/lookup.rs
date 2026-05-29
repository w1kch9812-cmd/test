//! Parcel lookup port and errors.

use async_trait::async_trait;
use shared_kernel::pnu::Pnu;
use thiserror::Error;

use crate::info::ParcelInfo;

/// PNU to Gongzzang parcel information lookup port.
#[async_trait]
pub trait ParcelInfoLookup: Send + Sync {
    /// Look up the narrow parcel information needed by Gongzzang.
    ///
    /// # Errors
    ///
    /// - [`LookupError::Backend`] when the Platform Core API call fails.
    /// - [`LookupError::Parse`] when Platform Core returns an invalid contract payload.
    async fn lookup_by_pnu(&self, pnu: &Pnu) -> Result<Option<ParcelInfo>, LookupError>;
}

/// Lookup error mapped by the route layer into RFC 7807 responses.
#[derive(Debug, Error)]
pub enum LookupError {
    /// Platform Core API call failed or returned a non-success status.
    #[error("backend error: {0}")]
    Backend(String),
    /// Platform Core returned an invalid or inconsistent payload.
    #[error("parse error: {0}")]
    Parse(String),
}
