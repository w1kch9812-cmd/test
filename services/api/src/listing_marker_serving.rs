use std::sync::Arc;
use std::time::Duration;

use crate::listing_marker_policy::{
    LISTING_MARKER_CACHE_TTL_SECONDS, LISTING_MARKER_SINGLE_FLIGHT_LOCK_SECONDS,
    LISTING_MARKER_SINGLE_FLIGHT_WAIT_ATTEMPTS, LISTING_MARKER_SINGLE_FLIGHT_WAIT_MS,
    MAX_LISTING_MARKER_MASK_IDS, MAX_LISTING_MARKER_TILE_BYTES, MAX_LISTING_MARKER_TILE_FEATURES,
};
use deadpool_redis::redis::{self, AsyncCommands};
use listing_domain::repository::{
    ListingMarkerCount, ListingMarkerMask, ListingMarkerMaskEncoding, ListingMarkerMaskQuery,
    ListingMarkerRegisteredFilter, ListingMarkerTile, ListingMarkerTileQuery, ListingRepository,
    NormalizedListingMarkerFilterSpec, RepoError, LISTING_MARKER_TILE_LAYER,
};
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use ulid::Ulid;

const TILE_CACHE_MAGIC: &[u8; 4] = b"LMT1";
const RELEASE_LOCK_LUA: &str = r#"
if redis.call("GET", KEYS[1]) == ARGV[1] then
  return redis.call("DEL", KEYS[1])
end
return 0
"#;

#[derive(Clone)]
pub struct ListingMarkerServingGateway {
    listing_repo: Arc<dyn ListingRepository>,
    redis_pool: Option<Arc<deadpool_redis::Pool>>,
}

impl ListingMarkerServingGateway {
    #[must_use]
    pub fn new(
        listing_repo: Arc<dyn ListingRepository>,
        redis_pool: Option<Arc<deadpool_redis::Pool>>,
    ) -> Self {
        Self {
            listing_repo,
            redis_pool,
        }
    }

    pub async fn register_listing_marker_filter(
        &self,
        filter: NormalizedListingMarkerFilterSpec,
    ) -> Result<ListingMarkerRegisteredFilter, RepoError> {
        self.listing_repo
            .register_listing_marker_filter(filter)
            .await
    }

    pub async fn resolve_listing_marker_filter(
        &self,
        filter_hash: &str,
    ) -> Result<Option<NormalizedListingMarkerFilterSpec>, RepoError> {
        self.listing_repo
            .resolve_listing_marker_filter(filter_hash)
            .await
    }

    pub async fn find_listing_marker_tile(
        &self,
        filter_hash: &str,
        query: ListingMarkerTileQuery,
    ) -> Result<ListingMarkerTile, RepoError> {
        if self.redis_pool.is_none() {
            return self.load_listing_marker_tile(query).await;
        }

        let cache_key = format!(
            "listing-marker:tile:v1:{}:{}:{}:{}",
            query.z, query.x, query.y, filter_hash
        );
        if let Some(cached) = self.read_cache(&cache_key).await? {
            return decode_cached_tile(&cached);
        }

        let lock_key = format!("listing-marker:lock:{cache_key}");
        if let Some(token) = self.acquire_lock(&lock_key).await? {
            let loaded = self.load_listing_marker_tile(query).await;
            if let Ok(tile) = &loaded {
                self.write_cache(&cache_key, encode_cached_tile(tile))
                    .await?;
            }
            self.release_lock_best_effort(&lock_key, &token).await;
            return loaded;
        }

        self.wait_for_cached_tile(&cache_key).await
    }

    pub async fn count_listing_markers(
        &self,
        filter_hash: &str,
        filter: NormalizedListingMarkerFilterSpec,
    ) -> Result<ListingMarkerCount, RepoError> {
        if self.redis_pool.is_none() {
            return self.listing_repo.count_listing_markers(filter).await;
        }

        let cache_key = format!("listing-marker:count:v1:{filter_hash}");
        if let Some(cached) = self.read_cache(&cache_key).await? {
            return decode_count_cache(&cached);
        }

        let lock_key = format!("listing-marker:lock:{cache_key}");
        if let Some(token) = self.acquire_lock(&lock_key).await? {
            let loaded = self.listing_repo.count_listing_markers(filter).await;
            if let Ok(count) = &loaded {
                self.write_cache(&cache_key, encode_count_cache(count)?)
                    .await?;
            }
            self.release_lock_best_effort(&lock_key, &token).await;
            return loaded;
        }

        self.wait_for_count_cache(&cache_key).await
    }

