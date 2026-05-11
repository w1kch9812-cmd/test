//! Test 전용 공유 utility — env mutation 직렬화.
//!
//! `cargo test` 의 multi-thread 환경에서 `std::env::set_var` / `remove_var` 가 process
//! global state 라 다른 모듈의 test 와 race. crate-wide `GLOBAL_ENV_LOCK` 으로 직렬화.
//!
//! 사용:
//! ```ignore
//! let _g = crate::test_support::GLOBAL_ENV_LOCK.lock().expect("env mutex");
//! std::env::set_var("FOO", "bar");
//! ```

use std::sync::Mutex;

/// Crate 전체에서 공유되는 env mutation lock. config.rs / promote.rs / 기타 env-mutating
/// test 가 *반드시* 본 mutex 를 hold 한 채 env 만지기.
pub static GLOBAL_ENV_LOCK: Mutex<()> = Mutex::new(());
