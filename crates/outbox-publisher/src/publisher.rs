//! `tick` — outbox publisher 의 한 사이클 단위.
//!
//! 1 tick = `fetch_unpublished` → 각 이벤트마다 `sink.publish` → 성공 시
//! `mark_published`. 어느 단계든 실패는 batch 격리 — row 가 미발행 상태로 남아
//! 다음 tick 에 재시도.

// `PublisherError` 등 의도된 공개 API 형태.
#![allow(clippy::module_name_repetitions)]

use chrono::Utc;
use outbox_event_domain::repository::{OutboxRepository, RepoError};
use thiserror::Error;
use tracing::{instrument, warn};

use crate::sink::Sink;

/// publisher 호출 실패.
///
/// `tick` 자체가 실패하는 경우는 `fetch_unpublished` 가 실패할 때예요. 개별 event
/// 의 sink 실패는 `tick` 의 `Ok(report)` 안에서 `failed` 카운터로 보고.
#[derive(Debug, Error)]
pub enum PublisherError {
    /// 저장소 호출 실패 — `fetch_unpublished` 또는 catastrophic mark 실패.
    #[error("repository error: {0}")]
    Repo(#[from] RepoError),
}

/// 한 tick 의 결과 — 메트릭 / 테스트 검증용.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TickReport {
    /// `fetch_unpublished` 가 가져온 이벤트 수.
    pub fetched: u32,
    /// `sink.publish` + `mark_published` 모두 성공한 이벤트 수.
    pub published: u32,
    /// `sink.publish` 또는 `mark_published` 가 실패한 이벤트 수 (재시도 예정).
    pub failed: u32,
}

/// outbox publisher tick — 1 사이클.
///
/// `fetch_unpublished(limit)` 으로 가져온 이벤트들을 sequential 하게 sink 로 발행 +
/// 발행 성공 시 `mark_published`. 부분 실패는 `report.failed` 로만 누적 (다음 tick
/// 에 재시도).
///
/// # Errors
///
/// `fetch_unpublished` 실패 시 [`PublisherError::Repo`] — 이 경우 batch 전체
/// skip. 개별 event 의 sink/mark 실패는 `Ok(report)` 안에서 `failed` 로 보고.
#[instrument(skip(repo, sink), fields(limit))]
pub async fn tick(
    repo: &dyn OutboxRepository,
    sink: &dyn Sink,
    limit: u32,
) -> Result<TickReport, PublisherError> {
    let events = repo.fetch_unpublished(limit).await?;
    let fetched = u32::try_from(events.len()).unwrap_or(u32::MAX);
    let mut report = TickReport {
        fetched,
        published: 0,
        failed: 0,
    };

    for event in events {
        match sink.publish(&event).await {
            Ok(()) => match repo.mark_published(&event.id, Utc::now()).await {
                Ok(()) => report.published += 1,
                Err(e) => {
                    warn!(
                        event_id = %event.id.as_str(),
                        error = %e,
                        "mark_published failed — will retry next tick"
                    );
                    report.failed += 1;
                }
            },
            Err(e) => {
                warn!(
                    event_id = %event.id.as_str(),
                    error = %e,
                    "sink publish failed — will retry next tick"
                );
                report.failed += 1;
            }
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

    use super::*;

    #[test]
    fn tick_report_default_is_zero() {
        let r = TickReport::default();
        assert_eq!(r.fetched, 0);
        assert_eq!(r.published, 0);
        assert_eq!(r.failed, 0);
    }

    #[test]
    fn publisher_error_display_format() {
        let e = PublisherError::Repo(RepoError::NotFound);
        assert_eq!(e.to_string(), "repository error: not found");
    }

    #[test]
    fn publisher_error_from_repo_error() {
        let e: PublisherError = RepoError::Database("oops".to_owned()).into();
        match e {
            PublisherError::Repo(RepoError::Database(s)) => assert_eq!(s, "oops"),
            _ => panic!("expected Repo Database variant"),
        }
    }
}
