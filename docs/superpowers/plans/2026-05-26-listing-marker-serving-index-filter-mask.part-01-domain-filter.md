# Listing Marker Serving Index And Filter Mask Plan - Part 1: Domain Filter Contract

> Extracted from `2026-05-26-listing-marker-serving-index-filter-mask.md` to keep each plan file below the 500-line SSS guardrail.
> See the index file for the full sequence and cross-links.

## Task 1: Domain Filter Contract

**Files:**
- Create: `crates/domain/core/listing/src/marker_filter.rs`
- Modify: `crates/domain/core/listing/src/lib.rs`
- Modify: `crates/domain/core/listing/src/repository.rs`
- Modify: `crates/domain/core/listing/Cargo.toml`

- [x] **Step 1: Add failing unit tests for canonicalization**

Create `crates/domain/core/listing/src/marker_filter.rs` with tests first. The module should compile-fail until the types are implemented.

```rust
#[cfg(test)]
mod tests {
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

        let first = match first.try_normalized() {
            Ok(value) => value,
            Err(err) => panic!("valid first filter rejected: {err}"),
        };
        let second = match second.try_normalized() {
            Ok(value) => value,
            Err(err) => panic!("valid second filter rejected: {err}"),
        };

        assert_eq!(first.filter_hash(), second.filter_hash());
    }

    #[test]
    fn all_active_v1_stays_supported() {
        let filter = match ListingMarkerFilter::try_from_hash("all-active-v1") {
            Ok(value) => value,
            Err(err) => panic!("all-active-v1 rejected: {err}"),
        };

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
```

- [x] **Step 2: Run the failing tests**

Run:

```bash
cargo test -p listing-domain marker_filter
```

Expected: failure because `ListingMarkerFilterSpec`, `ListingMarkerFilter`, and range validation are not implemented.

- [x] **Step 3: Implement the filter module**

Implement the module with these public types:

```rust
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use shared_kernel::listing_type::ListingType;
use shared_kernel::transaction_type::TransactionType;
use thiserror::Error;

pub const ALL_ACTIVE_LISTING_MARKER_FILTER_HASH: &str = "all-active-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListingMarkerFilterSpec {
    pub types: Vec<ListingType>,
    pub transactions: Vec<TransactionType>,
    pub min_area_m2: Option<i64>,
    pub max_area_m2: Option<i64>,
    pub min_price_krw: Option<i64>,
    pub max_price_krw: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedListingMarkerFilterSpec {
    pub types: Vec<ListingType>,
    pub transactions: Vec<TransactionType>,
    pub min_area_m2: Option<i64>,
    pub max_area_m2: Option<i64>,
    pub min_price_krw: Option<i64>,
    pub max_price_krw: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListingMarkerFilter {
    AllActive,
    Normalized(NormalizedListingMarkerFilterSpec),
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ListingMarkerFilterError {
    #[error("unsupported listing marker filter hash: {0}")]
    UnsupportedHash(String),
    #[error("invalid listing marker filter range: {field}")]
    InvalidRange { field: &'static str },
}
```

Normalization rules:

```rust
impl ListingMarkerFilterSpec {
    pub fn try_normalized(self) -> Result<NormalizedListingMarkerFilterSpec, ListingMarkerFilterError> {
        validate_range("min_area_m2", self.min_area_m2, self.max_area_m2)?;
        validate_range("min_price_krw", self.min_price_krw, self.max_price_krw)?;

        let mut types = self.types;
        types.sort_by_key(|v| v.as_str().to_owned());
        types.dedup();

        let mut transactions = self.transactions;
        transactions.sort_by_key(|v| v.as_str().to_owned());
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
```

Stable hash rule:

```rust
impl NormalizedListingMarkerFilterSpec {
    #[must_use]
    pub fn filter_hash(&self) -> String {
        if self.is_all_active() {
            return ALL_ACTIVE_LISTING_MARKER_FILTER_HASH.to_owned();
        }

        let canonical = format!(
            "v1|types={}|tx={}|area={}:{}|price={}:{}",
            self.types.iter().map(ListingType::as_str).collect::<Vec<_>>().join(","),
            self.transactions.iter().map(TransactionType::as_str).collect::<Vec<_>>().join(","),
            opt_i64(self.min_area_m2),
            opt_i64(self.max_area_m2),
            opt_i64(self.min_price_krw),
            opt_i64(self.max_price_krw),
        );
        let digest = Sha256::digest(canonical.as_bytes());
        format!("lst_filter_v1_{:x}", digest)
    }

    #[must_use]
    pub fn is_all_active(&self) -> bool {
        self.types.is_empty()
            && self.transactions.is_empty()
            && self.min_area_m2.is_none()
            && self.max_area_m2.is_none()
            && self.min_price_krw.is_none()
            && self.max_price_krw.is_none()
    }
}
```

Filter hash parsing must not reconstruct arbitrary historical hashes from the hash string alone. The API
registers normalized filter payloads and stores the hash/spec mapping in the serving layer. `all-active-v1`
is the only built-in hash because it has no payload:

```rust
impl ListingMarkerFilter {
    pub fn try_from_hash(value: &str) -> Result<Self, ListingMarkerFilterError> {
        if value == ALL_ACTIVE_LISTING_MARKER_FILTER_HASH {
            return Ok(Self::AllActive);
        }
        Err(ListingMarkerFilterError::UnsupportedHash(value.to_owned()))
    }

    #[must_use]
    pub fn hash(&self) -> String {
        match self {
            Self::AllActive => ALL_ACTIVE_LISTING_MARKER_FILTER_HASH.to_owned(),
            Self::Normalized(spec) => spec.filter_hash(),
        }
    }

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
```

- [x] **Step 4: Wire module exports**

Update `crates/domain/core/listing/src/lib.rs`:

```rust
pub mod marker_filter;
```

Update `crates/domain/core/listing/src/repository.rs` to import and re-export marker filter types from the module instead of defining `ListingMarkerFilter` inline. Keep `LISTING_MARKER_TILE_LAYER`, `LISTING_MARKER_TILE_CONTENT_TYPE`, and tile query types in `repository.rs`.

- [x] **Step 5: Add crate dependencies**

Update `crates/domain/core/listing/Cargo.toml`:

```toml
sha2 = { workspace = true }
```

`serde` and `thiserror` are already present.

- [x] **Step 6: Run domain tests**

Run:

```bash
cargo test -p listing-domain marker_filter
```

Expected: marker filter tests pass.
