//! `Policy` — 외부 API별 호출 정책 (timeout / retry / open threshold).
//!
//! 각 API 표준 정책은 명명된 상수로 표현 — `Policy::vworld_default()` 등.
//! 호출 측이 매번 `Policy::new(...)` 로 새 값 만들지 않게 강제 → 일관성 보장.

/// 외부 API 호출 정책 — timeout, retry, circuit breaker threshold.
///
/// `Copy` 가능 — 모든 필드가 primitive. 함수 인자로 부담 없이 전달.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Policy {
    /// 단일 호출 timeout (밀리초).
    pub timeout_ms: u64,
    /// 재시도 횟수 (총 시도 = `max_retries + 1`).
    pub max_retries: u32,
    /// 첫 retry 까지 base delay (밀리초). 지수 백오프 = `retry_base_ms * 2^attempt`.
    pub retry_base_ms: u64,
    /// `open_window_ms` 안에 N 회 실패하면 circuit `Open` 으로 전이.
    pub open_threshold: u32,
    /// failure window 길이 (밀리초).
    pub open_window_ms: u64,
    /// `Open` → `HalfOpen` 까지 cooldown (밀리초).
    pub open_cooldown_ms: u64,
}

impl Policy {
    /// V-World 표준 정책 — `docs/data-sources/v-world.md` § Circuit Breaker 정책.
    ///
    /// - timeout 10초 / 재시도 1회 (1s, 2s 지수 백오프)
    /// - 5초 안 5회 실패 → 30초 차단
    #[must_use]
    pub const fn vworld_default() -> Self {
        Self {
            timeout_ms: 10_000,
            max_retries: 1,
            retry_base_ms: 1_000,
            open_threshold: 5,
            open_window_ms: 5_000,
            open_cooldown_ms: 30_000,
        }
    }

    /// data.go.kr 표준 정책 — `docs/data-sources/data-go-kr.md` § Circuit Breaker 정책.
    ///
    /// - timeout 15초 / 재시도 2회 (1s, 2s, 4s 지수 백오프)
    /// - 5초 안 5회 실패 → 30초 차단
    ///
    /// V-World 보다 timeout 길고 retry 더 — 응답 본문 (건축물대장 등) 이 무거움.
    #[must_use]
    pub const fn data_go_kr_default() -> Self {
        Self {
            timeout_ms: 15_000,
            max_retries: 2,
            retry_base_ms: 1_000,
            open_threshold: 5,
            open_window_ms: 5_000,
            open_cooldown_ms: 30_000,
        }
    }

    /// R2 (Cloudflare R2, S3-호환) 정적 객체 정책 — SP4-iii-e.
    ///
    /// - timeout 8초 / 재시도 1회 (1s 지수 백오프)
    /// - 10초 안 5회 실패 → 60초 차단
    ///
    /// 정부 API 보다 short timeout 적정 — R2 가 정적 객체 (`PMTiles` / JSON 인덱스)
    /// 라 latency variance 작음. cooldown 길게 — 정적 객체 outage 는 보통
    /// region-level (CDN 자동 복구 대기).
    #[must_use]
    pub const fn r2_default() -> Self {
        Self {
            timeout_ms: 8_000,
            max_retries: 1,
            retry_base_ms: 1_000,
            open_threshold: 5,
            open_window_ms: 10_000,
            open_cooldown_ms: 60_000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vworld_default_matches_doc() {
        let p = Policy::vworld_default();
        assert_eq!(p.timeout_ms, 10_000);
        assert_eq!(p.max_retries, 1);
        assert_eq!(p.retry_base_ms, 1_000);
        assert_eq!(p.open_threshold, 5);
        assert_eq!(p.open_window_ms, 5_000);
        assert_eq!(p.open_cooldown_ms, 30_000);
    }

    #[test]
    fn data_go_kr_default_matches_doc() {
        let p = Policy::data_go_kr_default();
        assert_eq!(p.timeout_ms, 15_000);
        assert_eq!(p.max_retries, 2);
        assert_eq!(p.retry_base_ms, 1_000);
        assert_eq!(p.open_threshold, 5);
        assert_eq!(p.open_window_ms, 5_000);
        assert_eq!(p.open_cooldown_ms, 30_000);
    }

    #[test]
    fn data_go_kr_has_longer_timeout_than_vworld() {
        // 건축물대장 응답 본문이 V-World 보다 무거움 — timeout 길게.
        let v = Policy::vworld_default();
        let d = Policy::data_go_kr_default();
        assert!(d.timeout_ms > v.timeout_ms);
        assert!(d.max_retries > v.max_retries);
    }

    #[test]
    fn r2_default_matches_doc() {
        let p = Policy::r2_default();
        assert_eq!(p.timeout_ms, 8_000);
        assert_eq!(p.max_retries, 1);
        assert_eq!(p.retry_base_ms, 1_000);
        assert_eq!(p.open_threshold, 5);
        assert_eq!(p.open_window_ms, 10_000);
        assert_eq!(p.open_cooldown_ms, 60_000);
    }

    #[test]
    fn r2_has_shorter_timeout_than_data_go_kr() {
        // R2 는 정적 객체 — government API 보다 latency variance 작음.
        let r = Policy::r2_default();
        let d = Policy::data_go_kr_default();
        assert!(r.timeout_ms < d.timeout_ms);
        assert!(r.open_cooldown_ms > d.open_cooldown_ms);
    }

    #[test]
    fn policy_is_copy() {
        // 컴파일러가 `Copy` 를 강제 — 본 함수 인자 통과 자체가 trait bound 검증.
        const fn assert_copy<T: Copy>(_: &T) {}
        let p = Policy::vworld_default();
        assert_copy(&p);
        // p 가 move 안 됐는지 확인.
        assert_eq!(p.timeout_ms, 10_000);
    }
}
