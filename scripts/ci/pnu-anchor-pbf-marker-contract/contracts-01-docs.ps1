$contracts += @(
    [pscustomobject]@{
        RelativePath = "AGENTS.md"
        Tokens = @(
            "ADR 0037",
            "Listing PBF design spec",
            "owns parcel geometry",
            "owns listing semantics and Gongzzang-owned listing PBF marker tiles",
            "listing rows must not own canonical marker coordinates",
            "marker request shapes",
            "verification-first",
            "tests, migration smoke, and"
        )
        Forbidden = @(
            "platform-core owns Gongzzang listing price",
            "platform-core owns Gongzzang listing status",
            "platform-core owns Gongzzang listing exposure"
        )
    },
    [pscustomobject]@{
        RelativePath = "docs/adr/0037-pnu-anchor-pbf-marker-tiles.md"
        Tokens = @(
            "marker_tile_response_format = MVT_PBF",
            "marker_position_source = PNU_ANCHOR",
            "bbox_marker_runtime_forbidden = true",
            "dropped_marker_success_forbidden = true",
            "Gongzzang remains the SSOT for listing semantics",
            "dynamic PBF generated from listing rows joined to platform-core anchors by PNU",
            "Product-specific listing marker PBF tiles are a Gongzzang market-domain runtime surface",
            "find_listing_marker_tile",
            "parcel_marker_anchor",
            "Active listing saves are rejected",
            "GET /map/v1/marker-tiles/listing/{z}/{x}/{y}.pbf?filter_hash=all-active-v1",
            "approved by the user on",
            "No Gongzzang launch map/listing path may depend on viewport bounds as its public request shape"
        )
        Forbidden = @(
            "platform-core owns Gongzzang listing price",
            "platform-core owns Gongzzang listing status",
            "platform-core owns Gongzzang listing exposure"
        )
    },
    [pscustomobject]@{
        RelativePath = "docs/adr/0038-listing-marker-serving-index-filter-mask.md"
        Tokens = @(
            "listing_marker_projection",
            "listing_marker_filter_registry",
            "PNU anchor",
            "marker-counts/listing",
            "marker-masks/listing",
            "browser instant filtering"
        )
    },
    [pscustomobject]@{
        RelativePath = "docs/superpowers/specs/2026-05-22-gongzzang-owned-listing-pbf-marker-tiles-design.md"
        Tokens = @(
            "Gongzzang-owned listing PBF marker tiles",
            "platform-core owns PNU anchors",
            "Gongzzang owns listing semantics",
            "No listing-owned canonical coordinate",
            "No viewport-bounds public marker API",
            "No silent marker drop"
        )
    },
    [pscustomobject]@{
        RelativePath = "docs/superpowers/specs/2026-05-26-listing-marker-serving-index-filter-mask-design.md"
        Tokens = @(
            "listing_marker_projection",
            "filter_hash",
            "base marker tile",
            "browser instant filter",
            "server marker/filter index",
            "optional filter mask"
        )
    },
    [pscustomobject]@{
        RelativePath = "docs/superpowers/plans/2026-05-26-listing-marker-serving-index-filter-mask.md"
        Tokens = @(
            "listing_marker_projection",
            "listing_marker_filter_registry",
            "buildListingMarkerLayerFilter",
            "marker-counts/listing",
            "marker-masks/listing"
        )
    },
    [pscustomobject]@{
        RelativePath = "docs/superpowers/plans/2026-05-22-gongzzang-owned-listing-pbf-marker-tiles.md"
        Tokens = @(
            "Serve Gongzzang-owned active listing marker tiles as MVT/PBF",
            "Successful tiles represent every eligible listing",
            "migrations/30012_parcel_marker_anchor_projection.sql",
            "services/api/src/routes/listing_marker_tiles.rs",
            "scripts/ci/check-pnu-anchor-pbf-marker-contract.ps1"
        )
    },
    [pscustomobject]@{
        RelativePath = "docs/superpowers/handoff/2026-05-22-listing-pbf-review-gate.md"
        Tokens = @(
            "Implementation slice verified locally",
            "full project completion not claimed",
            "former `"do not implement yet`" gate is closed",
            "Still Do Not Do",
            "Do not call platform-core databases directly from Gongzzang",
            "If this slice is touched again, re-run the implementation verification checklist"
        )
        Forbidden = @(
            "Runtime listing PBF implementation is still pending",
            "Do not implement the Gongzzang listing PBF endpoint",
            "Do not create the Gongzzang anchor read model migration",
            "Do not switch the frontend to the Gongzzang listing PBF layer",
            "Spec and DB migration approved",
            "implementation verification in progress"
        )
    },
    [pscustomobject]@{
        RelativePath = "docs/superpowers/next-actions.md"
        Tokens = @(
            "local-verification-backed",
            "not a whole-product launch completion claim",
            "handoff/audit verification",
            "platform-core owns PNU anchors; Gongzzang owns listing semantics"
        )
        Forbidden = @(
            "Do not implement the listing PBF endpoint",
            "implementation-approved",
            "Verify the listing PBF endpoint",
            "guardrails before any completion claim"
        )
    },
    [pscustomobject]@{
        RelativePath = "docs/superpowers/roadmap.md"
        Tokens = @(
            "Current supersession",
            "ADR 0037",
            "Gongzzang-owned listing PBF design spec",
            "verification evidence",
            "not a whole-product launch completion claim",
            "handoff/audit verification"
        )
        Forbidden = @(
            "waiting for user review",
            "Do not implement the listing PBF endpoint",
            "implementation-approved",
            "implementation verification in progress"
        )
    },
    [pscustomobject]@{
        RelativePath = "docs/superpowers/handoff/2026-05-22-active-goal-completion-audit.md"
        Tokens = @(
            "Active Goal Completion Audit",
            "Completion claim allowed | false",
            "Prompt-To-Artifact Checklist",
            "completion_claim_allowed=false",
            "Do not call update_goal"
        )
    },
    [pscustomobject]@{
        RelativePath = "migrations/30012_parcel_marker_anchor_projection.sql"
        Tokens = @(
            "create table parcel_marker_anchor",
            "anchor_point geometry(Point, 4326) not null",
            "anchor_snapshot_id",
            "source_geometry_checksum_sha256",
            "platform_core_updated_at",
            "parcel_marker_anchor_srid_chk",
            "parcel_marker_anchor_point_gist_idx"
        )
        Forbidden = @(
            "anchor_lng",
            "anchor_lat"
        )
    }
)
