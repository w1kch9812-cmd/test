//! Catalog event contracts for the platform-core extraction path.
//!
//! These events describe the published language expected for catalog outbox rows.
//! During M1 they are schema only in `gongzzang`; actual catalog ownership moves to
//! `platform-core` during the ADR 0034 cutover sequence.

#![allow(
    clippy::doc_markdown,
    clippy::enum_variant_names,
    clippy::module_name_repetitions
)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::domain_event::DomainEvent;

/// Catalog event schema version used by ADR 0034 M1-M3 handover events.
pub const CATALOG_EVENT_SCHEMA_VERSION: u16 = 1;

/// Catalog event names emitted through the transactional outbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogEventKind {
    /// An `IndustrialComplex` fact changed.
    IndustrialComplexChanged,
    /// A `Parcel` fact changed.
    ParcelChanged,
    /// A `Building` fact changed.
    BuildingChanged,
    /// A `Manufacturer` fact changed.
    ManufacturerChanged,
}

impl CatalogEventKind {
    /// Stable outbox `event_type`.
    #[must_use]
    pub const fn event_type(self) -> &'static str {
        match self {
            Self::IndustrialComplexChanged => "catalog.industrial_complex.changed.v1",
            Self::ParcelChanged => "catalog.parcel.changed.v1",
            Self::BuildingChanged => "catalog.building.changed.v1",
            Self::ManufacturerChanged => "catalog.manufacturer.changed.v1",
        }
    }

    /// Stable outbox `aggregate_kind`.
    #[must_use]
    pub const fn aggregate_kind(self) -> &'static str {
        match self {
            Self::IndustrialComplexChanged => "industrial_complex",
            Self::ParcelChanged => "parcel",
            Self::BuildingChanged => "building",
            Self::ManufacturerChanged => "manufacturer",
        }
    }
}

/// Typed catalog outbox event payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogEventV1 {
    /// Event kind.
    pub kind: CatalogEventKind,
    /// Aggregate id in the owning catalog context.
    pub aggregate_id: String,
    /// Optional `PNU` when the event is parcel-scoped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pnu: Option<String>,
    /// Optional industrial complex code for events scoped to a complex.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub industrial_complex_code: Option<String>,
    /// Source system label, for example `vworld` or `data_go_kr`.
    pub source: String,
    /// Source-side freshness timestamp when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_fetched_at: Option<DateTime<Utc>>,
    /// Event occurrence time in UTC.
    pub occurred_at: DateTime<Utc>,
}

impl CatalogEventV1 {
    /// Create an event with minimal required fields.
    #[must_use]
    pub fn new(
        kind: CatalogEventKind,
        aggregate_id: impl Into<String>,
        source: impl Into<String>,
        occurred_at: DateTime<Utc>,
    ) -> Self {
        Self {
            kind,
            aggregate_id: aggregate_id.into(),
            pnu: None,
            industrial_complex_code: None,
            source: source.into(),
            source_fetched_at: None,
            occurred_at,
        }
    }

    /// Add a `PNU` field.
    #[must_use]
    pub fn with_pnu(mut self, pnu: impl Into<String>) -> Self {
        self.pnu = Some(pnu.into());
        self
    }

    /// Add an industrial complex code field.
    #[must_use]
    pub fn with_industrial_complex_code(mut self, code: impl Into<String>) -> Self {
        self.industrial_complex_code = Some(code.into());
        self
    }

    /// Add the source fetch timestamp.
    #[must_use]
    pub const fn with_source_fetched_at(mut self, fetched_at: DateTime<Utc>) -> Self {
        self.source_fetched_at = Some(fetched_at);
        self
    }

    /// Outbox aggregate kind for this event.
    #[must_use]
    pub const fn aggregate_kind(&self) -> &'static str {
        self.kind.aggregate_kind()
    }
}

impl DomainEvent for CatalogEventV1 {
    fn event_type(&self) -> &'static str {
        self.kind.event_type()
    }

    fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }

    fn aggregate_id(&self) -> String {
        self.aggregate_id.clone()
    }

    fn payload(&self) -> Value {
        json!({
            "schema_version": CATALOG_EVENT_SCHEMA_VERSION,
            "kind": self.kind,
            "aggregate_id": self.aggregate_id,
            "pnu": self.pnu,
            "industrial_complex_code": self.industrial_complex_code,
            "source": self.source,
            "source_fetched_at": self.source_fetched_at,
            "occurred_at": self.occurred_at,
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use chrono::TimeZone;

    use super::*;

    #[test]
    fn parcel_event_has_stable_v1_shape() {
        let at = Utc.with_ymd_and_hms(2026, 5, 12, 1, 2, 3).single().unwrap();
        let event = CatalogEventV1::new(
            CatalogEventKind::ParcelChanged,
            "1111010100100010000",
            "vworld",
            at,
        )
        .with_pnu("1111010100100010000")
        .with_source_fetched_at(at);

        assert_eq!(event.event_type(), "catalog.parcel.changed.v1");
        assert_eq!(event.aggregate_kind(), "parcel");
        assert_eq!(event.aggregate_id(), "1111010100100010000");
        assert_eq!(event.payload()["schema_version"], 1);
        assert_eq!(event.payload()["source"], "vworld");
    }

    #[test]
    fn manufacturer_event_does_not_require_business_number_payload() {
        let at = Utc.with_ymd_and_hms(2026, 5, 12, 1, 2, 3).single().unwrap();
        let event = CatalogEventV1::new(
            CatalogEventKind::ManufacturerChanged,
            "manufacturer:sha256:abc",
            "data_go_kr",
            at,
        );

        assert_eq!(event.event_type(), "catalog.manufacturer.changed.v1");
        assert_eq!(event.aggregate_kind(), "manufacturer");
        assert_eq!(event.payload()["aggregate_id"], "manufacturer:sha256:abc");
        assert!(event.payload()["pnu"].is_null());
    }
}
