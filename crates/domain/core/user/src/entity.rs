//! `User` Aggregate (Core BC, RDS 동적) — spec § 5.1 18 필드.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared_kernel::broker_license::BrokerLicense;
use shared_kernel::business_number::BusinessNumber;
use shared_kernel::email::Email;
use shared_kernel::id::{Id, UserMarker};

use crate::errors::UserError;

/// 사용자 종류.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserKind {
    /// 개인.
    Individual,
    /// 법인.
    Corporation,
}

impl UserKind {
    /// 정규화된 `snake_case` 문자열을 반환해요 (`DB user_kind` 컬럼 매핑).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Individual => "individual",
            Self::Corporation => "corporation",
        }
    }
}

/// 사용자 역할 (`DB roles text[]` 컬럼 원소).
///
/// 한 사용자는 동시에 여러 역할을 가질 수 있어요 (예: `Buyer` + `Seller`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UserRole {
    /// 매수자.
    Buyer,
    /// 매도자.
    Seller,
    /// 공인중개사.
    Broker,
    /// 시행사.
    Developer,
    /// 기업 사용자.
    Enterprise,
    /// 운영자.
    Operator,
    /// 관리자.
    Admin,
}

impl UserRole {
    /// 정규화된 문자열을 반환해요 (`DB roles text[]` 원소).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Buyer => "Buyer",
            Self::Seller => "Seller",
            Self::Broker => "Broker",
            Self::Developer => "Developer",
            Self::Enterprise => "Enterprise",
            Self::Operator => "Operator",
            Self::Admin => "Admin",
        }
    }
}

/// `User` Aggregate (spec § 5.1 — 18 필드).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    /// `usr_<26-char ULID>` 형식 `ID`.
    pub id: Id<UserMarker>,
    /// `Zitadel` `JWT` `sub` claim. 비어있지 않고 ≤255자.
    pub zitadel_sub: String,
    /// 이메일.
    pub email: Email,
    /// `SHA-256` 해시 (PIPA — 평문 저장 금지). 64-char hex string.
    pub phone_kr_hash: Option<String>,
    /// 표시 이름. 비어있지 않고 ≤100자.
    pub display_name: String,
    /// 사용자 종류 (개인/법인).
    pub user_kind: UserKind,
    /// 사업자등록번호 (검증된 경우만 `Some`).
    pub business_number: Option<BusinessNumber>,
    /// 사업자 검증 시각.
    pub business_verified_at: Option<DateTime<Utc>>,
    /// 공인중개사 자격번호.
    pub broker_license_number: Option<BrokerLicense>,
    /// 중개사 검증 시각.
    pub broker_verified_at: Option<DateTime<Utc>>,
    /// 역할 목록 (`Buyer`/`Seller`/`Broker`/`Developer`/`Enterprise`/`Operator`/`Admin`).
    pub roles: Vec<UserRole>,
    /// `NICE` 본인인증 시각.
    pub nice_verified_at: Option<DateTime<Utc>>,
    /// 마케팅 동의 시각.
    pub marketing_consent_at: Option<DateTime<Utc>>,
    /// 생성 시각.
    pub created_at: DateTime<Utc>,
    /// 마지막 갱신 시각.
    pub updated_at: DateTime<Utc>,
    /// 마지막 로그인 시각.
    pub last_login_at: Option<DateTime<Utc>>,
    /// Soft-delete 시각 (PIPA `RTBF`). `None`이면 활성.
    pub deleted_at: Option<DateTime<Utc>>,
    /// Optimistic locking 버전.
    pub version: i64,
}

