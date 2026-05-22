use thiserror::Error;

use super::super::spawn::SpawnError;

/// tippecanoe 에러.
#[derive(Debug, Error)]
pub enum TippecanoeError {
    /// command 빌드 단계 (program 이름 비어있음 등).
    #[error("spawn build failed: {0}")]
    Build(#[from] SpawnError),
    /// spawn / wait / I/O 에러.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// non-zero exit code — stderr 가 함께 캡처됨.
    #[error("tippecanoe exited with {code}: {stderr}")]
    Failed {
        /// exit code (signal kill 시 -1).
        code: i32,
        /// stderr 마지막 4KB (전체 캡처는 너무 큼).
        stderr: String,
    },
    /// 입력 inputs 가 비어있음.
    #[error("no input files provided")]
    NoInputs,
    /// 출력 파일이 안 만들어짐 (tippecanoe 가 silent fail).
    #[error("output file {path} not created")]
    OutputMissing {
        /// 기대한 출력 경로.
        path: String,
    },
}
