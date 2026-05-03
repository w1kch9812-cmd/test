//! 공짱 인증 핵심 게이트 — Zitadel access_token `JWT` 검증.
//!
//! - [`verifier::JwtVerifier`] — `JWKS` 캐시 + 서명·exp·iss·aud 검증
//! - [`middleware`] — Axum tower layer (`Bearer` → `Extension<AuthenticatedUser>`)
//! - [`extractor::AuthenticatedUser`] — 핸들러용 extractor
//! - [`role_guard::require_role`] — `UserRole` 가드 helper
//!
//! Spec: `docs/superpowers/specs/2026-05-03-sub-project-3-auth-zitadel-jwt-design.md`

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod claims;
pub mod errors;
pub mod extractor;
pub mod jwks_cache;
pub mod middleware;
pub mod role_guard;
pub mod verifier;
