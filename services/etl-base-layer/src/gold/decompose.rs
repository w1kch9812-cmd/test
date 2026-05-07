//! PMTiles 단일 파일 → flat `{z}/{x}/{y}.pbf` 디렉터리 분해 (ADR 0021).
//!
//! 본 단계가 X9 의 핵심 — PMTiles 단일 파일을 R2 의 flat tile directory 로 분해해서
//! mapbox-gl 의 가장 표준 source (`type:"vector" + tiles:[URL_TEMPLATE]`) 가 직결.
//! addSourceType / Service Worker / Blob URL / private API 의존 0.
//!
//! 구현:
//! - felt fork [`tile-join`] 이 `.pmtiles` 입력 + `--output-to-directory` 지원 (검증된 spec).
//! - `--no-tile-stats` 로 metadata.json 안의 tilestats 생략 (CDN 측 trivial cleanup).
//! - 출력: `<out_dir>/{z}/{x}/{y}.pbf` 플랫 구조.
//!
//! 다음 단계 (별도 모듈) — R2 upload 가 본 출력 디렉터리를 walk + batch PutObject.
//!
//! [`tile-join`]: https://github.com/felt/tippecanoe

#![allow(clippy::doc_markdown)]

use std::path::Path;

use thiserror::Error;
use tracing::{info, instrument};
use walkdir::WalkDir;

use super::spawn::{build_command, Arg, Host, SpawnError};

/// `tile-join --output-to-directory` 실행 설정.
#[derive(Debug, Clone)]
pub struct DecomposeArgs<'a> {
    /// 입력 `<layer>.pmtiles` 경로.
    pub input: &'a Path,
    /// 출력 디렉터리. 안에 `{z}/{x}/{y}.pbf` 생성됨. *비어있어야 함* — tile-join 이
    /// 이미 존재하는 파일은 거부.
    pub output_dir: &'a Path,
}

/// decompose 결과 — R2 upload step 이 walk 할 때 sanity 비교 용.
#[derive(Debug, Clone)]
pub struct DecomposeResult {
    /// 생성된 .pbf 타일 파일 수.
    pub tile_count: u64,
    /// .pbf 파일 총 bytes.
    pub total_bytes: u64,
}

/// decompose 에러.
#[derive(Debug, Error)]
pub enum DecomposeError {
    /// command 빌드 실패.
    #[error("spawn build failed: {0}")]
    Build(#[from] SpawnError),
    /// spawn / wait / I/O.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// non-zero exit code.
    #[error("tile-join exited with {code}: {stderr}")]
    Failed {
        /// exit code.
        code: i32,
        /// stderr 마지막 4KB.
        stderr: String,
    },
    /// 출력 디렉터리에 .pbf 파일이 0개 — tile-join silent fail.
    #[error("output directory {path} has no .pbf files")]
    NoTilesProduced {
        /// 검증한 디렉터리.
        path: String,
    },
    /// 출력 디렉터리가 이미 존재 + 비어있지 않음 (`tile-join` 이 거부할 가능성).
    #[error("output directory {path} already exists and is not empty")]
    OutputDirNotEmpty {
        /// 충돌 디렉터리.
        path: String,
    },
}

/// `tile-join` 실행 = `<input>.pmtiles` → `<output_dir>/{z}/{x}/{y}.pbf`.
///
/// # Errors
///
/// 출력 디렉터리 충돌 / spawn 실패 / non-zero exit / 산출물 0개.
#[instrument(
    skip(host, args),
    fields(
        input = %args.input.display(),
        output_dir = %args.output_dir.display(),
    ),
)]
pub async fn run(host: Host, args: &DecomposeArgs<'_>) -> Result<DecomposeResult, DecomposeError> {
    // 출력 디렉터리는 *존재하지 않거나 비어있어야* tile-join 이 진행. 호출자가 이전
    // 빌드 산출물을 정리해야 함 — 여기서는 *비어있는지* 만 검증 (실수 방지).
    if args.output_dir.exists() {
        let mut entries = tokio::fs::read_dir(args.output_dir).await?;
        if entries.next_entry().await?.is_some() {
            return Err(DecomposeError::OutputDirNotEmpty {
                path: args.output_dir.display().to_string(),
            });
        }
    } else {
        tokio::fs::create_dir_all(args.output_dir).await?;
    }

    let cmd_args = [
        Arg::Lit("--no-tile-stats"),
        Arg::Lit("--output-to-directory"),
        Arg::Path(args.output_dir),
        Arg::Path(args.input),
    ];
    let mut cmd = build_command(host, "tile-join", &cmd_args)?;

    info!("spawn tile-join (decompose)");

    let output = cmd.output().await?;

    if !output.status.success() {
        let code = output.status.code().unwrap_or(-1);
        let stderr_full = String::from_utf8_lossy(&output.stderr).into_owned();
        let stderr_tail = if stderr_full.len() > 4096 {
            stderr_full[(stderr_full.len() - 4096)..].to_owned()
        } else {
            stderr_full
        };
        return Err(DecomposeError::Failed {
            code,
            stderr: stderr_tail,
        });
    }

    // walk 로 .pbf 산출물 수 + bytes 합산. 0개면 silent fail 로 간주.
    let (tile_count, total_bytes) = count_pbf_files(args.output_dir)?;
    if tile_count == 0 {
        return Err(DecomposeError::NoTilesProduced {
            path: args.output_dir.display().to_string(),
        });
    }

    info!(tile_count, total_bytes, "decompose complete");

    Ok(DecomposeResult {
        tile_count,
        total_bytes,
    })
}

/// `<root>/{z}/{x}/{y}.pbf` 재귀 walk → (count, total_bytes).
fn count_pbf_files(root: &Path) -> std::io::Result<(u64, u64)> {
    let mut count: u64 = 0;
    let mut bytes: u64 = 0;
    for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("pbf") {
            continue;
        }
        let meta = entry.metadata()?;
        bytes = bytes.saturating_add(meta.len());
        count = count.saturating_add(1);
    }
    Ok((count, bytes))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[tokio::test]
    async fn output_dir_with_existing_file_returns_err() {
        let dir = std::env::temp_dir().join("etl_decompose_nonempty");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("dummy.txt"), b"hello")
            .await
            .unwrap();

        let input = dir.join("ignored.pmtiles");
        let args = DecomposeArgs {
            input: &input,
            output_dir: &dir,
        };
        let err = run(Host::Native, &args).await.unwrap_err();
        assert!(matches!(err, DecomposeError::OutputDirNotEmpty { .. }));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[test]
    fn count_pbf_walks_recursive_z_x_y_layout() {
        let root = std::env::temp_dir().join("etl_decompose_walk");
        let _ = std::fs::remove_dir_all(&root);
        let z14_x100 = root.join("14").join("100");
        std::fs::create_dir_all(&z14_x100).unwrap();
        std::fs::write(z14_x100.join("200.pbf"), b"abc").unwrap();
        std::fs::write(z14_x100.join("201.pbf"), b"defg").unwrap();
        // .pbf 가 아닌 파일은 카운트 X.
        std::fs::write(z14_x100.join("metadata.json"), b"{}").unwrap();

        let (count, bytes) = count_pbf_files(&root).unwrap();
        assert_eq!(count, 2);
        assert_eq!(bytes, 7);

        let _ = std::fs::remove_dir_all(&root);
    }
}
