# Rust 컨벤션

## 1. 도구

- **rustc**: 1.91.1 (`rust-toolchain.toml` 고정)
- **포맷**: rustfmt (`rustfmt.toml`)
- **lint**: clippy pedantic + nursery (`clippy.toml` + `Cargo.toml [workspace.lints]`)
- **공급망**: cargo-audit + cargo-deny (`deny.toml`)
- **테스트**: `cargo test` + insta + rstest + mockall

## 2. 포맷 (rustfmt.toml)

- max_width: 100
- tab_spaces: 4
- reorder_imports: true

`imports_granularity`, `group_imports` 는 stable rustfmt 에서 무시되는 nightly 전용 옵션이므로
`rustfmt.toml` 에 두지 않는다. import grouping 은 아래 §6 컨벤션을 따른다.

## 3. lint (clippy)

- 모든 워크스페이스 멤버에 `[lints] workspace = true`
- pedantic + nursery 기본 warn
- `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, `dbg_macro`, `print_stdout`, `print_stderr` = **deny**
- `unsafe_code = "forbid"` (전체 워크스페이스)

## 4. 도메인 패턴 (DDD)

### 값 객체 (Newtype)

```rust
// crates/domain/shared-kernel/src/pnu.rs
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Pnu(String);

impl Pnu {
    pub fn try_new(s: &str) -> Result<Self, PnuError> {
        if s.len() != 19 || !s.chars().all(|c| c.is_ascii_digit()) {
            return Err(PnuError::InvalidFormat);
        }
        Ok(Self(s.to_owned()))
    }
    pub fn as_str(&self) -> &str { &self.0 }
}
```

→ 모든 도메인 식별자 + 양/통화 = Newtype. 잘못된 값은 컴파일 시점 또는 생성 시점에 차단.

### Repository (Port)

```rust
// crates/domain/core/listing/src/repository.rs
#[async_trait::async_trait]
pub trait ListingRepository: Send + Sync {
    async fn find(&self, id: &ListingId) -> Result<Option<Listing>, RepoError>;
    async fn save(&self, listing: &Listing) -> Result<(), RepoError>;
}
```

구현체는 `crates/db/`에. 도메인은 trait만 알아야 함.

### 에러 처리

- 도메인 에러: `thiserror` (구체 enum)
- 앱 에러: `anyhow::Error` 또는 `eyre::Error` (서비스 레이어)
- 사용자 노출: `crates/api-types/error.rs` enum → RFC 9457 변환

```rust
#[derive(thiserror::Error, Debug)]
pub enum ListingError {
    #[error("Listing not found: {0}")]
    NotFound(ListingId),
    #[error("Invalid status transition: {from:?} -> {to:?}")]
    InvalidStatusTransition { from: ListingStatus, to: ListingStatus },
}
```

## 5. async / 동시성

- 런타임: `tokio` 풀 기능 (`features = ["full"]`)
- async trait: `async-trait` crate (Rust 1.83 기준 stable async fn in trait도 OK)
- `.await` 후 lock 보유 금지 (deadlock 위험)
- `Arc<Mutex<T>>`보다 actor 패턴 또는 channel 선호

## 6. import 순서

```rust
// std
use std::collections::HashMap;
use std::time::Duration;

// external
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

// crate
use crate::domain::Listing;
```

## 7. 금지 패턴

- ❌ `unwrap()`, `expect()` (테스트 외)
- ❌ `panic!()`, `todo!()`, `unimplemented!()`
- ❌ `dbg!()`, `println!()`, `eprintln!()` (대신 `tracing::*`)
- ❌ `unsafe` 블록 (`forbid` 정책)
- ❌ 글로벌 mutable state (`lazy_static`은 OK, `static mut`는 금지)
- ❌ TODO/HACK/XXX 코멘트 (대신 GitHub Issue + ADR)

## 8. 의존성 방향 (Cargo workspace)

```
apps/* → packages/*
services/* → crates/*
crates/domain/* → crates/shared-kernel만
crates/data-clients/* → crates/{circuit-breaker, observability, api-types}
crates/db → crates/{domain (ports만), api-types}
```

위반 시 cargo-arch (또는 자체 deps 룰) CI 차단.

## 9. 테스트

- 단위: `#[cfg(test)] mod tests` (같은 파일)
- 통합: `tests/` 폴더 (각 crate 안)
- 스냅샷: `insta`
- 픽스처: `rstest`
- mock: `mockall`
- DB: `sqlx::test` (자동 transaction rollback)

테스트 네이밍: → [testing.md](./testing.md)
