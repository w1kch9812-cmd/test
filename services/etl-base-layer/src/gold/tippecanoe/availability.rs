use super::super::spawn::{build_command, Arg, Host};
use super::TippecanoeError;

/// tippecanoe binary 가 실행 가능한지 빠르게 검사 (`--version`).
///
/// 환경 점검용 — 실 빌드 직전 호출하면 친절한 에러 가능.
///
/// Round 5+ (Codex audit): SSOT `TIPPECANOE_GIT_SHA` 와 *실제 설치 SHA* 비교 검사.
/// dev WSL 환경에서 `scripts/setup-dev-tippecanoe.sh` 가 박제한 `.sp9-tippecanoe-sha`
/// 파일 검사 → mismatch 시 warning 로그 (production CI 는 workflow 가 직접 SHA pin
/// 빌드라 본 검사 skip 자연 통과).
///
/// # Errors
///
/// spawn 실패 / non-zero exit.
pub async fn check_available(host: Host) -> Result<String, TippecanoeError> {
    let mut cmd = build_command(host, "tippecanoe", &[Arg::Lit("--version")])?;
    let output = cmd.output().await?;
    if !output.status.success() {
        return Err(TippecanoeError::Failed {
            code: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }
    // tippecanoe 는 --version 을 stderr 로 출력하기도 함 — 양쪽 합쳐서 반환.
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    // SSOT SHA 검사 (best-effort). 미일치 시 warning — local dev 의 capability
    // detection trick 완전 제거 path 의 첫 단계. ADR 0028 + 0029 후속.
    check_ssot_sha();

    Ok(combined.trim().to_owned())
}

/// SSOT `TIPPECANOE_GIT_SHA` 와 dev tooling 박제 SHA 비교. mismatch 시 warning.
///
/// 박제 파일 (`/usr/local/bin/.sp9-tippecanoe-sha`) 는 `scripts/setup-dev-tippecanoe.sh`
/// 가 생성. 파일 부재 = dev 환경이 SSOT 스크립트 미실행 (예: upstream tippecanoe 직접
/// apt install). 이 경우도 warning — operator 가 setup script 실행하도록 유도.
fn check_ssot_sha() {
    const SHA_FILE: &str = "/usr/local/bin/.sp9-tippecanoe-sha";
    let Some(installed) = read_installed_tippecanoe_sha(SHA_FILE) else {
        log_missing_tippecanoe_sha_file(SHA_FILE);
        return;
    };

    log_tippecanoe_sha_status(&installed);
}

fn read_installed_tippecanoe_sha(path: &str) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|raw| raw.trim().to_owned())
}

fn log_missing_tippecanoe_sha_file(path: &str) {
    // 파일 부재 — dev 환경 미설정 가능성. CI 는 workflow 가 직접 SHA 빌드라
    // 본 파일 없음. 즉 trace 만 debug level (CI noise 회피).
    tracing::debug!(
        sha_file = path,
        ssot_sha = sp9_base_layer_config::TIPPECANOE_GIT_SHA,
        "tippecanoe SHA file 부재 — CI/Container 환경이면 정상, dev 환경이면 scripts/setup-dev-tippecanoe.sh 실행 권장"
    );
}

fn log_tippecanoe_sha_status(installed: &str) {
    if is_tippecanoe_sha_current(installed) {
        log_tippecanoe_sha_match(installed);
        return;
    }

    log_tippecanoe_sha_mismatch(installed);
}

fn is_tippecanoe_sha_current(installed: &str) -> bool {
    installed == sp9_base_layer_config::TIPPECANOE_GIT_SHA
}

fn log_tippecanoe_sha_match(installed: &str) {
    tracing::info!(sha = installed, "tippecanoe SHA matches SSOT");
}

fn log_tippecanoe_sha_mismatch(installed: &str) {
    tracing::warn!(
        installed,
        ssot = sp9_base_layer_config::TIPPECANOE_GIT_SHA,
        "tippecanoe SHA mismatch with SSOT — capability drift 위험. 실행: scripts/setup-dev-tippecanoe.sh"
    );
}
