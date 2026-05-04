//! `PNU` 19자리 → data.go.kr 건축물대장 5 분해 파라미터.
//!
//! data.go.kr `getBrTitleInfo` 가 받는 query parameter 5 개:
//! - `sigunguCd` (5) — `PNU[0..5]` (시도 2 + 시군구 3)
//! - `bjdongCd`  (5) — `PNU[5..10]` (법정동 5)
//! - `platGbCd`  (1) — `PNU[10..11]` (`0` 일반 / `1` 산 / `2` 블록)
//! - `bun`       (4) — `PNU[11..15]` (본번)
//! - `ji`        (4) — `PNU[15..19]` (부번)
//!
//! `Pnu::try_new` 가 19자리 ASCII 숫자를 강제 → 본 분해는 무조건 성공.

#![allow(clippy::module_name_repetitions)]

use shared_kernel::pnu::Pnu;

/// `PNU` 분해 결과 — 모두 PNU 내부 슬라이스 (allocation 없음).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PnuParts<'a> {
    /// 시군구 코드 5자리 (`sigunguCd`).
    pub sigungu_cd: &'a str,
    /// 법정동 코드 5자리 (`bjdongCd`).
    pub bjdong_cd: &'a str,
    /// 대지 구분 1자리 (`platGbCd`). `0` 일반 / `1` 산 / `2` 블록.
    pub plat_gb_cd: &'a str,
    /// 본번 4자리 (`bun`).
    pub bun: &'a str,
    /// 부번 4자리 (`ji`).
    pub ji: &'a str,
}

/// `PNU` 19자리 → 5 분해 파라미터.
///
/// `Pnu` 가 19자리 invariant 보장 — 무조건 성공 (slicing panic 없음).
#[must_use]
pub fn split(pnu: &Pnu) -> PnuParts<'_> {
    let s = pnu.as_str();
    PnuParts {
        sigungu_cd: &s[0..5],
        bjdong_cd: &s[5..10],
        plat_gb_cd: &s[10..11],
        bun: &s[11..15],
        ji: &s[15..19],
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn split_seoul_jongno_matches_doc() {
        // PNU = "1111010100100010000" (서울특별시 종로구 청운효자동, 본번1, 부번0, 일반)
        let pnu = Pnu::try_new("1111010100100010000").expect("valid");
        let p = split(&pnu);
        assert_eq!(p.sigungu_cd, "11110");
        assert_eq!(p.bjdong_cd, "10100");
        assert_eq!(p.plat_gb_cd, "1");
        assert_eq!(p.bun, "0001");
        assert_eq!(p.ji, "0000");
    }

    #[test]
    fn split_concatenation_round_trips() {
        // 분해 후 재조합 → 원본 PNU.
        let pnu = Pnu::try_new("4111010100100010000").expect("valid");
        let p = split(&pnu);
        let recombined = format!(
            "{}{}{}{}{}",
            p.sigungu_cd, p.bjdong_cd, p.plat_gb_cd, p.bun, p.ji
        );
        assert_eq!(recombined, pnu.as_str());
    }

    #[test]
    fn split_field_widths_are_5_5_1_4_4() {
        let pnu = Pnu::try_new("4111010100200090099").expect("valid");
        let p = split(&pnu);
        assert_eq!(p.sigungu_cd.len(), 5);
        assert_eq!(p.bjdong_cd.len(), 5);
        assert_eq!(p.plat_gb_cd.len(), 1);
        assert_eq!(p.bun.len(), 4);
        assert_eq!(p.ji.len(), 4);
    }

    #[test]
    fn split_san_parcel_plat_gb_cd_is_2() {
        // char 10 = '2' → 산 (mountain) 필지.
        let pnu = Pnu::try_new("1111010100200010000").expect("valid");
        let p = split(&pnu);
        assert_eq!(p.plat_gb_cd, "2");
    }
}
