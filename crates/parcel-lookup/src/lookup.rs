//! [`ParcelInfoLookup`] port trait + 에러.

use async_trait::async_trait;
use shared_kernel::pnu::Pnu;
use thiserror::Error;

use crate::info::ParcelInfo;

/// PNU → [`ParcelInfo`] lookup port.
///
/// 구현체:
/// - [`crate::vworld_lookup::VWorldParcelInfoLookup`] — V-World 직접 호출
/// - (향후) Redis 캐시 wrapper
/// - (향후) Bronze SHP R-tree 인메모리 인덱스
///
/// `Ok(None)` 은 *해당 PNU 의 필지가 정부 데이터에 없음* — 호출자가 정상으로
/// 처리. 네트워크/파싱 실패는 [`LookupError`] 로 분리.
#[async_trait]
pub trait ParcelInfoLookup: Send + Sync {
    /// PNU 한 건 조회.
    ///
    /// # Errors
    ///
    /// - 백엔드 통신 실패 → [`LookupError::Backend`]
    /// - 응답 파싱 실패 → [`LookupError::Parse`]
    async fn lookup_by_pnu(&self, pnu: &Pnu) -> Result<Option<ParcelInfo>, LookupError>;
}

/// Lookup 에러 — RFC 7807 매핑은 호출자(라우트) 책임.
#[derive(Debug, Error)]
pub enum LookupError {
    /// 백엔드 (V-World 등) 통신 실패.
    #[error("backend error: {0}")]
    Backend(String),
    /// 응답 파싱/도메인 invariant 위반.
    #[error("parse error: {0}")]
    Parse(String),
}
