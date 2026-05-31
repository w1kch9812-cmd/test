use crate::listing_marker_policy::{
    MAX_LISTING_MARKER_MASK_IDS, MAX_LISTING_MARKER_TILE_BYTES, MAX_LISTING_MARKER_TILE_FEATURES,
};
use listing_domain::repository::{
    ListingMarkerDeltas, ListingMarkerMask, ListingMarkerTile, ListingMarkerTombstones, RepoError,
};

pub(super) fn validate_tile_budget(tile: &ListingMarkerTile) -> Result<(), RepoError> {
    if tile.bytes.len() > MAX_LISTING_MARKER_TILE_BYTES {
        return Err(RepoError::Database(format!(
            "listing marker tile budget violation: bytes={} max={}",
            tile.bytes.len(),
            MAX_LISTING_MARKER_TILE_BYTES
        )));
    }
    if tile.feature_count < 0 || tile.aggregate_count < 0 {
        return Err(RepoError::Database(
            "listing marker tile budget violation: negative feature count".to_owned(),
        ));
    }
    let feature_total = tile.feature_count + tile.aggregate_count;
    if feature_total > MAX_LISTING_MARKER_TILE_FEATURES {
        return Err(RepoError::Database(format!(
            "listing marker tile budget violation: features={feature_total} max={MAX_LISTING_MARKER_TILE_FEATURES}",
        )));
    }
    Ok(())
}

pub(super) fn validate_mask_budget(mask: &ListingMarkerMask) -> Result<(), RepoError> {
    if mask.marker_ids.len() > MAX_LISTING_MARKER_MASK_IDS {
        return Err(RepoError::Database(format!(
            "listing marker mask budget violation: marker_ids={} max={}",
            mask.marker_ids.len(),
            MAX_LISTING_MARKER_MASK_IDS
        )));
    }
    Ok(())
}

pub(super) fn validate_tombstone_budget(
    tombstones: &ListingMarkerTombstones,
) -> Result<(), RepoError> {
    if tombstones.marker_ids.len() > MAX_LISTING_MARKER_MASK_IDS {
        return Err(RepoError::Database(format!(
            "listing marker tombstone budget violation: marker_ids={} max={}",
            tombstones.marker_ids.len(),
            MAX_LISTING_MARKER_MASK_IDS
        )));
    }
    Ok(())
}

pub(super) fn validate_delta_budget(deltas: &ListingMarkerDeltas) -> Result<(), RepoError> {
    if deltas.bytes.len() > MAX_LISTING_MARKER_TILE_BYTES {
        return Err(RepoError::Database(format!(
            "listing marker delta budget violation: bytes={} max={}",
            deltas.bytes.len(),
            MAX_LISTING_MARKER_TILE_BYTES
        )));
    }
    if deltas.feature_count < 0 {
        return Err(RepoError::Database(
            "listing marker delta budget violation: negative feature count".to_owned(),
        ));
    }
    if deltas.feature_count > MAX_LISTING_MARKER_TILE_FEATURES {
        return Err(RepoError::Database(format!(
            "listing marker delta budget violation: features={} max={MAX_LISTING_MARKER_TILE_FEATURES}",
            deltas.feature_count
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use listing_domain::repository::{ListingMarkerTile, LISTING_MARKER_TILE_LAYER};

    use super::*;

    fn sample_tile(bytes: Vec<u8>) -> ListingMarkerTile {
        ListingMarkerTile {
            bytes,
            layer_name: LISTING_MARKER_TILE_LAYER,
            eligible_count: 3,
            represented_count: 3,
            feature_count: 2,
            aggregate_count: 1,
            anchor_snapshot_id: Some("snapshot-test-v1".to_owned()),
        }
    }

    #[test]
    fn listing_marker_tile_budget_rejects_overlarge_payload() -> Result<(), String> {
        let tile = sample_tile(vec![0; MAX_LISTING_MARKER_TILE_BYTES + 1]);

        match validate_tile_budget(&tile) {
            Err(err) => {
                assert!(err.to_string().contains("budget violation"));
                Ok(())
            }
            Ok(()) => Err("over budget tile must fail".to_owned()),
        }
    }
}
