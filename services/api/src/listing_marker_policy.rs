//! Generated listing marker serving policy from docs/architecture/traffic-auth-policy-registry.v1.json.
//! Run scripts/ci/generate-traffic-auth-policy.ps1 after editing the registry.

pub const MAX_LISTING_MARKER_TILE_BYTES: usize = 262_144;
pub const MAX_LISTING_MARKER_TILE_FEATURES: i64 = 10_000;
pub const MAX_LISTING_MARKER_MASK_IDS: usize = 20_000;
pub const LISTING_MARKER_CACHE_TTL_SECONDS: u64 = 30;
pub const LISTING_MARKER_SINGLE_FLIGHT_LOCK_SECONDS: u64 = 5;
pub const LISTING_MARKER_SINGLE_FLIGHT_WAIT_ATTEMPTS: usize = 10;
pub const LISTING_MARKER_SINGLE_FLIGHT_WAIT_MS: u64 = 50;
