//! Plan D **L2 Verification** — Gold PMTiles 빌드 후 결정성/정합성 검증.
//!
//! 사용자 명시 invariant: "강남 PNU `1168010100107370000` 가 출력에 *반드시* 존재".
//! 추가 invariant (SSS-grade prod):
//! - sha256 (lineage 박제 + 결정성 검증)
//! - file size sanity (너무 작으면 build silent fail 의심)
//! - tile-level 스팟 체크 (특정 (z,x,y) 의 PNU substring 존재)
//!
//! ## 비교 대안 — 왜 spot-check?
//!
//! - 전체 decode (`tippecanoe-decode <pmtiles>`): 전국 parcels 의 경우 1.4B feature
//!   = 수 TB JSON → 비현실. 한 시군구 smoke 에서는 ~100MB → 가능하지만 일관 path X.
//! - `pmtiles-rs` crate 으로 header 읽기: 의존성 추가 (alpha 단계). 본 commit 에선 spawn 만.
//! - **본 모듈 spot-check**: `tippecanoe-decode <pmtiles> <z> <x> <y>` — 특정 tile 만
//!   디코드 (~10KB JSON), 그 안에 expected PNU substring 존재 확인. 빠름 + 일관.
//!
//! ## 호출자
//!
//! `run_gold` 가 build 직후 환경변수 `VERIFY_*` 로 invariant 명시 시 호출.

#![allow(dead_code, clippy::doc_markdown)]

use std::path::Path;

use sha2::{Digest, Sha256};
use thiserror::Error;
use tracing::{info, instrument, warn};

use super::spawn::{build_command, Arg, Host, SpawnError};

/// 단일 tile 의 expected invariant — `(z,x,y)` 에 모든 `must_contain` substring 등장.
#[derive(Debug, Clone)]
pub struct TileSpec {
    /// zoom level. 통상 maxzoom (parcels=17, complex=16) 권장 — simplification 0 보장.
    pub z: u8,
    /// tile x.
    pub x: u32,
    /// tile y.
    pub y: u32,
    /// 해당 tile 의 decoded JSON 안에 *반드시* 등장해야 할 substring 들.
    pub must_contain: Vec<String>,
}

/// Verify 입력.
#[derive(Debug, Clone)]
pub struct VerifySpec<'a> {
    /// 검증 대상 PMTiles 파일.
    pub pmtiles: &'a Path,
    /// PMTiles 안 layer 이름 (`tippecanoe-decode -l <name>`).
    pub layer_name: &'a str,
    /// 파일이 *최소* 이 바이트 이상이어야 함 — 빌드 silent-fail 감지.
    pub min_file_bytes: u64,
    /// 모든 `TileSpec` 가 통과해야 함.
    pub tile_specs: &'a [TileSpec],
}

/// Verify 결과.
#[derive(Debug, Clone)]
pub struct VerifyResult {
    /// 입력 PMTiles 의 SHA-256 hex digest — manifest lineage 박제 (L10).
    pub sha256: String,
    /// 파일 size (bytes).
    pub file_bytes: u64,
    /// 통과한 `TileSpec` 수 (= `spec.tile_specs.len()`).
    pub tiles_passed: u64,
}

