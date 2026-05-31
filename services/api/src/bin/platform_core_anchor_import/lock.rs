use sha2::{Digest, Sha256};
use sqlx::{pool::PoolConnection, PgPool, Postgres};

use super::error::AnchorImporterError;

pub struct EventImportLock {
    event_id: String,
    key: i64,
    connection: PoolConnection<Postgres>,
}

impl EventImportLock {
    pub async fn release(mut self) -> Result<(), AnchorImporterError> {
        let released: bool = sqlx::query_scalar("select pg_advisory_unlock($1)")
            .bind(self.key)
            .fetch_one(&mut *self.connection)
            .await?;
        if released {
            return Ok(());
        }

        Err(AnchorImporterError::EventImportLockReleaseFailed {
            event_id: self.event_id,
        })
    }
}

pub async fn acquire_optional_event_import_lock(
    pool: &PgPool,
    event_id: Option<&str>,
) -> Result<Option<EventImportLock>, AnchorImporterError> {
    let Some(event_id) = event_id else {
        return Ok(None);
    };

    let key = event_import_lock_key(event_id);
    let mut connection = pool.acquire().await?;
    let acquired: bool = sqlx::query_scalar("select pg_try_advisory_lock($1)")
        .bind(key)
        .fetch_one(&mut *connection)
        .await?;
    if !acquired {
        return Err(AnchorImporterError::InboxEventAlreadyLocked {
            event_id: event_id.to_owned(),
        });
    }

    Ok(Some(EventImportLock {
        event_id: event_id.to_owned(),
        key,
        connection,
    }))
}

pub fn event_import_lock_key(event_id: &str) -> i64 {
    let digest = Sha256::digest(event_id.as_bytes());
    i64::from_be_bytes([
        digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6], digest[7],
    ])
}
