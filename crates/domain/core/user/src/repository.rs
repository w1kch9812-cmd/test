//! `UserRepository` port (interface). 구현체는 `crates/db` 또는 sub-project 5에서.

// `UserRepository` 처럼 모듈명 반복은 의도된 공개 API 형태.
#![allow(clippy::module_name_repetitions)]

use async_trait::async_trait;
use shared_kernel::email::Email;
use shared_kernel::id::{Id, UserMarker};
use shared_kernel::mutation::MutationContext;
use thiserror::Error;

use crate::entity::User;

/// `User` 저장/조회 포트.
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// `id`로 활성 사용자 조회 (`deleted_at IS NULL`). 없으면 `Ok(None)`.
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn find_by_id(&self, id: &Id<UserMarker>) -> Result<Option<User>, RepoError>;

    /// `zitadel_sub`로 활성 사용자 조회 (인증 미들웨어 lookup 용).
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn find_by_zitadel_sub(&self, sub: &str) -> Result<Option<User>, RepoError>;

    /// `Email`로 활성 사용자 조회 (가입 중복 방지).
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, RepoError>;

    /// 저장 (insert or update). Optimistic lock 충돌 시 [`RepoError::Conflict`].
    ///
    /// `ctx` 의 `actor_id` / `action` / `metadata` / `events` 가 같은 트랜잭션
    /// 안에서 `audit_log` 와 `outbox_event` 로 자동 기록돼요 (SP5-iv 의
    /// transactional 패턴). 시스템 mutation (예: first-sign-in) 은
    /// [`MutationContext::new_system_action`].
    ///
    /// # Errors
    ///
    /// 버전 불일치 → [`RepoError::Conflict`]. DB 통신 실패 → [`RepoError::Database`].
    async fn save(&self, user: &User, ctx: MutationContext) -> Result<(), RepoError>;

    /// `external_account` 에 `zitadel` 행을 삽입해요 (first sign-in 시 호출).
    ///
    /// `ON CONFLICT DO NOTHING` — 중복 호출 safe. best-effort (실패해도 인증 흐름 차단 안 함).
    /// SP6-Social federation 이 kakao/naver/google 행을 채워요.
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn link_zitadel_account(
        &self,
        user_id: &Id<UserMarker>,
        zitadel_sub: &str,
    ) -> Result<(), RepoError>;
}

/// `Repository` 에러.
#[derive(Debug, Error)]
pub enum RepoError {
    /// 대상 Aggregate 미존재.
    #[error("not found")]
    NotFound,
    /// Optimistic lock 버전 불일치.
    #[error("conflict (version mismatch)")]
    Conflict,
    /// DB 통신/SQL 에러 (정보 누설 방지로 메시지만).
    #[error("database error: {0}")]
    Database(String),
}
