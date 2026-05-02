//! `Manufacturer` Aggregate (`R2` 정적, 9 필드).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared_kernel::business_number::BusinessNumber;
use shared_kernel::ksic_code::KsicCode;
use shared_kernel::pnu::Pnu;

use crate::employee_count_band::EmployeeCountBand;

/// `Manufacturer` Aggregate. `R2` 정적 — *read-only*, mutation 메서드 없음.
///
/// 한국 제조업체 (산단 입주기업 포함). 식별은 `BusinessNumber` (10자리).
/// `R2` 정적이므로 다른 BC 와 cross-BC `FK` 관계 *없음* —
/// `industrial_complex_code` / `pnu` 는 *단순 참조 문자열/`Pnu`*.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manufacturer {
    /// 사업자등록번호 (`PK`, `FK` 아님 — `R2` 정적).
    pub business_number: BusinessNumber,
    /// 회사명.
    pub company_name: String,
    /// 입주 산단 코드 (있으면, 단순 참조 — `FK` 아님).
    pub industrial_complex_code: Option<String>,
    /// 위치 필지 (있으면).
    pub pnu: Option<Pnu>,
    /// 한국 표준 산업분류.
    pub ksic_code: KsicCode,
    /// 종사자 수 구간 (`KOSIS` 기준).
    pub employee_count_band: EmployeeCountBand,
    /// 설립연도 (선택).
    pub founded_year: Option<u16>,
    /// 대표자명 (공시, 선택).
    pub representative_name: Option<String>,
    /// `R2` 객체에서 fetch한 시각 (캐시 만료 판단용).
    pub fetched_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::Manufacturer;
    use crate::employee_count_band::EmployeeCountBand;
    use chrono::Utc;
    use shared_kernel::business_number::BusinessNumber;
    use shared_kernel::ksic_code::KsicCode;
    use shared_kernel::pnu::Pnu;

    /// 체크섬 통과하는 샘플 사업자번호 (NTS 알고리즘 기준).
    /// `1234567891` — `shared-kernel` 의 `business_number` 단위 테스트에서 검증된 값.
    fn sample_business_number() -> BusinessNumber {
        BusinessNumber::try_new("1234567891").expect("valid business number")
    }

    #[test]
    fn manufacturer_constructs_from_r2_data() {
        let m = Manufacturer {
            business_number: sample_business_number(),
            company_name: "샘플전자".to_owned(),
            industrial_complex_code: Some("I000001".to_owned()),
            pnu: Some(Pnu::try_new("1111010100100010000").unwrap()),
            ksic_code: KsicCode::try_new("C2620").expect("valid"),
            employee_count_band: EmployeeCountBand::TenToFortyNine,
            founded_year: Some(2010),
            representative_name: Some("홍길동".to_owned()),
            fetched_at: Utc::now(),
        };
        assert_eq!(m.company_name, "샘플전자");
        assert_eq!(m.ksic_code.as_str(), "C2620");
        assert_eq!(m.employee_count_band, EmployeeCountBand::TenToFortyNine);
        assert_eq!(m.founded_year, Some(2010));
    }

    #[test]
    fn manufacturer_optional_fields_none() {
        let m = Manufacturer {
            business_number: sample_business_number(),
            company_name: "최소정보업체".to_owned(),
            industrial_complex_code: None,
            pnu: None,
            ksic_code: KsicCode::try_new("C1010").expect("valid"),
            employee_count_band: EmployeeCountBand::OneToFour,
            founded_year: None,
            representative_name: None,
            fetched_at: Utc::now(),
        };
        assert!(m.industrial_complex_code.is_none());
        assert!(m.pnu.is_none());
        assert!(m.founded_year.is_none());
        assert!(m.representative_name.is_none());
    }

    #[test]
    fn manufacturer_serde_roundtrip() {
        let m = Manufacturer {
            business_number: sample_business_number(),
            company_name: "테스트제조".to_owned(),
            industrial_complex_code: Some("I000050".to_owned()),
            pnu: Some(Pnu::try_new("2817700100100010000").unwrap()),
            ksic_code: KsicCode::try_new("C2511").expect("valid"),
            employee_count_band: EmployeeCountBand::OneHundredToTwoNinetyNine,
            founded_year: Some(1995),
            representative_name: Some("김대표".to_owned()),
            fetched_at: Utc::now(),
        };
        let json = serde_json::to_string(&m).expect("serialize");
        let back: Manufacturer = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(m, back);
    }

    #[test]
    fn manufacturer_clone_preserves_fields() {
        let m = Manufacturer {
            business_number: sample_business_number(),
            company_name: "복제테스트".to_owned(),
            industrial_complex_code: None,
            pnu: None,
            ksic_code: KsicCode::try_new("C3100").expect("valid"),
            employee_count_band: EmployeeCountBand::ThreeHundredPlus,
            founded_year: Some(1980),
            representative_name: Some("이대표".to_owned()),
            fetched_at: Utc::now(),
        };
        let cloned = m.clone();
        assert_eq!(m, cloned);
    }
}
