//! Development-only parcel lookup fallback.
//!
//! The production API requires `PLATFORM_CORE_API_BASE_URL` because Platform
//! Core owns canonical Catalog parcel data. This fallback exists only for local
//! development and tests that do not need catalog enrichment.

use async_trait::async_trait;
use shared_kernel::pnu::Pnu;

use crate::info::ParcelInfo;
use crate::lookup::{LookupError, ParcelInfoLookup};

/// Lookup implementation that returns `Ok(None)` for every PNU.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoOpParcelInfoLookup;

impl NoOpParcelInfoLookup {
    /// Create a `NoOp` lookup instance.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ParcelInfoLookup for NoOpParcelInfoLookup {
    async fn lookup_by_pnu(&self, _pnu: &Pnu) -> Result<Option<ParcelInfo>, LookupError> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[tokio::test]
    async fn noop_returns_none_for_any_pnu() {
        let lookup = NoOpParcelInfoLookup::new();
        let pnu = Pnu::try_new("1168010100107370000").unwrap();
        assert!(lookup.lookup_by_pnu(&pnu).await.unwrap().is_none());
    }
}
