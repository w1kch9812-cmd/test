//! Gongzzang listing marker filter contract.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use shared_kernel::listing_type::ListingType;
use shared_kernel::transaction_type::TransactionType;
use thiserror::Error;

/// Initial public listing marker filter hash.
pub const ALL_ACTIVE_LISTING_MARKER_FILTER_HASH: &str = "all-active-v1";

/// Client-provided listing marker filter payload before canonicalization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListingMarkerFilterSpec {
    /// Listing asset types selected by the user.
    pub types: Vec<ListingType>,
    /// Transaction types selected by the user.
    pub transactions: Vec<TransactionType>,
    /// Inclusive minimum area in square meters.
    pub min_area_m2: Option<i64>,
    /// Inclusive maximum area in square meters.
    pub max_area_m2: Option<i64>,
    /// Inclusive minimum price in Korean won.
    pub min_price_krw: Option<i64>,
    /// Inclusive maximum price in Korean won.
    pub max_price_krw: Option<i64>,
}

/// Canonical listing marker filter payload used for stable hashing and server indexes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedListingMarkerFilterSpec {
    /// Canonically sorted and deduplicated listing asset types.
    pub types: Vec<ListingType>,
    /// Canonically sorted and deduplicated transaction types.
    pub transactions: Vec<TransactionType>,
    /// Inclusive minimum area in square meters.
    pub min_area_m2: Option<i64>,
    /// Inclusive maximum area in square meters.
    pub max_area_m2: Option<i64>,
    /// Inclusive minimum price in Korean won.
    pub min_price_krw: Option<i64>,
    /// Inclusive maximum price in Korean won.
    pub max_price_krw: Option<i64>,
}

/// Supported listing marker filter identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListingMarkerFilter {
    /// Public active listings without additional user filters.
    AllActive,
    /// A normalized registered filter payload.
    Normalized(NormalizedListingMarkerFilterSpec),
}

/// Listing marker filter parse or validation error.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ListingMarkerFilterError {
    /// The filter hash is not registered or supported.
    #[error("unsupported listing marker filter hash: {0}")]
    UnsupportedHash(String),
    /// A minimum value is greater than the corresponding maximum value.
    #[error("invalid listing marker filter range: {field}")]
    InvalidRange {
        /// Invalid field name.
        field: &'static str,
    },
}

impl ListingMarkerFilterSpec {
    /// Validate and canonicalize a listing marker filter.
    ///
    /// # Errors
    ///
    /// Returns [`ListingMarkerFilterError::InvalidRange`] when a minimum bound is greater than the
    /// matching maximum bound.
    pub fn try_normalized(
        self,
    ) -> Result<NormalizedListingMarkerFilterSpec, ListingMarkerFilterError> {
        validate_range("min_area_m2", self.min_area_m2, self.max_area_m2)?;
        validate_range("min_price_krw", self.min_price_krw, self.max_price_krw)?;

        let mut types = self.types;
        types.sort_by_key(|value| value.as_str());
        types.dedup();

        let mut transactions = self.transactions;
        transactions.sort_by_key(|value| value.as_str());
        transactions.dedup();

        Ok(NormalizedListingMarkerFilterSpec {
            types,
            transactions,
            min_area_m2: self.min_area_m2,
            max_area_m2: self.max_area_m2,
            min_price_krw: self.min_price_krw,
            max_price_krw: self.max_price_krw,
        })
    }
}

impl NormalizedListingMarkerFilterSpec {
    /// Return the stable cache/index identity for this normalized filter.
    #[must_use]
    pub fn filter_hash(&self) -> String {
        if self.is_all_active() {
            return ALL_ACTIVE_LISTING_MARKER_FILTER_HASH.to_owned();
        }

        let canonical = format!(
            "v1|types={}|tx={}|area={}:{}|price={}:{}",
            self.types
                .iter()
                .map(|value| value.as_str())
                .collect::<Vec<_>>()
                .join(","),
            self.transactions
                .iter()
                .map(|value| value.as_str())
                .collect::<Vec<_>>()
                .join(","),
            opt_i64(self.min_area_m2),
            opt_i64(self.max_area_m2),
            opt_i64(self.min_price_krw),
            opt_i64(self.max_price_krw),
        );
        let digest = Sha256::digest(canonical.as_bytes());
        format!("lst_filter_v1_{digest:x}")
    }

