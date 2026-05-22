use std::path::Path;

use tracing::{info, instrument};

use super::super::spawn::{build_command, Arg, Host};
use super::{LayerKind, TippecanoeError};

/// tippecanoe 실행 설정.
#[derive(Debug, Clone)]
pub struct TippecanoeArgs<'a> {
    /// layer kind — zoom range / layer name 결정.
    pub kind: LayerKind,
    /// 입력 GeoJSON 파일들 (1개 이상).
    pub inputs: &'a [&'a Path],
    /// 출력 .pmtiles 경로.
    pub output: &'a Path,
}

/// tippecanoe 결과.
#[derive(Debug, Clone)]
pub struct TippecanoeResult {
    /// 출력 파일 크기 (bytes) — sanity 검증 (너무 작거나 크면 실패).
    pub output_bytes: u64,
    /// tippecanoe 가 `--metadata-json` 에 박제한 실제 feature 수.
    /// tippecanoe 버전이 해당 필드를 지원하지 않으면 `None`.
    pub feature_count: Option<u64>,
}

/// tippecanoe 실행. `args.inputs` 의 GeoJSON 들을 한 PMTiles 로 빌드.
///
/// flag 는 [`reference_flags`] 가 결정 — design-lab 의 검증된 셋과 동일.
///
/// # Errors
///
/// spawn 실패 / non-zero exit / output 미생성.
#[instrument(skip(host, args), fields(layer = %args.kind.layer_name(), output = %args.output.display()))]
pub async fn run(
    host: Host,
    args: &TippecanoeArgs<'_>,
) -> Result<TippecanoeResult, TippecanoeError> {
    if args.inputs.is_empty() {
        return Err(TippecanoeError::NoInputs);
    }

    let (min_z, max_z) = args.kind.zoom_range();
    let layer_name = args.kind.layer_name();
    let min_z_str = min_z.to_string();
    let max_z_str = max_z.to_string();

    let mut spawn_args: Vec<Arg<'_>> = vec![
        Arg::Lit("-o"),
        Arg::Path(args.output),
        Arg::Lit("-l"),
        Arg::Lit(layer_name),
        Arg::Lit("-P"),
        Arg::Lit("-Z"),
        Arg::Lit(&min_z_str),
        Arg::Lit("-z"),
        Arg::Lit(&max_z_str),
        // SSS 화 (사용자 needs: 폴리곤 망가짐/비틀림 0, 사라짐 0, 생긴거 그대로):
        // - simplification=1: 최소 simplification (default 12 → 1, epsilon ~2mm 수준).
        //   maxzoom 에서는 항상 0 — 정확히 원본 (tippecanoe invariant).
        // - coalesce-smallest-as-needed: 작은 polygon 'drop' → 'merge' (사라짐 0)
        // - detect-shared-borders: 인접 polygon boundary 정확히 일치 (틈 0, 겹침 0)
        // - no-tiny-polygon-reduction: 저줌 (z0-5) 에서도 작은 polygon 이 *점* 으로
        //   reduce 안 됨 — 산단 "모든 zoom 에서 visible" 요구사항.
        // - maximum-tile-bytes 4MB: detail 보존, default 500KB 보다 8x
        Arg::Lit("--no-feature-limit"),
        Arg::Lit("--no-tile-size-limit"),
        Arg::Lit("--no-tiny-polygon-reduction"),
        Arg::Lit("--force"),
        Arg::Lit("--coalesce-smallest-as-needed"),
        Arg::Lit("--detect-shared-borders"),
        Arg::Lit("--simplification=1"),
        Arg::Lit("--maximum-tile-bytes=4000000"),
        Arg::Lit("--extend-zooms-if-still-dropping"),
        Arg::Lit("--attribute-type=pnu:string"),
    ];
    for input in args.inputs {
        spawn_args.push(Arg::Path(input));
    }

    info!(
        inputs = args.inputs.len(),
        min_zoom = min_z,
        max_zoom = max_z,
        "tippecanoe starting"
    );

    // --metadata-json: tippecanoe (felt fork) 가 빌드 메타(feature_count) 를 별도 파일
    // 로 박제. SSOT SHA pin 의 tippecanoe 빌드만 본 flag 지원 — local dev WSL 의
    // 공식 tippecanoe 2.80 같은 변종은 미지원 (Round 5 verify smoke 발견).
    //
    // Capability detection — `--help` 출력에서 `--metadata-json` 존재 검사 후 추가.
    // 미지원 시 graceful skip — `feature_count` 가 `None` 으로 lineage 박제 (이미
    // honest absence semantic).
    let metadata_file = args.output.with_extension("tippecanoe-meta.json");
    let supports_metadata_json = check_supports_flag(host, "--metadata-json").await;
    if supports_metadata_json {
        spawn_args.push(Arg::Lit("--metadata-json"));
        // metadata_file_str 은 String 이라 Arg::Lit lifetime 불일치 — Arg::Path 사용.
        spawn_args.push(Arg::Path(&metadata_file));
    } else {
        info!("tippecanoe missing --metadata-json (likely upstream build, not felt fork) — feature_count will be None");
    }

    let mut cmd = build_command(host, "tippecanoe", &spawn_args)?;
    let output = cmd.output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // 마지막 4KB 만 — 큰 입력 시 stderr 가 매우 길 수 있음.
        let trimmed = if stderr.len() > 4096 {
            stderr[stderr.len() - 4096..].to_owned()
        } else {
            stderr.into_owned()
        };
        return Err(TippecanoeError::Failed {
            code: output.status.code().unwrap_or(-1),
            stderr: trimmed,
        });
    }

    // 검증 — 출력 파일 존재 + 크기.
    let meta =
        tokio::fs::metadata(args.output)
            .await
            .map_err(|_| TippecanoeError::OutputMissing {
                path: args.output.display().to_string(),
            })?;
    let output_bytes = meta.len();

    // --metadata-json 에서 feature_count 추출. capability detection 결과 false 면
    // metadata file 자체가 안 생성됨 → `extract_feature_count` 가 `None` 반환 (정상).
    let feature_count = if supports_metadata_json {
        extract_feature_count(&metadata_file).await
    } else {
        None
    };
    // 임시 메타파일 정리 — 실패해도 무시 (best-effort cleanup).
    let _ = tokio::fs::remove_file(&metadata_file).await;

    info!(bytes = output_bytes, ?feature_count, "tippecanoe complete");
    Ok(TippecanoeResult {
        output_bytes,
        feature_count,
    })
}

