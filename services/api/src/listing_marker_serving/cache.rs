use listing_domain::repository::{
    ListingMarkerCount, ListingMarkerMask, ListingMarkerMaskEncoding, ListingMarkerTile, RepoError,
    LISTING_MARKER_TILE_LAYER,
};
use serde::{Deserialize, Serialize};

const TILE_CACHE_MAGIC: &[u8; 4] = b"LMT1";

pub(super) fn encode_cached_tile(tile: &ListingMarkerTile) -> Vec<u8> {
    let anchor = tile.anchor_snapshot_id.as_deref().unwrap_or("").as_bytes();
    let mut payload = Vec::with_capacity(48 + anchor.len() + tile.bytes.len());
    payload.extend_from_slice(TILE_CACHE_MAGIC);
    push_i64(&mut payload, tile.eligible_count);
    push_i64(&mut payload, tile.represented_count);
    push_i64(&mut payload, tile.feature_count);
    push_i64(&mut payload, tile.aggregate_count);
    push_u32(
        &mut payload,
        u32::try_from(anchor.len()).unwrap_or(u32::MAX),
    );
    payload.extend_from_slice(anchor);
    push_u32(
        &mut payload,
        u32::try_from(tile.bytes.len()).unwrap_or(u32::MAX),
    );
    payload.extend_from_slice(&tile.bytes);
    payload
}

pub(super) fn decode_cached_tile(payload: &[u8]) -> Result<ListingMarkerTile, RepoError> {
    let mut cursor = 0;
    let magic = take(payload, &mut cursor, TILE_CACHE_MAGIC.len())?;
    if magic != TILE_CACHE_MAGIC {
        return Err(cache_decode_err("invalid tile cache magic"));
    }
    let eligible_count = read_i64(payload, &mut cursor)?;
    let represented_count = read_i64(payload, &mut cursor)?;
    let feature_count = read_i64(payload, &mut cursor)?;
    let aggregate_count = read_i64(payload, &mut cursor)?;
    let anchor_len = usize::try_from(read_u32(payload, &mut cursor)?)
        .map_err(|_| cache_decode_err("invalid anchor length"))?;
    let anchor_bytes = take(payload, &mut cursor, anchor_len)?;
    let bytes_len = usize::try_from(read_u32(payload, &mut cursor)?)
        .map_err(|_| cache_decode_err("invalid tile byte length"))?;
    let bytes = take(payload, &mut cursor, bytes_len)?.to_vec();
    if cursor != payload.len() {
        return Err(cache_decode_err("trailing tile cache bytes"));
    }
    let anchor_snapshot_id = if anchor_bytes.is_empty() {
        None
    } else {
        Some(
            String::from_utf8(anchor_bytes.to_vec())
                .map_err(|_| cache_decode_err("invalid anchor snapshot utf-8"))?,
        )
    };

    Ok(ListingMarkerTile {
        bytes,
        layer_name: LISTING_MARKER_TILE_LAYER,
        eligible_count,
        represented_count,
        feature_count,
        aggregate_count,
        anchor_snapshot_id,
    })
}

#[derive(Debug, Deserialize, Serialize)]
struct CachedListingMarkerCount {
    total_count: i64,
    projection_version: Option<i64>,
    anchor_snapshot_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct CachedListingMarkerMask {
    encoding: String,
    marker_ids: Vec<String>,
    projection_version: Option<i64>,
    anchor_snapshot_id: Option<String>,
}

pub(super) fn encode_count_cache(count: &ListingMarkerCount) -> Result<Vec<u8>, RepoError> {
    encode_json_cache(&CachedListingMarkerCount {
        total_count: count.total_count,
        projection_version: count.projection_version,
        anchor_snapshot_id: count.anchor_snapshot_id.clone(),
    })
}

pub(super) fn decode_count_cache(payload: &[u8]) -> Result<ListingMarkerCount, RepoError> {
    let cached: CachedListingMarkerCount = decode_json_cache(payload)?;
    Ok(ListingMarkerCount {
        total_count: cached.total_count,
        projection_version: cached.projection_version,
        anchor_snapshot_id: cached.anchor_snapshot_id,
    })
}

pub(super) fn encode_mask_cache(mask: &ListingMarkerMask) -> Result<Vec<u8>, RepoError> {
    encode_json_cache(&CachedListingMarkerMask {
        encoding: mask.encoding.as_str().to_owned(),
        marker_ids: mask.marker_ids.clone(),
        projection_version: mask.projection_version,
        anchor_snapshot_id: mask.anchor_snapshot_id.clone(),
    })
}

pub(super) fn decode_mask_cache(payload: &[u8]) -> Result<ListingMarkerMask, RepoError> {
    let cached: CachedListingMarkerMask = decode_json_cache(payload)?;
    let encoding = match cached.encoding.as_str() {
        "show" => ListingMarkerMaskEncoding::Show,
        "hide" => ListingMarkerMaskEncoding::Hide,
        _ => return Err(cache_decode_err("invalid marker mask encoding")),
    };
    Ok(ListingMarkerMask {
        encoding,
        marker_ids: cached.marker_ids,
        projection_version: cached.projection_version,
        anchor_snapshot_id: cached.anchor_snapshot_id,
    })
}

fn encode_json_cache<T>(value: &T) -> Result<Vec<u8>, RepoError>
where
    T: Serialize,
{
    serde_json::to_vec(value).map_err(|error| {
        RepoError::Database(format!("listing marker cache encode failed: {error}"))
    })
}

fn decode_json_cache<T>(payload: &[u8]) -> Result<T, RepoError>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_slice(payload).map_err(|error| {
        RepoError::Database(format!("listing marker cache decode failed: {error}"))
    })
}

fn push_i64(payload: &mut Vec<u8>, value: i64) {
    payload.extend_from_slice(&value.to_be_bytes());
}

fn push_u32(payload: &mut Vec<u8>, value: u32) {
    payload.extend_from_slice(&value.to_be_bytes());
}

fn read_i64(payload: &[u8], cursor: &mut usize) -> Result<i64, RepoError> {
    let bytes = take(payload, cursor, 8)?;
    let array: [u8; 8] = bytes
        .try_into()
        .map_err(|_| cache_decode_err("invalid i64 width"))?;
    Ok(i64::from_be_bytes(array))
}

fn read_u32(payload: &[u8], cursor: &mut usize) -> Result<u32, RepoError> {
    let bytes = take(payload, cursor, 4)?;
    let array: [u8; 4] = bytes
        .try_into()
        .map_err(|_| cache_decode_err("invalid u32 width"))?;
    Ok(u32::from_be_bytes(array))
}

fn take<'a>(payload: &'a [u8], cursor: &mut usize, len: usize) -> Result<&'a [u8], RepoError> {
    let end = cursor
        .checked_add(len)
        .ok_or_else(|| cache_decode_err("tile cache cursor overflow"))?;
    let bytes = payload
        .get(*cursor..end)
        .ok_or_else(|| cache_decode_err("tile cache truncated"))?;
    *cursor = end;
    Ok(bytes)
}

fn cache_decode_err(message: &str) -> RepoError {
    RepoError::Database(format!("listing marker cache decode failed: {message}"))
}

#[cfg(test)]
mod tests {
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
    fn cached_tile_codec_round_trips_metadata_and_bytes() -> Result<(), RepoError> {
        let tile = sample_tile(vec![1, 2, 3, 4, 5]);

        let encoded = encode_cached_tile(&tile);
        let decoded = decode_cached_tile(&encoded)?;

        assert_eq!(decoded, tile);
        Ok(())
    }
}