    /// Return whether this normalized payload is equivalent to the built-in all-active filter.
    #[must_use]
    pub const fn is_all_active(&self) -> bool {
        self.types.is_empty()
            && self.transactions.is_empty()
            && self.min_area_m2.is_none()
            && self.max_area_m2.is_none()
            && self.min_price_krw.is_none()
            && self.max_price_krw.is_none()
    }
}

impl ListingMarkerFilter {
    /// Parse a built-in public filter hash into a typed marker filter.
    ///
    /// # Errors
    ///
    /// Returns [`ListingMarkerFilterError::UnsupportedHash`] when `hash` is not a built-in filter.
    pub fn try_from_hash(hash: &str) -> Result<Self, ListingMarkerFilterError> {
        match hash {
            ALL_ACTIVE_LISTING_MARKER_FILTER_HASH => Ok(Self::AllActive),
            other => Err(ListingMarkerFilterError::UnsupportedHash(other.to_owned())),
        }
    }

    /// Stable cache identity for this filter.
    #[must_use]
    pub fn hash(&self) -> String {
        match self {
            Self::AllActive => ALL_ACTIVE_LISTING_MARKER_FILTER_HASH.to_owned(),
            Self::Normalized(spec) => spec.filter_hash(),
        }
    }

    /// Convert the filter identity into a normalized payload.
    #[must_use]
    pub fn into_spec(self) -> NormalizedListingMarkerFilterSpec {
        match self {
            Self::AllActive => NormalizedListingMarkerFilterSpec {
                types: Vec::new(),
                transactions: Vec::new(),
                min_area_m2: None,
                max_area_m2: None,
                min_price_krw: None,
                max_price_krw: None,
            },
            Self::Normalized(spec) => spec,
        }
    }
}

const fn validate_range(
    field: &'static str,
    min: Option<i64>,
    max: Option<i64>,
) -> Result<(), ListingMarkerFilterError> {
    match (min, max) {
        (Some(min), Some(max)) if min > max => {
            Err(ListingMarkerFilterError::InvalidRange { field })
        }
        _ => Ok(()),
    }
}

fn opt_i64(value: Option<i64>) -> String {
    value.map_or_else(|| "_".to_owned(), |inner| inner.to_string())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;
    use shared_kernel::listing_type::ListingType;
    use shared_kernel::transaction_type::TransactionType;

    #[test]
    fn equivalent_filter_order_produces_same_hash() {
        let first = ListingMarkerFilterSpec {
            types: vec![ListingType::Warehouse, ListingType::Factory],
            transactions: vec![TransactionType::Sale, TransactionType::Jeonse],
            min_area_m2: Some(100),
            max_area_m2: Some(5000),
            min_price_krw: None,
            max_price_krw: Some(5_000_000_000),
        };
        let second = ListingMarkerFilterSpec {
            types: vec![ListingType::Factory, ListingType::Warehouse],
            transactions: vec![TransactionType::Jeonse, TransactionType::Sale],
            min_area_m2: Some(100),
            max_area_m2: Some(5000),
            min_price_krw: None,
            max_price_krw: Some(5_000_000_000),
        };

        assert_eq!(
            first.try_normalized().expect("valid first").filter_hash(),
            second.try_normalized().expect("valid second").filter_hash()
        );
    }

    #[test]
    fn all_active_v1_stays_supported() {
        let filter = ListingMarkerFilter::try_from_hash("all-active-v1").expect("filter");

        assert_eq!(filter.hash(), "all-active-v1");
        assert_eq!(filter.into_spec().types, Vec::<ListingType>::new());
    }

    #[test]
    fn invalid_range_is_rejected() {
        let err = ListingMarkerFilterSpec {
            types: vec![],
            transactions: vec![],
            min_area_m2: Some(5000),
            max_area_m2: Some(100),
            min_price_krw: None,
            max_price_krw: None,
        }
        .try_normalized()
        .expect_err("invalid range");

        assert!(err.to_string().contains("min_area_m2"));
    }
}