/// tippecanoe 가 특정 flag 를 지원하는지 검사. `--help` 출력에서 flag 문자열 grep.
///
/// Round 5 verify smoke 발견: SSOT SHA pin (felt fork) 의 tippecanoe 빌드는
/// `--metadata-json` 지원, local dev 환경의 official tippecanoe 2.80 은 미지원.
/// 본 helper 는 capability detection — production CI 에서는 항상 `true` (SSOT 빌드).
async fn check_supports_flag(host: Host, flag: &str) -> bool {
    let Ok(mut cmd) = build_command(host, "tippecanoe", &[Arg::Lit("--help")]) else {
        return false;
    };
    let Ok(output) = cmd.output().await else {
        return false;
    };
    // tippecanoe 의 --help 는 stderr 로 나가는 빌드도 있고 stdout 으로 나가는 빌드도 있음.
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    combined.contains(flag)
}

/// `--metadata-json` 파일에서 feature count 를 읽어 반환.
/// tippecanoe 가 `{"layers":[{"features": N}]}` 형태로 박제. 필드 부재 / 파싱 실패 = `None`.
async fn extract_feature_count(metadata_file: &std::path::Path) -> Option<u64> {
    let bytes = tokio::fs::read(metadata_file).await.ok()?;
    let v: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    // tippecanoe metadata schema: top-level `"features"` 또는 `"layers"[0]["features"]`.
    if let Some(n) = v.get("features").and_then(serde_json::Value::as_u64) {
        return Some(n);
    }
    // 일부 버전: `"vector_layers"` or `"tilestats" -> "layers"[0]["count"]`.
    if let Some(layers) = v
        .get("tilestats")
        .and_then(|ts| ts.get("layers"))
        .and_then(|l| l.as_array())
    {
        let total: u64 = layers
            .iter()
            .filter_map(|layer| layer.get("count").and_then(serde_json::Value::as_u64))
            .sum();
        if total > 0 {
            return Some(total);
        }
    }
    None
}
