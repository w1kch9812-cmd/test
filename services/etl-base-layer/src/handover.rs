use std::process::ExitCode;

use tracing::error;

pub fn run_promote_cli(_args: &[String]) -> ExitCode {
    error!("{}", manifest_write_path_disabled_message("promote"));
    ExitCode::from(2)
}

/// Legacy manifest backup cleanup subcommand (ADR 0028 + runbook § 6).
///
/// Consumer-only handover 이후 항상 실패한다. manifest backup lifecycle 은
/// platform-core Catalog 가 담당한다.
pub fn run_cleanup_backups_cli(_args: &[String]) -> ExitCode {
    error!(
        "{}",
        manifest_write_path_disabled_message("cleanup-manifest-backups")
    );
    ExitCode::from(2)
}

fn manifest_write_path_disabled_message(action: &str) -> String {
    format!(
        "Gongzzang `{action}` is disabled: static vector tile manifest ownership moved to platform-core Catalog. Gongzzang is consumer only; read /catalog/v1/vector-tiles/manifest instead."
    )
}

#[cfg(test)]
mod tests {
    use super::manifest_write_path_disabled_message;

    #[test]
    fn manifest_write_path_points_to_platform_core_owner() {
        let message = manifest_write_path_disabled_message("promote");

        assert!(message.contains("platform-core Catalog"));
        assert!(message.contains("/catalog/v1/vector-tiles/manifest"));
        assert!(message.contains("consumer only"));
    }
}
