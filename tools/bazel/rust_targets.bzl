"""Rust verification target sets for Bazel-native checks."""

RUST_CRATE_TARGETS = [
    "//crates/auth:auth",
    "//crates/circuit-breaker:circuit_breaker",
    "//crates/db:db",
    "//crates/domain/audit/audit-log:audit_log_domain",
    "//crates/domain/audit/outbox-event:outbox_event_domain",
    "//crates/domain/core/listing-photo:listing_photo_domain",
    "//crates/domain/core/listing:listing_domain",
    "//crates/domain/core/shared-kernel:shared_kernel",
    "//crates/domain/core/user:user_domain",
    "//crates/domain/insights/analysis-report:analysis_report_domain",
    "//crates/domain/insights/bookmark:bookmark_domain",
    "//crates/domain/insights/notification:notification_domain",
    "//crates/domain/insights/search-history:search_history_domain",
    "//crates/domain/market/court-auction:court_auction_domain",
    "//crates/domain/market/real-transaction:real_transaction_domain",
    "//crates/operations/admin-action:admin_action_domain",
    "//crates/operations/business-verification-queue:business_verification_queue_domain",
    "//crates/operations/listing-report:listing_report_domain",
    "//crates/operations/listing-review-queue:lrq_domain",
    "//crates/operations/operations-meta:operations_meta_domain",
    "//crates/outbox-publisher:outbox_publisher",
    "//crates/parcel-lookup:parcel_lookup",
    "//services/api:api_service",
    "//services/api:platform_core_anchor_import",
    "//services/etl-base-layer:etl_base_layer_service",
    "//services/outbox-publisher:outbox_publisher_service",
]

RUST_UNIT_TEST_TARGETS = [
    target + "_unit_test"
    for target in RUST_CRATE_TARGETS
]
