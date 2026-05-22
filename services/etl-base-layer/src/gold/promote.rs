//! Plan D **L3 Atomicity** — gold 빌드 / R2 PUT 와 *manifest publish* 분리.
//!
//! ## 문제 (이전 design)
//!
//! `gold` subcommand 가 layer 빌드 후 *바로* `gold/manifest.json` 으로 publish.
//! matrix 가 3 layer parallel 일 때 마지막 layer 빌드만 manifest 에 살아남음 → 부분
//! state 가 클라이언트에 노출 (e.g. parcels 빌드 실패 했는데 admin 빌드만 성공한
//! manifest 가 publish 되어 클라가 admin 만 fetch).
//!
//! ## 해결 (본 모듈)
//!
//! 1. **gold subcommand**: layer 별로 R2 의 `gold/<version>/<layer>/...` 에 flat tile
//!    PUT *후* `gold/staging/<version>/<layer>.lineage.json` 박제. manifest 미건드림.
//! 2. **promote subcommand** (신규, 본 모듈): 모든 layer 의 lineage 가 R2 staging 에
//!    존재하는지 검증 → 새 `GoldManifest` 빌드 → atomic PUT `gold/manifest.json` →
//!    Cloudflare CDN cache purge (manifest 만 — flat tile 은 immutable URL).
//! 3. 빌드 실패 시 staging buffer 만 남고 prod manifest 변경 0 — degrade gracefully.
//!
//! ## CDN cache purge
//!
//! `CLOUDFLARE_API_TOKEN` + `CLOUDFLARE_ZONE_ID` 환경변수 양쪽 set 시 활성. `gold/manifest.json`
//! 만 purge (flat tile 들은 immutable URL 이라 불필요). 둘 중 하나 미설정 시 warn skip
//! — manifest 의 `Cache-Control: no-cache, max-age=0` 가 fallback (CDN 가 next-fetch 에서
//! revalidate, 분 단위 staleness 가능 — purge 하면 즉시).

#![allow(clippy::doc_markdown)]
#![cfg_attr(test, allow(dead_code))]

#[cfg(test)]
mod cdn;
#[cfg(test)]
mod cleanup;
mod error;
#[cfg(test)]
mod run;
mod staging;
#[cfg(test)]
mod tests;
mod types;

#[cfg(test)]
use super::manifest::BuildLineage;

#[cfg(test)]
use cdn::{cloudflare_purge, preflight_cdn_config, CdnPurgeOutcome};
#[cfg(test)]
use cleanup::cleanup_manifest_backups;

pub use error::PromoteError;
#[cfg(test)]
#[allow(unused_imports)]
pub use run::{run, PromoteArgs, PromoteResult};
pub use staging::write_staging_spec;
pub use types::ArtifactSpec;