    pub async fn find_listing_marker_mask(
        &self,
        filter_hash: &str,
        query: ListingMarkerMaskQuery,
    ) -> Result<ListingMarkerMask, RepoError> {
        if self.redis_pool.is_none() {
            return self.load_listing_marker_mask(query).await;
        }

        let base_version = query
            .base_version
            .map_or_else(|| "none".to_owned(), |version| version.to_string());
        let cache_key = format!(
            "listing-marker:mask:v1:{}:{}:{}:{}:{}",
            query.z, query.x, query.y, filter_hash, base_version
        );
        if let Some(cached) = self.read_cache(&cache_key).await? {
            return decode_mask_cache(&cached);
        }

        let lock_key = format!("listing-marker:lock:{cache_key}");
        if let Some(token) = self.acquire_lock(&lock_key).await? {
            let loaded = self.load_listing_marker_mask(query).await;
            if let Ok(mask) = &loaded {
                self.write_cache(&cache_key, encode_mask_cache(mask)?)
                    .await?;
            }
            self.release_lock_best_effort(&lock_key, &token).await;
            return loaded;
        }

        self.wait_for_mask_cache(&cache_key).await
    }

    async fn load_listing_marker_tile(
        &self,
        query: ListingMarkerTileQuery,
    ) -> Result<ListingMarkerTile, RepoError> {
        let tile = self.listing_repo.find_listing_marker_tile(query).await?;
        validate_tile_budget(&tile)?;
        Ok(tile)
    }

    async fn load_listing_marker_mask(
        &self,
        query: ListingMarkerMaskQuery,
    ) -> Result<ListingMarkerMask, RepoError> {
        let mask = self.listing_repo.find_listing_marker_mask(query).await?;
        validate_mask_budget(&mask)?;
        Ok(mask)
    }

    async fn wait_for_cached_tile(&self, cache_key: &str) -> Result<ListingMarkerTile, RepoError> {
        for _ in 0..LISTING_MARKER_SINGLE_FLIGHT_WAIT_ATTEMPTS {
            sleep(Duration::from_millis(LISTING_MARKER_SINGLE_FLIGHT_WAIT_MS)).await;
            if let Some(cached) = self.read_cache(cache_key).await? {
                return decode_cached_tile(&cached);
            }
        }
        Err(RepoError::Database(
            "listing marker tile single-flight wait timeout".to_owned(),
        ))
    }

    async fn wait_for_count_cache(&self, cache_key: &str) -> Result<ListingMarkerCount, RepoError> {
        for _ in 0..LISTING_MARKER_SINGLE_FLIGHT_WAIT_ATTEMPTS {
            sleep(Duration::from_millis(LISTING_MARKER_SINGLE_FLIGHT_WAIT_MS)).await;
            if let Some(cached) = self.read_cache(cache_key).await? {
                return decode_count_cache(&cached);
            }
        }
        Err(RepoError::Database(
            "listing marker count single-flight wait timeout".to_owned(),
        ))
    }

    async fn wait_for_mask_cache(&self, cache_key: &str) -> Result<ListingMarkerMask, RepoError> {
        for _ in 0..LISTING_MARKER_SINGLE_FLIGHT_WAIT_ATTEMPTS {
            sleep(Duration::from_millis(LISTING_MARKER_SINGLE_FLIGHT_WAIT_MS)).await;
            if let Some(cached) = self.read_cache(cache_key).await? {
                return decode_mask_cache(&cached);
            }
        }
        Err(RepoError::Database(
            "listing marker mask single-flight wait timeout".to_owned(),
        ))
    }

    async fn read_cache(&self, key: &str) -> Result<Option<Vec<u8>>, RepoError> {
        let Some(pool) = &self.redis_pool else {
            return Ok(None);
        };
        let mut conn = pool.get().await.map_err(|error| redis_pool_err(&error))?;
        conn.get(key).await.map_err(|error| redis_err(&error))
    }

