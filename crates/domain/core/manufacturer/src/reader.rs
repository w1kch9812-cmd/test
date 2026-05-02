//! `ManufacturerReader` port. 구현체는 sub-project 4 (`crates/data-clients/r2-public-data/`).

// `ManufacturerReader` 처럼 모듈명 반복은 의도된 공개 API 형태.
#![allow(clippy::module_name_repetitions)]

use async_trait::async_trait;
use shared_kernel::business_number::BusinessNumber;

use crate::entity::Manufacturer;
use crate::errors::ReaderError;

/// `Manufacturer` 조회 포트 (`R2` 정적).
///
/// 식별은 `BusinessNumber` 로. 산단 입주 / `KSIC` 대분류 기반 다수 조회는 `Vec` 반환.
#[async_trait]
pub trait ManufacturerReader: Send + Sync {
    /// 사업자등록번호로 조회.
    ///
    /// 미존재 시 `Ok(None)` — `NotFound` 는 hard-error 경로 (예: 인덱스 깨짐).
    ///
    /// # Errors
    ///
    /// 네트워크 실패 → `Fetch`. 데이터 파싱 실패 → `Parse`.
    async fn fetch_by_business_number(
        &self,
        bn: &BusinessNumber,
    ) -> Result<Option<Manufacturer>, ReaderError>;

    /// 산단 코드(`industrial_complex_code`)로 입주 제조업체 전체 조회.
    ///
    /// 매칭 없으면 빈 `Vec`. `R2` 자체 접근 실패만 에러로 반환.
    ///
    /// # Errors
    ///
    /// 네트워크 실패 → `Fetch`. 데이터 파싱 실패 → `Parse`.
    async fn fetch_by_industrial_complex(
        &self,
        ic_code: &str,
    ) -> Result<Vec<Manufacturer>, ReaderError>;

    /// `KSIC` 대분류 (section 영문 1자, 예: `'C'` = 제조업) 로 조회.
    ///
    /// 매칭 없으면 빈 `Vec`.
    ///
    /// # Errors
    ///
    /// 네트워크 실패 → `Fetch`. 데이터 파싱 실패 → `Parse`.
    async fn fetch_by_ksic_section(&self, section: char) -> Result<Vec<Manufacturer>, ReaderError>;
}

// Trait shape 검증만 — 실제 비동기 실행은 sub-project 4 구현체 테스트에서.
#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::{BusinessNumber, Manufacturer, ManufacturerReader, ReaderError};
    use crate::employee_count_band::EmployeeCountBand;
    use async_trait::async_trait;
    use chrono::Utc;
    use shared_kernel::ksic_code::KsicCode;

    fn sample_manufacturer(name: &str) -> Manufacturer {
        Manufacturer {
            business_number: BusinessNumber::try_new("1234567891").expect("valid"),
            company_name: name.to_owned(),
            industrial_complex_code: Some("I000001".to_owned()),
            pnu: None,
            ksic_code: KsicCode::try_new("C2620").expect("valid"),
            employee_count_band: EmployeeCountBand::TenToFortyNine,
            founded_year: Some(2010),
            representative_name: None,
            fetched_at: Utc::now(),
        }
    }

    struct StubReader {
        manufacturers: Vec<Manufacturer>,
    }

    #[async_trait]
    impl ManufacturerReader for StubReader {
        async fn fetch_by_business_number(
            &self,
            bn: &BusinessNumber,
        ) -> Result<Option<Manufacturer>, ReaderError> {
            Ok(self
                .manufacturers
                .iter()
                .find(|m| m.business_number == *bn)
                .cloned())
        }

        async fn fetch_by_industrial_complex(
            &self,
            ic_code: &str,
        ) -> Result<Vec<Manufacturer>, ReaderError> {
            Ok(self
                .manufacturers
                .iter()
                .filter(|m| m.industrial_complex_code.as_deref() == Some(ic_code))
                .cloned()
                .collect())
        }

        async fn fetch_by_ksic_section(
            &self,
            section: char,
        ) -> Result<Vec<Manufacturer>, ReaderError> {
            Ok(self
                .manufacturers
                .iter()
                .filter(|m| m.ksic_code.section() == section)
                .cloned()
                .collect())
        }
    }

    /// `ManufacturerReader` 가 trait object 로 사용 가능한지 (`Send + Sync`) 컴파일 타임 검증.
    #[test]
    fn reader_is_object_safe() {
        fn assert_obj_safe<T: ManufacturerReader + ?Sized>() {}
        assert_obj_safe::<dyn ManufacturerReader>();
    }

    /// `StubReader` 가 trait 을 만족하는지 (Send + Sync 포함) 컴파일 타임 검증.
    #[test]
    fn stub_reader_implements_trait() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<StubReader>();
        let _r = StubReader {
            manufacturers: vec![sample_manufacturer("ACME 제조")],
        };
    }
}
