//! 통합 테스트 공통 헬퍼.
//!
//! `DATABASE_URL` 환경 변수로 `Postgres` 에 연결해요. 미설정 시 panic — 통합
//! 테스트는 명시적인 `DB` 환경을 가정해요.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::missing_panics_doc,
    dead_code
)]
#![cfg(feature = "integration")]

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

/// 테스트용 `Postgres` `PgPool` 생성.
pub async fn setup_test_pool() -> PgPool {
    let url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests");
    PgPoolOptions::new()
        .max_connections(3)
        .connect(&url)
        .await
        .expect("connect to test Postgres")
}

/// 테스트 격리: 각 테스트 시작 전 모든 도메인 테이블 truncate.
///
/// `FK CASCADE` 활용 — `listing_photo` 가 `listing` 의 `FK on delete cascade` 라
/// `cascade` 옵션으로 한꺼번에 비워요. `audit_log` 는 `V002` immutable 트리거가
/// `UPDATE`/`DELETE` 만 차단하고 `TRUNCATE` (DDL) 는 통과해요.
pub async fn truncate_all(pool: &PgPool) {
    sqlx::query(r#"truncate "user", listing, listing_photo, audit_log, outbox_event cascade"#)
        .execute(pool)
        .await
        .expect("truncate failed");
}
