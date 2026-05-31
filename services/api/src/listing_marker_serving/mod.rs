use std::sync::Arc;
use std::time::Duration;

use crate::listing_marker_policy::{
    LISTING_MARKER_CACHE_TTL_SECONDS, LISTING_MARKER_SINGLE_FLIGHT_LOCK_SECONDS,
    LISTING_MARKER_SINGLE_FLIGHT_WAIT_ATTEMPTS, LISTING_MARKER_SINGLE_FLIGHT_WAIT_MS,
};
use deadpool_redis::redis::{self, AsyncCommands};
use listing_domain::repository::{
    ListingMarkerCount, ListingMarkerDeltas, ListingMarkerMask, ListingMarkerMaskQuery,
    ListingMarkerOverlayTileQuery, ListingMarkerRegisteredFilter, ListingMarkerTile,
    ListingMarkerTileQuery, ListingMarkerTombstones, ListingRepository,
    NormalizedListingMarkerFilterSpec, RepoError,
};
use tokio::time::sleep;
use ulid::Ulid;

use self::budget::{
    validate_delta_budget, validate_mask_budget, validate_tile_budget, validate_tombstone_budget,
};
use self::cache::{
    decode_cached_tile, decode_count_cache, decode_mask_cache, encode_cached_tile,
    encode_count_cache, encode_mask_cache,
};

mod budget;
mod cache;

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

    pub async fn find_listing_marker_tombstones(
        &self,
        query: ListingMarkerOverlayTileQuery,
    ) -> Result<ListingMarkerTombstones, RepoError> {
        let tombstones = self
            .listing_repo
            .find_listing_marker_tombstones(query)
            .await?;
        validate_tombstone_budget(&tombstones)?;
        Ok(tombstones)
    }

    pub async fn find_listing_marker_deltas(
        &self,
        query: ListingMarkerOverlayTileQuery,
    ) -> Result<ListingMarkerDeltas, RepoError> {
        let deltas = self.listing_repo.find_listing_marker_deltas(query).await?;
        validate_delta_budget(&deltas)?;
        Ok(deltas)
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

fn redis_pool_err(error: &deadpool_redis::PoolError) -> RepoError {
    RepoError::Database(format!("listing marker redis pool error: {error}"))
}

fn redis_err(error: &redis::RedisError) -> RepoError {
    RepoError::Database(format!("listing marker redis error: {error}"))
}
