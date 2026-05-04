//! `execute` — circuit breaker + retry + timeout 을 래핑한 단일 진입점.
//!
//! 모든 외부 API 호출이 이 함수를 통과:
//!
//! 1. `Breaker::check()` — `Open` 이면 즉시 `BreakerError::Open` 반환
//! 2. attempt 0 부터 `max_retries` 까지 반복:
//!    - `tokio::time::timeout(timeout_ms, op())`
//!    - 성공 → `record_success` + `Ok(value)`
//!    - timeout 또는 inner err → `record_failure` + 다음 retry (지수 백오프)
//! 3. 모든 retry 실패 → `BreakerError::MaxRetriesExceeded`

#![allow(clippy::module_name_repetitions)]

use std::fmt::Display;
use std::future::Future;
use std::time::Duration;

use thiserror::Error;
use tokio::time::{sleep, timeout};
use tracing::{debug, instrument, warn};

use crate::breaker::{Breaker, CircuitState};
use crate::policy::Policy;

/// `execute` 의 모든 실패 모드.
///
/// `E` 는 호출 작업 (`op`) 의 inner error 타입 — `Display` 만 요구
/// (where 절로 표기 — `clippy::trait_duplication_in_bounds` 회피).
#[derive(Debug, Error)]
pub enum BreakerError<E>
where
    E: Display,
{
    /// circuit breaker 가 `Open` 상태 — 호출 즉시 차단됨.
    #[error("circuit open — too many recent failures")]
    Open,
    /// 단일 시도가 timeout 초과.
    #[error("operation timed out after {timeout_ms}ms")]
    Timeout {
        /// 적용된 timeout.
        timeout_ms: u64,
    },
    /// 모든 retry 가 실패 — 마지막 inner error 메시지 보존.
    #[error("max retries exceeded ({max_retries}): last error: {last}")]
    MaxRetriesExceeded {
        /// 정책 상 retry 최대.
        max_retries: u32,
        /// 마지막 시도의 inner error 메시지.
        last: String,
    },
    /// inner error — `op()` 가 반환한 에러.
    ///
    /// 1회 호출 + retry 0 인 경우 또는 첫 호출 직후 transient 가 아닌 명시적
    /// 에러를 그대로 노출하고 싶을 때 사용. 현재 `execute` 는 모든 실패를
    /// retry 하므로 이 variant 가 직접 반환되진 않지만, future API 변경 여지로
    /// 보존.
    #[error("inner error: {0}")]
    Inner(E),
}

