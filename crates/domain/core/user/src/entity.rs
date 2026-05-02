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
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    fn sample_email() -> Email {
        Email::try_new("alice@example.com").expect("valid")
    }

    // 64-char hex (lowercase). SHA-256 of "test" — fixture only.
    const SAMPLE_PHONE_HASH: &str =
        "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";

    fn sample_business_number() -> BusinessNumber {
        BusinessNumber::try_new("1234567891").expect("valid")
    }

    fn sample_broker_license() -> BrokerLicense {
        BrokerLicense::try_new("11-2024-12345").expect("valid")
    }

    // ── try_new (minimal, 6 args) ─────────────────────────────────────────────

    #[test]
    fn try_new_happy_path() {
        let id = Id::<UserMarker>::new();
        let now = Utc::now();
        let user = User::try_new(
            id.clone(),
            "zitadel-sub-1",
            sample_email(),
            "Alice",
            UserKind::Individual,
            now,
        )
        .expect("valid");
        assert_eq!(user.id, id);
        assert_eq!(user.display_name, "Alice");
        assert_eq!(user.version, 1);
        assert_eq!(user.created_at, now);
        assert_eq!(user.updated_at, now);
        // Optional fields default
        assert!(user.phone_kr_hash.is_none());
        assert!(user.business_number.is_none());
        assert!(user.business_verified_at.is_none());
        assert!(user.broker_license_number.is_none());
        assert!(user.broker_verified_at.is_none());
        assert!(user.roles.is_empty());
        assert!(user.nice_verified_at.is_none());
        assert!(user.marketing_consent_at.is_none());
        assert!(user.last_login_at.is_none());
        assert!(user.deleted_at.is_none());
    }

    #[test]
    fn rejects_empty_display_name() {
        let err = User::try_new(
            Id::new(),
            "sub",
            sample_email(),
            "",
            UserKind::Individual,
            Utc::now(),
        )
        .unwrap_err();
        assert!(matches!(err, UserError::EmptyDisplayName));
    }

    #[test]
    fn rejects_whitespace_only_display_name() {
        let err = User::try_new(
            Id::new(),
            "sub",
            sample_email(),
            "   ",
            UserKind::Individual,
            Utc::now(),
        )
        .unwrap_err();
        assert!(matches!(err, UserError::EmptyDisplayName));
    }

    #[test]
    fn rejects_too_long_display_name() {
        let long = "X".repeat(101);
        let err = User::try_new(
            Id::new(),
            "sub",
            sample_email(),
            &long,
            UserKind::Individual,
            Utc::now(),
        )
        .unwrap_err();
        assert!(matches!(err, UserError::DisplayNameTooLong { actual: 101 }));
    }

    #[test]
    fn accepts_exactly_100_char_display_name() {
        let exactly = "X".repeat(100);
        let user = User::try_new(
            Id::new(),
            "sub",
            sample_email(),
            &exactly,
            UserKind::Individual,
            Utc::now(),
        )
        .expect("100 chars OK");
        assert_eq!(user.display_name, exactly);
    }

    #[test]
    fn rejects_empty_zitadel_sub() {
        let err = User::try_new(
            Id::new(),
            "",
            sample_email(),
            "Alice",
            UserKind::Individual,
            Utc::now(),
        )
        .unwrap_err();
        assert!(matches!(err, UserError::EmptyZitadelSub));
    }

    #[test]
    fn rejects_whitespace_only_zitadel_sub() {
        let err = User::try_new(
            Id::new(),
            "   ",
            sample_email(),
            "Alice",
            UserKind::Individual,
            Utc::now(),
        )
        .unwrap_err();
        assert!(matches!(err, UserError::EmptyZitadelSub));
    }

    #[test]
    fn rejects_too_long_zitadel_sub() {
        let long = "X".repeat(256);
        let err = User::try_new(
            Id::new(),
            &long,
            sample_email(),
            "Alice",
            UserKind::Individual,
            Utc::now(),
        )
        .unwrap_err();
        assert!(matches!(err, UserError::ZitadelSubTooLong { actual: 256 }));
    }

    #[test]
    fn accepts_exactly_255_char_zitadel_sub() {
        let exactly = "X".repeat(255);
        let user = User::try_new(
            Id::new(),
            &exactly,
            sample_email(),
            "Alice",
            UserKind::Individual,
            Utc::now(),
        )
        .expect("255 chars OK");
        assert_eq!(user.zitadel_sub, exactly);
    }

    #[test]
    fn user_kind_serializes_snake_case() {
        let json = serde_json::to_string(&UserKind::Individual).expect("ok");
        assert_eq!(json, r#""individual""#);
        let json = serde_json::to_string(&UserKind::Corporation).expect("ok");
        assert_eq!(json, r#""corporation""#);
    }

    #[test]
    fn user_kind_as_str() {
        assert_eq!(UserKind::Individual.as_str(), "individual");
        assert_eq!(UserKind::Corporation.as_str(), "corporation");
    }

    #[test]
    fn user_serde_roundtrip_minimal() {
        let now = Utc::now();
        let user = User::try_new(
            Id::new(),
            "sub",
            sample_email(),
            "Alice",
            UserKind::Individual,
            now,
        )
        .expect("valid");
        let json = serde_json::to_string(&user).expect("serialize");
        let back: User = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(user, back);
    }

    #[test]
    fn corporation_user_kind_works() {
        let user = User::try_new(
            Id::new(),
            "sub",
            sample_email(),
            "Acme Co.",
            UserKind::Corporation,
            Utc::now(),
        )
        .expect("valid");
        assert_eq!(user.user_kind, UserKind::Corporation);
    }

    // ── try_new_full (full, 14 args) ──────────────────────────────────────────

    #[test]
    fn try_new_full_happy_path_all_some() {
        let now = Utc::now();
        let user = User::try_new_full(
            Id::new(),
            "sub-full",
            sample_email(),
            Some(SAMPLE_PHONE_HASH.to_owned()),
            "Alice",
            UserKind::Corporation,
            Some(sample_business_number()),
            Some(now),
            Some(sample_broker_license()),
            Some(now),
            vec![UserRole::Buyer, UserRole::Broker],
            Some(now),
            Some(now),
            now,
        )
        .expect("valid full");
        assert_eq!(user.phone_kr_hash.as_deref(), Some(SAMPLE_PHONE_HASH));
        assert!(user.business_number.is_some());
        assert_eq!(user.business_verified_at, Some(now));
        assert!(user.broker_license_number.is_some());
        assert_eq!(user.broker_verified_at, Some(now));
        assert_eq!(user.roles.len(), 2);
        assert_eq!(user.nice_verified_at, Some(now));
        assert_eq!(user.marketing_consent_at, Some(now));
        assert_eq!(user.last_login_at, None);
        assert_eq!(user.deleted_at, None);
        assert_eq!(user.version, 1);
    }

    #[test]
    fn try_new_full_happy_path_all_none_matches_try_new() {
        let now = Utc::now();
        let id = Id::<UserMarker>::new();
        let from_full = User::try_new_full(
            id.clone(),
            "sub",
            sample_email(),
            None,
            "Alice",
            UserKind::Individual,
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
            None,
            now,
        )
        .expect("valid");
        let from_min = User::try_new(
            id,
            "sub",
            sample_email(),
            "Alice",
            UserKind::Individual,
            now,
        )
        .expect("valid");
        assert_eq!(from_full, from_min);
    }

    #[test]
    fn rejects_phone_hash_wrong_length() {
        let now = Utc::now();
        let too_short = "a".repeat(63);
        let err = User::try_new_full(
            Id::new(),
            "sub",
            sample_email(),
            Some(too_short),
            "Alice",
            UserKind::Individual,
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
            None,
            now,
        )
        .unwrap_err();
        assert!(matches!(err, UserError::InvalidPhoneHash));
    }

    #[test]
    fn rejects_phone_hash_non_hex() {
        let now = Utc::now();
        // 64 chars but contains 'g' (not hex).
        let bad = "g".repeat(64);
        let err = User::try_new_full(
            Id::new(),
            "sub",
            sample_email(),
            Some(bad),
            "Alice",
            UserKind::Individual,
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
            None,
            now,
        )
        .unwrap_err();
        assert!(matches!(err, UserError::InvalidPhoneHash));
    }

    #[test]
    fn accepts_uppercase_phone_hash() {
        // SHA-256 hex digits A-F should be accepted (is_ascii_hexdigit).
        let now = Utc::now();
        let upper = "9F86D081884C7D659A2FEAA0C55AD015A3BF4F1B2B0B822CD15D6C15B0F00A08";
        let user = User::try_new_full(
            Id::new(),
            "sub",
            sample_email(),
            Some(upper.to_owned()),
            "Alice",
            UserKind::Individual,
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
            None,
            now,
        )
        .expect("uppercase hex OK");
        assert_eq!(user.phone_kr_hash.as_deref(), Some(upper));
    }

    #[test]
    fn rejects_business_verified_without_business_number() {
        let now = Utc::now();
        let err = User::try_new_full(
            Id::new(),
            "sub",
            sample_email(),
            None,
            "Alice",
            UserKind::Corporation,
            None, // no business_number
            Some(now),
            None,
            None,
            Vec::new(),
            None,
            None,
            now,
        )
        .unwrap_err();
        assert!(matches!(err, UserError::BusinessVerificationInconsistent));
    }

    #[test]
    fn rejects_broker_verified_without_broker_license() {
        let now = Utc::now();
        let err = User::try_new_full(
            Id::new(),
            "sub",
            sample_email(),
            None,
            "Alice",
            UserKind::Individual,
            None,
            None,
            None, // no broker_license_number
            Some(now),
            Vec::new(),
            None,
            None,
            now,
        )
        .unwrap_err();
        assert!(matches!(err, UserError::BrokerVerificationInconsistent));
    }

    #[test]
    fn accepts_business_verification_with_number() {
        let now = Utc::now();
        let user = User::try_new_full(
            Id::new(),
            "sub",
            sample_email(),
            None,
            "Alice",
            UserKind::Corporation,
            Some(sample_business_number()),
            Some(now),
            None,
            None,
            Vec::new(),
            None,
            None,
            now,
        )
        .expect("valid");
        assert!(user.business_number.is_some());
        assert_eq!(user.business_verified_at, Some(now));
    }

    #[test]
    fn accepts_broker_verification_with_license() {
        let now = Utc::now();
        let user = User::try_new_full(
            Id::new(),
            "sub",
            sample_email(),
            None,
            "Alice",
            UserKind::Individual,
            None,
            None,
            Some(sample_broker_license()),
            Some(now),
            vec![UserRole::Broker],
            None,
            None,
            now,
        )
        .expect("valid");
        assert!(user.broker_license_number.is_some());
        assert_eq!(user.broker_verified_at, Some(now));
    }

    #[test]
    fn business_number_some_without_verified_is_ok() {
        // 사업자번호는 입력했지만 아직 검증 전 — 합법.
        let now = Utc::now();
        let user = User::try_new_full(
            Id::new(),
            "sub",
            sample_email(),
            None,
            "Alice",
            UserKind::Corporation,
            Some(sample_business_number()),
            None, // not yet verified
            None,
            None,
            Vec::new(),
            None,
            None,
            now,
        )
        .expect("valid");
        assert!(user.business_number.is_some());
        assert_eq!(user.business_verified_at, None);
    }

    #[test]
    fn roles_with_multiple_values() {
        let now = Utc::now();
        let user = User::try_new_full(
            Id::new(),
            "sub",
            sample_email(),
            None,
            "Alice",
            UserKind::Individual,
            None,
            None,
            None,
            None,
            vec![UserRole::Buyer, UserRole::Seller, UserRole::Operator],
            None,
            None,
            now,
        )
        .expect("valid");
        assert_eq!(user.roles.len(), 3);
        assert_eq!(user.roles[0], UserRole::Buyer);
        assert_eq!(user.roles[1], UserRole::Seller);
        assert_eq!(user.roles[2], UserRole::Operator);
    }

    // ── UserRole ──────────────────────────────────────────────────────────────

    #[test]
    fn user_role_as_str_all_variants() {
        assert_eq!(UserRole::Buyer.as_str(), "Buyer");
        assert_eq!(UserRole::Seller.as_str(), "Seller");
        assert_eq!(UserRole::Broker.as_str(), "Broker");
        assert_eq!(UserRole::Developer.as_str(), "Developer");
        assert_eq!(UserRole::Enterprise.as_str(), "Enterprise");
        assert_eq!(UserRole::Operator.as_str(), "Operator");
        assert_eq!(UserRole::Admin.as_str(), "Admin");
    }

    #[test]
    fn user_role_serde_roundtrip() {
        for role in [
            UserRole::Buyer,
            UserRole::Seller,
            UserRole::Broker,
            UserRole::Developer,
            UserRole::Enterprise,
            UserRole::Operator,
            UserRole::Admin,
        ] {
            let json = serde_json::to_string(&role).expect("serialize");
            let back: UserRole = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(role, back);
        }
    }

    // ── Full serde round-trip ─────────────────────────────────────────────────

    #[test]
    fn user_serde_roundtrip_full() {
        let now = Utc::now();
        let user = User::try_new_full(
            Id::new(),
            "sub-full",
            sample_email(),
            Some(SAMPLE_PHONE_HASH.to_owned()),
            "Alice",
            UserKind::Corporation,
            Some(sample_business_number()),
            Some(now),
            Some(sample_broker_license()),
            Some(now),
            vec![UserRole::Buyer, UserRole::Broker],
            Some(now),
            Some(now),
            now,
        )
        .expect("valid");
        let json = serde_json::to_string(&user).expect("serialize");
        let back: User = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(user, back);
    }
}