impl User {
    /// 최소 필드로 새 `User`를 생성해요 (Walking Skeleton 호환).
    ///
    /// Optional 필드는 모두 `None` / `Vec::new()`. `created_at == updated_at == now`,
    /// `version = 1`, `last_login_at == None`, `deleted_at == None`.
    ///
    /// # Errors
    ///
    /// `display_name` 빈 → [`UserError::EmptyDisplayName`]. 100자 초과 → [`UserError::DisplayNameTooLong`].
    /// `zitadel_sub` 빈 → [`UserError::EmptyZitadelSub`]. 255자 초과 → [`UserError::ZitadelSubTooLong`].
    // 6 args (>5 default) — Walking Skeleton 호환 시그니처라 의도적.
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        id: Id<UserMarker>,
        zitadel_sub: &str,
        email: Email,
        display_name: &str,
        user_kind: UserKind,
        now: DateTime<Utc>,
    ) -> Result<Self, UserError> {
        Self::try_new_full(
            id,
            zitadel_sub,
            email,
            None,
            display_name,
            user_kind,
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
            None,
            now,
        )
    }

    /// 모든 필드로 새 `User`를 생성해요.
    ///
    /// `created_at == updated_at == now`, `version = 1`, `last_login_at == None`,
    /// `deleted_at == None`.
    ///
    /// # Errors
    ///
    /// 위 [`Self::try_new`]와 동일한 에러에 더해:
    /// - `phone_kr_hash` `Some`이면 64-char hex string이어야 — 위반 → [`UserError::InvalidPhoneHash`].
    /// - `business_verified_at` `Some`인데 `business_number` `None` → [`UserError::BusinessVerificationInconsistent`].
    /// - `broker_verified_at` `Some`인데 `broker_license_number` `None` → [`UserError::BrokerVerificationInconsistent`].
    // Aggregate 풀 생성자 — 의도적으로 전 필드 명시.
    #[allow(clippy::too_many_arguments)]
    pub fn try_new_full(
        id: Id<UserMarker>,
        zitadel_sub: &str,
        email: Email,
        phone_kr_hash: Option<String>,
        display_name: &str,
        user_kind: UserKind,
        business_number: Option<BusinessNumber>,
        business_verified_at: Option<DateTime<Utc>>,
        broker_license_number: Option<BrokerLicense>,
        broker_verified_at: Option<DateTime<Utc>>,
        roles: Vec<UserRole>,
        nice_verified_at: Option<DateTime<Utc>>,
        marketing_consent_at: Option<DateTime<Utc>>,
        now: DateTime<Utc>,
    ) -> Result<Self, UserError> {
        let display_name = display_name.trim();
        if display_name.is_empty() {
            return Err(UserError::EmptyDisplayName);
        }
        let display_len = display_name.chars().count();
        if display_len > 100 {
            return Err(UserError::DisplayNameTooLong {
                actual: display_len,
            });
        }

        let zitadel_sub = zitadel_sub.trim();
        if zitadel_sub.is_empty() {
            return Err(UserError::EmptyZitadelSub);
        }
        let sub_len = zitadel_sub.chars().count();
        if sub_len > 255 {
            return Err(UserError::ZitadelSubTooLong { actual: sub_len });
        }

        if let Some(ref hash) = phone_kr_hash {
            if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(UserError::InvalidPhoneHash);
            }
        }

        if business_verified_at.is_some() && business_number.is_none() {
            return Err(UserError::BusinessVerificationInconsistent);
        }
        if broker_verified_at.is_some() && broker_license_number.is_none() {
            return Err(UserError::BrokerVerificationInconsistent);
        }

        Ok(Self {
            id,
            zitadel_sub: zitadel_sub.to_owned(),
            email,
            phone_kr_hash,
            display_name: display_name.to_owned(),
            user_kind,
            business_number,
            business_verified_at,
            broker_license_number,
            broker_verified_at,
            roles,
            nice_verified_at,
            marketing_consent_at,
            created_at: now,
            updated_at: now,
            last_login_at: None,
            deleted_at: None,
            version: 1,
        })
    }

    // ── 도메인 mutation 메서드 ────────────────────────────────────────────────

    /// 사업자 검증 완료. `business_number`/`business_verified_at` 설정 + `version` bump.
    ///
    /// 같은 `User`에 중복 호출 시 가장 최근 시각으로 갱신해요 (idempotent).
    pub fn verify_business(&mut self, business_number: BusinessNumber, at: DateTime<Utc>) {
        self.business_number = Some(business_number);
        self.business_verified_at = Some(at);
        self.bump_version(at);
    }

    /// 사업자 검증 취소. `business_verified_at`만 비우고 `business_number`는 보존해요
    /// (재검증 가능).
    pub fn revoke_business_verification(&mut self, at: DateTime<Utc>) {
        self.business_verified_at = None;
        self.bump_version(at);
    }

    /// 공인중개사 검증 완료. `broker_license_number`/`broker_verified_at` 설정 +
    /// `version` bump.
    pub fn verify_broker(&mut self, license: BrokerLicense, at: DateTime<Utc>) {
        self.broker_license_number = Some(license);
        self.broker_verified_at = Some(at);
        self.bump_version(at);
    }

    /// 중개사 검증 취소. `broker_verified_at`만 비우고 `broker_license_number`는 보존.
    pub fn revoke_broker_verification(&mut self, at: DateTime<Utc>) {
        self.broker_verified_at = None;
        self.bump_version(at);
    }

    /// `NICE` 본인인증 기록.
    pub fn record_nice_verification(&mut self, at: DateTime<Utc>) {
        self.nice_verified_at = Some(at);
        self.bump_version(at);
    }

    /// 마케팅 동의 기록.
    pub fn record_marketing_consent(&mut self, at: DateTime<Utc>) {
        self.marketing_consent_at = Some(at);
        self.bump_version(at);
    }

    /// 마케팅 동의 철회.
    pub fn revoke_marketing_consent(&mut self, at: DateTime<Utc>) {
        self.marketing_consent_at = None;
        self.bump_version(at);
    }

    /// 마지막 로그인 시각 기록.
    ///
    /// `version`은 일부러 bump *안* 함 — 로그인은 빈번한 갱신이며 동시성 충돌
    /// 검사 대상이 아니에요. `updated_at`만 갱신.
    pub fn record_login(&mut self, at: DateTime<Utc>) {
        self.last_login_at = Some(at);
        self.updated_at = at;
    }

    /// 역할 추가. 이미 보유한 역할이면 변화 없음 (no-op).
    pub fn add_role(&mut self, role: UserRole, at: DateTime<Utc>) {
        if !self.roles.contains(&role) {
            self.roles.push(role);
            self.bump_version(at);
        }
    }

    /// 역할 제거. 보유하지 않은 역할이면 변화 없음 (no-op).
    pub fn remove_role(&mut self, role: UserRole, at: DateTime<Utc>) {
        let len_before = self.roles.len();
        self.roles.retain(|r| *r != role);
        if self.roles.len() != len_before {
            self.bump_version(at);
        }
    }

    /// 역할 보유 여부.
    #[must_use]
    pub fn has_role(&self, role: UserRole) -> bool {
        self.roles.contains(&role)
    }

    /// `PIPA` `RTBF` Soft-delete. `deleted_at` 설정 + `version` bump.
    ///
    /// 이미 삭제된 경우 무시해요 (idempotent).
    pub fn soft_delete(&mut self, at: DateTime<Utc>) {
        if self.deleted_at.is_none() {
            self.deleted_at = Some(at);
            self.bump_version(at);
        }
    }

    /// 활성 사용자 여부 (soft-delete 안 된 상태).
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.deleted_at.is_none()
    }

    /// 사업자 검증 완료 여부.
    #[must_use]
    pub const fn is_business_verified(&self) -> bool {
        self.business_verified_at.is_some()
    }

    /// 중개사 검증 완료 여부.
    #[must_use]
    pub const fn is_broker(&self) -> bool {
        self.broker_verified_at.is_some()
    }

    /// 내부 헬퍼: `version` bump + `updated_at` 갱신.
    fn bump_version(&mut self, at: DateTime<Utc>) {
        self.version += 1;
        self.updated_at = at;
    }
}

#[cfg(test)]
#[path = "entity_tests.rs"]
mod tests;
