//! 외부 binary spawn helper — Linux 직접 호출 / Windows → WSL pass-through.
//!
//! 동기:
//! - CI Ubuntu 22.04 large 에서는 `apt install gdal-bin` + tippecanoe 빌드 → PATH 에 직접.
//! - Dev Windows 에서는 GDAL/tippecanoe 가 보통 WSL Ubuntu 안에만 있음 →
//!   `wsl.exe -d Ubuntu -- ogr2ogr ...` 패턴.
//!
//! [`build_command`] 가 호스트 OS 자동 감지 + 경로 변환:
//! - `c:\Users\foo\bar.shp` → `/mnt/c/Users/foo/bar.shp` (WSL 가 마운트한 형식)
//!
//! 본 helper 는 한 번 호출 = 한 spawn. exit status / stdout / stderr 는 호출자가 처리.

use std::path::Path;

use thiserror::Error;
use tokio::process::Command;
use tracing::debug;

/// spawn 실행 환경. `WslPassThrough` 는 dev Windows + WSL Ubuntu 조합 전용.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Host {
    /// Linux/macOS 등 — binary 가 PATH 에 직접 존재.
    Native,
    /// Windows — `wsl.exe -d <distro> --` 으로 pass-through.
    WslPassThrough,
}

impl Host {
    /// 현재 OS 기반 자동 결정. `cfg(windows)` → `WslPassThrough`, 그 외 → `Native`.
    #[must_use]
    pub const fn detect() -> Self {
        if cfg!(windows) {
            Self::WslPassThrough
        } else {
            Self::Native
        }
    }
}

/// WSL distro 이름 — 환경변수 `ETL_WSL_DISTRO` 우선, 미설정 시 `Ubuntu`.
fn wsl_distro() -> String {
    std::env::var("ETL_WSL_DISTRO").unwrap_or_else(|_| "Ubuntu".to_owned())
}

/// 인자 종류 — string 그대로 전달할지 path 변환할지 구분.
#[derive(Debug, Clone)]
pub enum Arg<'a> {
    /// 일반 옵션/리터럴 (예: `-o`, `EPSG:4326`).
    Lit(&'a str),
    /// 파일 / 디렉터리 경로 — `WslPassThrough` 일 때 자동 변환.
    Path(&'a Path),
}

/// `c:\Users\foo` → `/mnt/c/Users/foo` 변환 (WSL 마운트 표준).
///
/// 비-Windows 경로 (예: `/tmp/xxx`) 는 그대로 반환. UNC 경로 (`\\server\share`)
/// 는 변환 안 함 (WSL 마운트 형식이 다양 — `/mnt/wsl/...` 등). 호출자가 ASCII 경로
/// 만 쓰는 책임.
fn windows_to_wsl_path(p: &Path) -> String {
    let s = p.to_string_lossy().replace('\\', "/");
    let bytes = s.as_bytes();
    // `<letter>:` prefix 검출 — 영문 알파벳 + 콜론.
    if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
        let drive = bytes[0].to_ascii_lowercase() as char;
        // `<letter>:` 이후 부분 (e.g. `/Users/foo`).
        let rest = &s[2..];
        format!("/mnt/{drive}{rest}")
    } else {
        s
    }
}

/// Spawn helper 에러 — 명령 빌드 단계 (실 spawn 은 [`tokio::process::Command`] 가 담당).
#[derive(Debug, Error)]
pub enum SpawnError {
    /// 빈 program 이름.
    #[error("program name must not be empty")]
    EmptyProgram,
}

/// Host 에 맞춰 [`Command`] 빌드.
///
/// `Native`: `Command::new(program).args(args_str)`.
/// `WslPassThrough`: `Command::new("wsl.exe").args(["-d", <distro>, "--", program, ...args_str])`.
///
/// `Arg::Path` 는 `WslPassThrough` 에서 `/mnt/c/...` 로 변환 후 전달.
///
/// # Errors
///
/// `program` 이 빈 문자열.
pub fn build_command(host: Host, program: &str, args: &[Arg<'_>]) -> Result<Command, SpawnError> {
    if program.is_empty() {
        return Err(SpawnError::EmptyProgram);
    }
    let mut cmd = match host {
        Host::Native => {
            let mut c = Command::new(program);
            for a in args {
                match a {
                    Arg::Lit(s) => {
                        c.arg(s);
                    }
                    Arg::Path(p) => {
                        c.arg(p);
                    }
                }
            }
            c
        }
        Host::WslPassThrough => {
            let distro = wsl_distro();
            let mut c = Command::new("wsl.exe");
            c.arg("-d").arg(&distro).arg("--").arg(program);
            for a in args {
                match a {
                    Arg::Lit(s) => {
                        c.arg(s);
                    }
                    Arg::Path(p) => {
                        c.arg(windows_to_wsl_path(p));
                    }
                }
            }
            c
        }
    };
    cmd.kill_on_drop(true);
    debug!(?host, program, "spawn command built");
    Ok(cmd)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn windows_path_to_wsl_mount() {
        let p = PathBuf::from(r"C:\Users\foo\bar.shp");
        assert_eq!(windows_to_wsl_path(&p), "/mnt/c/Users/foo/bar.shp");
    }

    #[test]
    fn lowercase_drive_translates() {
        let p = PathBuf::from(r"c:\tmp\x.geojson");
        assert_eq!(windows_to_wsl_path(&p), "/mnt/c/tmp/x.geojson");
    }

    #[test]
    fn linux_path_passthrough() {
        let p = PathBuf::from("/tmp/foo.geojson");
        assert_eq!(windows_to_wsl_path(&p), "/tmp/foo.geojson");
    }

    #[test]
    fn empty_program_rejected() {
        let err = build_command(Host::Native, "", &[]).unwrap_err();
        assert!(matches!(err, SpawnError::EmptyProgram));
    }

    #[test]
    fn native_command_uses_program_directly() {
        let cmd = build_command(Host::Native, "ogr2ogr", &[Arg::Lit("--version")]).unwrap();
        let std_cmd = cmd.as_std();
        assert_eq!(
            std_cmd.get_program().to_string_lossy(),
            "ogr2ogr",
            "Native should call binary directly"
        );
        let args: Vec<&std::ffi::OsStr> = std_cmd.get_args().collect();
        assert_eq!(args, vec![std::ffi::OsStr::new("--version")]);
    }

    #[test]
    fn wsl_command_prepends_wsl_invocation() {
        let path = PathBuf::from(r"C:\tmp\x.geojson");
        let cmd = build_command(
            Host::WslPassThrough,
            "tippecanoe",
            &[Arg::Lit("-o"), Arg::Path(&path)],
        )
        .unwrap();
        let std_cmd = cmd.as_std();
        assert_eq!(std_cmd.get_program().to_string_lossy(), "wsl.exe");
        let args: Vec<String> = std_cmd
            .get_args()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        assert_eq!(args[0], "-d");
        // distro name from env (default "Ubuntu")
        assert_eq!(args[2], "--");
        assert_eq!(args[3], "tippecanoe");
        assert_eq!(args[4], "-o");
        assert_eq!(args[5], "/mnt/c/tmp/x.geojson");
    }
}
