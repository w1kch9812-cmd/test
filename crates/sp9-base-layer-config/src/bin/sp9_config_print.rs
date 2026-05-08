//! `sp9-config-print` — SP9 SSOT 를 다양한 format 으로 stdout 출력.
//!
//! 본 binary 는 SSOT 의 *외부 소비자* (workflow YAML / Dockerfile build args /
//! shell scripts) 가 Rust const 를 읽기 위한 단일 게이트웨이.
//!
//! ## 사용법
//!
//! ```sh
//! # 모든 const → JSON (jq 친화적)
//! cargo run -q -p sp9-base-layer-config --bin sp9-config-print -- json
//!
//! # 모든 const → KEY=VALUE shell-eval format
//! cargo run -q -p sp9-base-layer-config --bin sp9-config-print -- env
//!
//! # 특정 키만 (workflow YAML build-arg 용)
//! cargo run -q -p sp9-base-layer-config --bin sp9-config-print -- key tippecanoe_version
//!
//! # layer 목록 JSON 배열 (etl.yml matrix 동적 생성)
//! cargo run -q -p sp9-base-layer-config --bin sp9-config-print -- layers
//! ```

#![forbid(unsafe_code)]
// CLI binary — println / eprintln 명시적 출력 + arg parse failure 시 panic 정답.
#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::expect_used,
    clippy::unwrap_used
)]

use std::process::ExitCode;

use sp9_base_layer_config::{ConfigSnapshot, Layer};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map_or("json", String::as_str);
    let snapshot = ConfigSnapshot::current();

    match cmd {
        "json" => {
            println!(
                "{}",
                serde_json::to_string_pretty(&snapshot).expect("json serialize")
            );
            ExitCode::SUCCESS
        }
        "env" => {
            // shell-eval 형식 — quoting 필요 없는 단순 값들 (string 안에 공백 / quote X).
            // 향후 값에 공백 들어가면 본 함수에서 escape 추가.
            println!("TIPPECANOE_VERSION={}", snapshot.tippecanoe_version);
            println!("TIPPECANOE_GIT_SHA={}", snapshot.tippecanoe_git_sha);
            println!("GDAL_VERSION_PIN={}", snapshot.gdal_version_pin);
            println!("RUST_TOOLCHAIN_VERSION={}", snapshot.rust_toolchain_version);
            println!("DTMK_DS_ID={}", snapshot.dtmk_ds_id);
            println!("SOURCE_SRS_VWORLD={}", snapshot.source_srs_vworld);
            println!("TARGET_SRS_WEB={}", snapshot.target_srs_web);
            println!(
                "DTMK_DOWNLOAD_CONCURRENCY={}",
                snapshot.dtmk_download_concurrency
            );
            println!(
                "NATIONWIDE_PMTILES_MIN_BYTES={}",
                snapshot.nationwide_pmtiles_min_bytes
            );
            ExitCode::SUCCESS
        }
        "key" => {
            let Some(name) = args.get(1) else {
                eprintln!("usage: sp9-config-print key <name>");
                return ExitCode::from(2);
            };
            let value = match name.as_str() {
                "tippecanoe_version" => snapshot.tippecanoe_version,
                "tippecanoe_git_sha" => snapshot.tippecanoe_git_sha,
                "gdal_version_pin" => snapshot.gdal_version_pin,
                "rust_toolchain_version" => snapshot.rust_toolchain_version,
                "dtmk_ds_id" => snapshot.dtmk_ds_id.to_string(),
                "source_srs_vworld" => snapshot.source_srs_vworld,
                "target_srs_web" => snapshot.target_srs_web,
                "dtmk_download_concurrency" => snapshot.dtmk_download_concurrency.to_string(),
                "nationwide_pmtiles_min_bytes" => snapshot.nationwide_pmtiles_min_bytes.to_string(),
                "dtmk_license" => snapshot.dtmk_license,
                "dtmk_source_url" => snapshot.dtmk_source_url,
                other => {
                    eprintln!("unknown key: {other}");
                    return ExitCode::from(2);
                }
            };
            println!("{value}");
            ExitCode::SUCCESS
        }
        "layers" => {
            // etl.yml matrix 의 fromJson 입력 — JSON array of strings.
            let names: Vec<&str> = Layer::ALL.iter().map(|l| l.name()).collect();
            println!("{}", serde_json::to_string(&names).expect("json"));
            ExitCode::SUCCESS
        }
        "active_layers" => {
            // Round 4 #2 — workflow matrix 가 *현재 ETL build-active* layer 만 소비.
            // `Layer::is_active_in_etl()` SSOT. admin/complex 는 source 미준비 →
            // matrix 에서 제외 (silent partial build 차단). ADR 0027 박제.
            let names: Vec<&str> = Layer::ALL
                .iter()
                .filter(|l| l.is_active_in_etl())
                .map(|l| l.name())
                .collect();
            println!("{}", serde_json::to_string(&names).expect("json"));
            ExitCode::SUCCESS
        }
        "landmarks" => {
            println!(
                "{}",
                serde_json::to_string(&snapshot.verify_landmarks).expect("json")
            );
            ExitCode::SUCCESS
        }
        other => {
            eprintln!(
                "unknown subcommand `{other}` — use json | env | key <name> | layers | active_layers | landmarks"
            );
            ExitCode::from(2)
        }
    }
}