/// Verify 에러.
#[derive(Debug, Error)]
pub enum VerifyError {
    /// command 빌드 (`build_command`).
    #[error("spawn build: {0}")]
    Build(#[from] SpawnError),
    /// I/O — 파일 읽기 / `tippecanoe-decode` 실행.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// PMTiles 파일이 너무 작음 (silent build fail 의심).
    #[error("file size {actual} < expected min {expected}: {path}")]
    TooSmall {
        /// 실제 size.
        actual: u64,
        /// 기대 최소.
        expected: u64,
        /// 대상 path.
        path: String,
    },
    /// `tippecanoe-decode` non-zero exit.
    #[error("tippecanoe-decode {code}: {stderr}")]
    DecodeFailed {
        /// exit code.
        code: i32,
        /// stderr 마지막 4KB.
        stderr: String,
    },
    /// 특정 tile 에 expected substring 없음.
    #[error("tile {z}/{x}/{y}: missing expected substring {missing:?}")]
    MissingSubstring {
        /// zoom.
        z: u8,
        /// tile x.
        x: u32,
        /// tile y.
        y: u32,
        /// 빠진 substring (첫 번째).
        missing: String,
    },
}

/// PMTiles 파일의 SHA-256 계산 (streaming — 큰 파일도 메모리 적재 0).
///
/// # Errors
///
/// I/O.
pub async fn compute_sha256(path: &Path) -> Result<String, VerifyError> {
    use tokio::io::AsyncReadExt;
    let mut file = tokio::fs::File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// 단일 tile 디코드 + substring 체크.
///
/// `tippecanoe-decode -l <layer> <pmtiles> <z> <x> <y>` → stdout = GeoJSON Feature 들의
/// FeatureCollection. 본 함수는 stdout 전체를 메모리에 로드 (한 tile 은 ~10KB ~ 수MB).
async fn check_tile(
    host: Host,
    pmtiles: &Path,
    layer_name: &str,
    spec: &TileSpec,
) -> Result<(), VerifyError> {
    let z = spec.z.to_string();
    let x = spec.x.to_string();
    let y = spec.y.to_string();
    let mut cmd = build_command(
        host,
        "tippecanoe-decode",
        &[
            Arg::Lit("-l"),
            Arg::Lit(layer_name),
            Arg::Path(pmtiles),
            Arg::Lit(&z),
            Arg::Lit(&x),
            Arg::Lit(&y),
        ],
    )?;
    let output = cmd.output().await?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let trimmed = if stderr.len() > 4096 {
            stderr[stderr.len() - 4096..].to_owned()
        } else {
            stderr.into_owned()
        };
        return Err(VerifyError::DecodeFailed {
            code: output.status.code().unwrap_or(-1),
            stderr: trimmed,
        });
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    for needle in &spec.must_contain {
        if !stdout.contains(needle.as_str()) {
            return Err(VerifyError::MissingSubstring {
                z: spec.z,
                x: spec.x,
                y: spec.y,
                missing: needle.clone(),
            });
        }
    }
    Ok(())
}

/// 전체 verify 파이프라인 — sha256 + file size + tile 들의 substring 체크.
///
/// # Errors
///
/// 파일 너무 작음 / decode 실패 / substring 누락.
#[instrument(skip(host, spec), fields(pmtiles = %spec.pmtiles.display(), tile_specs = spec.tile_specs.len()))]
pub async fn run(host: Host, spec: &VerifySpec<'_>) -> Result<VerifyResult, VerifyError> {
    // 1. File size sanity.
    let meta = tokio::fs::metadata(spec.pmtiles).await?;
    let file_bytes = meta.len();
    if file_bytes < spec.min_file_bytes {
        return Err(VerifyError::TooSmall {
            actual: file_bytes,
            expected: spec.min_file_bytes,
            path: spec.pmtiles.display().to_string(),
        });
    }
    info!(file_bytes, "file size sanity OK");

    // 2. SHA-256.
    let sha256 = compute_sha256(spec.pmtiles).await?;
    info!(sha256 = %sha256, "sha256 computed");

    // 3. Tile spot-checks.
    for ts in spec.tile_specs {
        check_tile(host, spec.pmtiles, spec.layer_name, ts).await?;
        info!(z = ts.z, x = ts.x, y = ts.y, "tile spot-check OK");
    }
    if spec.tile_specs.is_empty() {
        warn!("no tile_specs — skipping spot-check (consider providing 강남 PNU tile)");
    }

    Ok(VerifyResult {
        sha256,
        file_bytes,
        tiles_passed: spec.tile_specs.len() as u64,
    })
}

/// WGS84 lon/lat → web mercator tile (z, x, y). 강남 PNU 같은 known-coord 의 tile 계산용.
///
/// 표준 mercator 변환식. y 는 latitude 가 고위도일수록 작은 값.
///
/// # Panics
///
/// `z > 31` (실용 zoom 0-22).
#[must_use]
// 후속 cast 들은 clamp(0, n-1) 후 = `[0, 2^31 - 1]` 보장 → u32 안전.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn lonlat_to_tile(lon: f64, lat: f64, z: u8) -> (u32, u32) {
    assert!(z <= 31, "z must be <= 31");
    let n = 2_f64.powi(i32::from(z));
    let x = ((lon + 180.0) / 360.0 * n).floor();
    let lat_rad = lat.to_radians();
    let y = ((1.0 - lat_rad.tan().asinh() / std::f64::consts::PI) / 2.0 * n).floor();
    let x_clamped = x.clamp(0.0, n - 1.0) as u32;
    let y_clamped = y.clamp(0.0, n - 1.0) as u32;
    (x_clamped, y_clamped)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;
    use std::io::Write;

    #[tokio::test]
    async fn sha256_matches_known_value() {
        let mut tmp = tempfile::NamedTempFile::new().expect("tempfile");
        tmp.write_all(b"hello world").expect("write");
        // sha256 of "hello world" = b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9.
        let sha = compute_sha256(tmp.path()).await.expect("sha");
        assert_eq!(
            sha,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[tokio::test]
    async fn too_small_returns_error() {
        let mut tmp = tempfile::NamedTempFile::new().expect("tempfile");
        tmp.write_all(b"tiny").expect("write");
        let spec = VerifySpec {
            pmtiles: tmp.path(),
            layer_name: "parcels",
            min_file_bytes: 1024,
            tile_specs: &[],
        };
        let err = run(Host::Native, &spec).await.unwrap_err();
        assert!(matches!(err, VerifyError::TooSmall { actual: 4, .. }));
    }

    #[test]
    fn gangnam_z14_tile_coords() {
        // 강남 PNU 1168010100107370000 → 위도/경도 기준점 ~127.04, 37.51.
        // z14 tile = (13973, 6347) (web mercator).
        let (x, y) = lonlat_to_tile(127.04, 37.51, 14);
        assert_eq!(x, 13973, "z14 x for 강남");
        assert_eq!(y, 6347, "z14 y for 강남");
    }

    #[test]
    fn gangnam_z17_tile_coords() {
        // z17 = z14 × 8.  강남 (127.04, 37.51) → (~111789, ~50783).
        let (x, y) = lonlat_to_tile(127.04, 37.51, 17);
        // 정확 값 — 17 은 maxzoom 권장.
        assert_eq!(x, 111_789, "z17 x for 강남");
        assert_eq!(y, 50_783, "z17 y for 강남");
    }

    #[test]
    fn lonlat_zero_is_origin() {
        // (0, 0) at z0 → (0, 0).
        let (x, y) = lonlat_to_tile(0.0, 0.0, 0);
        assert_eq!((x, y), (0, 0));
    }
}
