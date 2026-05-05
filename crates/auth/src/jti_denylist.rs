//! `JTI` 무효화 목록 (logout / refresh rotation / role change 시 token 즉시 무효).

use async_trait::async_trait;
use deadpool_redis::{redis::AsyncCommands, Pool};

/// `JWT` `JTI` denylist 트레잇.
#[async_trait]
pub trait JtiDenylist: Send + Sync {
    /// 해당 jti 가 무효인지 (denylist hit).
    ///
    /// # Errors
    ///
    /// Redis 연결/명령 실패 시 [`JtiError::Redis`].
    async fn is_denied(&self, jti: &str) -> Result<bool, JtiError>;

    /// jti 를 ttl 초 동안 무효화.
    ///
    /// # Errors
    ///
    /// Redis 연결/명령 실패 시 [`JtiError::Redis`].
    async fn deny(&self, jti: &str, ttl_sec: u64) -> Result<(), JtiError>;
}

/// `JTI` denylist 작업 중 발생할 수 있는 오류.
#[derive(Debug, thiserror::Error)]
pub enum JtiError {
    /// Redis 연결 실패 또는 명령 오류.
    #[error("redis: {0}")]
    Redis(String),
}

impl From<deadpool_redis::PoolError> for JtiError {
    fn from(e: deadpool_redis::PoolError) -> Self {
        Self::Redis(e.to_string())
    }
}

impl From<deadpool_redis::redis::RedisError> for JtiError {
    fn from(e: deadpool_redis::redis::RedisError) -> Self {
        Self::Redis(e.to_string())
    }
}

/// Redis 기반 `JTI` denylist 구현.
pub struct RedisJtiDenylist {
    pool: Pool,
}

impl RedisJtiDenylist {
    /// `Pool` 로 새 인스턴스 생성.
    #[must_use]
    pub const fn new(pool: Pool) -> Self {
        Self { pool }
    }

    fn key(jti: &str) -> String {
        format!("jti:deny:{jti}")
    }
}

#[async_trait]
impl JtiDenylist for RedisJtiDenylist {
    async fn is_denied(&self, jti: &str) -> Result<bool, JtiError> {
        let mut conn = self.pool.get().await?;
        let exists: bool = conn.exists(Self::key(jti)).await?;
        Ok(exists)
    }

    async fn deny(&self, jti: &str, ttl_sec: u64) -> Result<(), JtiError> {
        let mut conn = self.pool.get().await?;
        let _: () = conn.set_ex(Self::key(jti), "1", ttl_sec).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;
    use deadpool_redis::{Config, Runtime};

    fn pool() -> Option<Pool> {
        let url = std::env::var("REDIS_URL").ok()?;
        let cfg = Config::from_url(url);
        cfg.create_pool(Some(Runtime::Tokio1)).ok()
    }

    fn unique_suffix() -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        format!("{}-{nanos}", std::process::id())
    }

    #[tokio::test]
    async fn deny_then_is_denied_true() {
        let Some(p) = pool() else {
            // REDIS_URL 미설정 시 skip — CI Redis service container 필요.
            return;
        };
        let dl = RedisJtiDenylist::new(p);
        let jti = format!("test-{}", unique_suffix());
        assert!(!dl.is_denied(&jti).await.expect("query"));
        dl.deny(&jti, 60).await.expect("deny");
        assert!(dl.is_denied(&jti).await.expect("query"));
    }

    #[tokio::test]
    async fn unknown_jti_not_denied() {
        let Some(p) = pool() else {
            // REDIS_URL 미설정 시 skip — CI Redis service container 필요.
            return;
        };
        let dl = RedisJtiDenylist::new(p);
        let jti = format!("nonexistent-{}", unique_suffix());
        assert!(!dl.is_denied(&jti).await.expect("query"));
    }
}
