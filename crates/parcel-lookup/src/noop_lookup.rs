//! [`NoOpParcelInfoLookup`] — 모든 lookup 이 `Ok(None)` 을 반환하는 stub.
//!
//! 용도:
//! - **로컬 dev / 통합 테스트** — V-World 자격 증명 없이 listing 등록 흐름 검증
//! - **CI smoke** — V-World 외부 의존성 없이 컴파일/E2E 패스
//!
//! production 에서는 절대 사용하지 않음 — `VWORLD_API_KEY` 미설정 시 main 에서
//! `tracing::warn!` 으로 의도된 fallback 임을 명시.

use async_trait::async_trait;
use shared_kernel::pnu::Pnu;

use crate::info::ParcelInfo;
use crate::lookup::{LookupError, ParcelInfoLookup};

/// 모든 PNU lookup 이 `Ok(None)` 을 반환.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoOpParcelInfoLookup;

impl NoOpParcelInfoLookup {
    /// 새 `NoOp` 인스턴스.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ParcelInfoLookup for NoOpParcelInfoLookup {
    async fn lookup_by_pnu(&self, _pnu: &Pnu) -> Result<Option<ParcelInfo>, LookupError> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[tokio::test]
    async fn noop_returns_none_for_any_pnu() {
        let lookup = NoOpParcelInfoLookup::new();
        let pnu = Pnu::try_new("1168010100107370000").unwrap();
        assert!(lookup.lookup_by_pnu(&pnu).await.unwrap().is_none());
    }
}