/// circuit breaker + retry + timeout 으로 보호된 호출.
///
/// `op` 는 `FnMut` 이 아닌 `Fn` — 매 retry 마다 새 future 생성 (re-execute).
/// HTTP 요청 등 idempotent 한 작업에 안전.
///
/// # Errors
///
/// - `BreakerError::Open`: circuit 가 차단된 상태에서 호출됨
/// - `BreakerError::Timeout`: 단일 호출 timeout (모든 retry 시도 후)
/// - `BreakerError::MaxRetriesExceeded`: 모든 retry 실패
#[instrument(skip(breaker, policy, op), fields(op_name = %op_name))]
pub async fn execute<F, Fut, T, E>(
    breaker: &Breaker,
    policy: &Policy,
    op_name: &'static str,
    op: F,
) -> Result<T, BreakerError<E>>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: Display,
{
    // 1. circuit 검사 — Open 이면 즉시 거부.
    let state = breaker.check(policy);
    if state == CircuitState::Open {
        warn!(op_name, "circuit open — refusing call");
        return Err(BreakerError::Open);
    }

    let timeout_dur = Duration::from_millis(policy.timeout_ms);
    let mut last_error_msg = String::new();

    // 2. attempt loop: 0..=max_retries (총 시도 = max_retries + 1)
    for attempt in 0..=policy.max_retries {
        if attempt > 0 {
            // 지수 백오프: retry_base_ms * 2^(attempt-1)
            let backoff_ms = policy
                .retry_base_ms
                .saturating_mul(1_u64 << (attempt - 1).min(10));
            debug!(op_name, attempt, backoff_ms, "retrying after backoff");
            sleep(Duration::from_millis(backoff_ms)).await;
        }

        match timeout(timeout_dur, op()).await {
            Ok(Ok(value)) => {
                breaker.record_success();
                return Ok(value);
            }
            Ok(Err(inner)) => {
                last_error_msg = inner.to_string();
                breaker.record_failure(policy);
                warn!(
                    op_name,
                    attempt,
                    error = %last_error_msg,
                    "inner error — recording failure"
                );
            }
            Err(_) => {
                last_error_msg = format!("timeout after {}ms", policy.timeout_ms);
                breaker.record_failure(policy);
                warn!(op_name, attempt, "timeout — recording failure");
                if attempt == policy.max_retries {
                    return Err(BreakerError::Timeout {
                        timeout_ms: policy.timeout_ms,
                    });
                }
            }
        }
    }

    Err(BreakerError::MaxRetriesExceeded {
        max_retries: policy.max_retries,
        last: last_error_msg,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    use super::*;
    use crate::breaker::CircuitState;

    fn test_policy() -> Policy {
        Policy {
            timeout_ms: 50,
            max_retries: 1,
            retry_base_ms: 1, // 1ms backoff — 테스트 빠름
            open_threshold: 3,
            open_window_ms: 1_000,
            open_cooldown_ms: 5_000,
        }
    }

    #[tokio::test]
    async fn execute_returns_inner_ok_immediately() {
        let b = Breaker::new();
        let p = test_policy();
        let result: Result<u32, BreakerError<&'static str>> =
            execute(&b, &p, "test", || async { Ok(42_u32) }).await;
        assert_eq!(result.expect("ok"), 42);
        assert_eq!(b.state(), CircuitState::Closed);
    }

    #[tokio::test]
    async fn execute_retries_on_inner_err_then_succeeds() {
        let b = Breaker::new();
        let p = test_policy();
        let counter = Arc::new(AtomicU32::new(0));
        let counter2 = counter.clone();
        let result: Result<u32, BreakerError<&'static str>> = execute(&b, &p, "test", move || {
            let c = counter2.clone();
            async move {
                let n = c.fetch_add(1, Ordering::SeqCst);
                if n == 0 {
                    Err("transient")
                } else {
                    Ok(7_u32)
                }
            }
        })
        .await;
        assert_eq!(result.expect("ok after retry"), 7);
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn execute_returns_max_retries_exceeded_after_all_fails() {
        let b = Breaker::new();
        let p = test_policy();
        let result: Result<u32, BreakerError<&'static str>> =
            execute(&b, &p, "test", || async { Err("always fails") }).await;
        match result.unwrap_err() {
            BreakerError::MaxRetriesExceeded { max_retries, last } => {
                assert_eq!(max_retries, 1);
                assert!(last.contains("always fails"));
            }
            other => panic!("expected MaxRetriesExceeded, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn execute_timeout_records_failure() {
        let b = Breaker::new();
        let p = Policy {
            timeout_ms: 10,
            max_retries: 0, // 1 시도만
            ..test_policy()
        };
        let result: Result<u32, BreakerError<&'static str>> = execute(&b, &p, "test", || async {
            tokio::time::sleep(Duration::from_millis(100)).await;
            Ok(0_u32)
        })
        .await;
        match result.unwrap_err() {
            BreakerError::Timeout { timeout_ms } => assert_eq!(timeout_ms, 10),
            other => panic!("expected Timeout, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn execute_returns_open_when_breaker_open() {
        let b = Breaker::new();
        let p = test_policy();
        // Open 으로 강제 전이 (3회 실패)
        for _ in 0..3 {
            b.record_failure(&p);
        }
        assert_eq!(b.state(), CircuitState::Open);
        let result: Result<u32, BreakerError<&'static str>> =
            execute(&b, &p, "test", || async { Ok(42_u32) }).await;
        assert!(matches!(result, Err(BreakerError::Open)));
    }

    #[tokio::test]
    async fn execute_threshold_failures_trigger_open() {
        let b = Breaker::new();
        let p = Policy {
            // max_retries 0 + open_threshold 3 — 1번 호출당 1번 실패 기록
            max_retries: 0,
            open_threshold: 3,
            ..test_policy()
        };
        for _ in 0..3 {
            let _: Result<u32, BreakerError<&'static str>> =
                execute(&b, &p, "test", || async { Err("fail") }).await;
        }
        assert_eq!(b.state(), CircuitState::Open);
    }

    #[tokio::test]
    async fn execute_success_after_failures_clears_state() {
        let b = Breaker::new();
        let p = Policy {
            max_retries: 0,
            open_threshold: 3,
            ..test_policy()
        };
        // 2번 실패 (threshold 미만)
        for _ in 0..2 {
            let _: Result<u32, BreakerError<&'static str>> =
                execute(&b, &p, "test", || async { Err("fail") }).await;
        }
        // 1번 성공 — 누적 실패 비워짐
        let _: Result<u32, BreakerError<&'static str>> =
            execute(&b, &p, "test", || async { Ok(1_u32) }).await;
        assert_eq!(b.state(), CircuitState::Closed);

        // 다시 2번 실패해도 Closed 유지 (window 비워졌으므로)
        for _ in 0..2 {
            let _: Result<u32, BreakerError<&'static str>> =
                execute(&b, &p, "test", || async { Err("fail") }).await;
        }
        assert_eq!(b.state(), CircuitState::Closed);
    }

    #[test]
    fn breaker_error_display_open() {
        let e: BreakerError<&'static str> = BreakerError::Open;
        assert_eq!(e.to_string(), "circuit open — too many recent failures");
    }

    #[test]
    fn breaker_error_display_timeout() {
        let e: BreakerError<&'static str> = BreakerError::Timeout { timeout_ms: 5_000 };
        assert_eq!(e.to_string(), "operation timed out after 5000ms");
    }

    #[test]
    fn breaker_error_display_max_retries() {
        let e: BreakerError<&'static str> = BreakerError::MaxRetriesExceeded {
            max_retries: 3,
            last: "boom".into(),
        };
        assert_eq!(e.to_string(), "max retries exceeded (3): last error: boom");
    }

    #[test]
    fn breaker_error_display_inner() {
        let e: BreakerError<&'static str> = BreakerError::Inner("wrapped");
        assert_eq!(e.to_string(), "inner error: wrapped");
    }
}