    async fn write_cache(&self, key: &str, payload: Vec<u8>) -> Result<(), RepoError> {
        let Some(pool) = &self.redis_pool else {
            return Ok(());
        };
        let mut conn = pool.get().await.map_err(|error| redis_pool_err(&error))?;
        conn.set_ex(key, payload, LISTING_MARKER_CACHE_TTL_SECONDS)
            .await
            .map_err(|error| redis_err(&error))
    }

    async fn acquire_lock(&self, key: &str) -> Result<Option<String>, RepoError> {
        let Some(pool) = &self.redis_pool else {
            return Ok(None);
        };
        let mut conn = pool.get().await.map_err(|error| redis_pool_err(&error))?;
        let token = Ulid::new().to_string();
        let acquired: Option<String> = redis::cmd("SET")
            .arg(key)
            .arg(&token)
            .arg("NX")
            .arg("EX")
            .arg(LISTING_MARKER_SINGLE_FLIGHT_LOCK_SECONDS)
            .query_async(&mut conn)
            .await
            .map_err(|error| redis_err(&error))?;
        Ok(acquired.map(|_| token))
    }

    async fn release_lock_best_effort(&self, key: &str, token: &str) {
        let Some(pool) = &self.redis_pool else {
            return;
        };
        let Ok(mut conn) = pool.get().await else {
            tracing::warn!(
                key,
                "listing marker serving lock release skipped: redis pool error"
            );
            return;
        };
        let released: Result<i32, redis::RedisError> = redis::cmd("EVAL")
            .arg(RELEASE_LOCK_LUA)
            .arg(1)
            .arg(key)
            .arg(token)
            .query_async(&mut conn)
            .await;
        if let Err(error) = released {
            tracing::warn!(key, error = %error, "listing marker serving lock release failed");
        }
    }
}

fn validate_tile_budget(tile: &ListingMarkerTile) -> Result<(), RepoError> {
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

fn validate_mask_budget(mask: &ListingMarkerMask) -> Result<(), RepoError> {
    if mask.marker_ids.len() > MAX_LISTING_MARKER_MASK_IDS {
        return Err(RepoError::Database(format!(
            "listing marker mask budget violation: marker_ids={} max={}",
            mask.marker_ids.len(),
            MAX_LISTING_MARKER_MASK_IDS
        )));
    }
    Ok(())
}

fn encode_cached_tile(tile: &ListingMarkerTile) -> Vec<u8> {
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

fn decode_cached_tile(payload: &[u8]) -> Result<ListingMarkerTile, RepoError> {
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

fn encode_count_cache(count: &ListingMarkerCount) -> Result<Vec<u8>, RepoError> {
    encode_json_cache(&CachedListingMarkerCount {
        total_count: count.total_count,
        projection_version: count.projection_version,
        anchor_snapshot_id: count.anchor_snapshot_id.clone(),
    })
}

fn decode_count_cache(payload: &[u8]) -> Result<ListingMarkerCount, RepoError> {
    let cached: CachedListingMarkerCount = decode_json_cache(payload)?;
    Ok(ListingMarkerCount {
        total_count: cached.total_count,
        projection_version: cached.projection_version,
        anchor_snapshot_id: cached.anchor_snapshot_id,
    })
}

fn encode_mask_cache(mask: &ListingMarkerMask) -> Result<Vec<u8>, RepoError> {
    encode_json_cache(&CachedListingMarkerMask {
        encoding: mask.encoding.as_str().to_owned(),
        marker_ids: mask.marker_ids.clone(),
        projection_version: mask.projection_version,
        anchor_snapshot_id: mask.anchor_snapshot_id.clone(),
    })
}

fn decode_mask_cache(payload: &[u8]) -> Result<ListingMarkerMask, RepoError> {
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

fn redis_pool_err(error: &deadpool_redis::PoolError) -> RepoError {
    RepoError::Database(format!("listing marker redis pool error: {error}"))
}

fn redis_err(error: &redis::RedisError) -> RepoError {
    RepoError::Database(format!("listing marker redis error: {error}"))
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
    fn cached_tile_codec_round_trips_metadata_and_bytes() -> Result<(), RepoError> {
        let tile = sample_tile(vec![1, 2, 3, 4, 5]);

        let encoded = encode_cached_tile(&tile);
        let decoded = decode_cached_tile(&encoded)?;

        assert_eq!(decoded, tile);
        Ok(())
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
