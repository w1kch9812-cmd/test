//! 시각 헬퍼 — `UTC` 저장 / `KST` 표시 분리.
//!
//! 도메인 내부 표준 시각은 `UTC`. 사용자에게 노출할 때만 `to_kst`로 변환해요.

use chrono::{DateTime, FixedOffset, Utc};

/// 현재 `UTC` 시각.
#[must_use]
pub fn now_utc() -> DateTime<Utc> {
    Utc::now()
}

/// `UTC` → `KST`(+09:00) 변환. 사용자 노출 전용.
#[must_use]
#[allow(clippy::expect_used)] // 9*3600 = 32400 is a const within FixedOffset's ±86400 range; expect is provably infallible
pub fn to_kst(t: DateTime<Utc>) -> DateTime<FixedOffset> {
    let kst = FixedOffset::east_opt(9 * 3600).expect("9*3600 is a valid FixedOffset");
    t.with_timezone(&kst)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;
    use chrono::{TimeZone, Timelike};

    #[test]
    fn now_utc_is_close_to_chrono_now() {
        let our = now_utc();
        let theirs = Utc::now();
        let diff_secs = (our - theirs).num_seconds().abs();
        assert!(diff_secs < 2, "now_utc deviated by {diff_secs}s from Utc::now()");
    }

    #[test]
    fn to_kst_converts_offset() {
        let utc = Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).expect("valid");
        let kst = to_kst(utc);
        assert_eq!(kst.hour(), 9);
        assert_eq!(kst.offset().local_minus_utc(), 9 * 3600);
    }

    #[test]
    fn to_kst_preserves_instant() {
        // The instant in UTC seconds since epoch must be unchanged after timezone conversion.
        let utc = Utc.with_ymd_and_hms(2026, 5, 1, 12, 30, 45).expect("valid");
        let kst = to_kst(utc);
        assert_eq!(utc.timestamp(), kst.timestamp());
    }

    #[test]
    fn to_kst_rolls_date_forward_at_midnight_utc() {
        // 2026-05-01 00:00 UTC → 2026-05-01 09:00 KST (same date)
        let utc = Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).expect("valid");
        let kst = to_kst(utc);
        assert_eq!(kst.format("%Y-%m-%d").to_string(), "2026-05-01");
    }

    #[test]
    fn to_kst_rolls_date_forward_at_late_utc() {
        // 2026-05-01 16:00 UTC → 2026-05-02 01:00 KST (date rolled)
        let utc = Utc.with_ymd_and_hms(2026, 5, 1, 16, 0, 0).expect("valid");
        let kst = to_kst(utc);
        assert_eq!(kst.format("%Y-%m-%d").to_string(), "2026-05-02");
        assert_eq!(kst.hour(), 1);
    }
}
