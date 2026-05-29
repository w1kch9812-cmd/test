use std::process::ExitCode;

use tracing::error;

pub fn run_bronze_cli(_args: &[String]) -> ExitCode {
    error!("{}", handover_disabled_message("bronze"));
    ExitCode::from(2)
}

pub fn run_gold_cli(_args: &[String]) -> ExitCode {
    error!("{}", handover_disabled_message("gold"));
    ExitCode::from(2)
}

pub fn run_promote_cli(_args: &[String]) -> ExitCode {
    error!("{}", handover_disabled_message("promote"));
    ExitCode::from(2)
}

/// Legacy manifest backup cleanup subcommand.
///
/// After platform-core handover this always fails closed. Platform Core Catalog
/// owns the manifest backup lifecycle.
pub fn run_cleanup_backups_cli(_args: &[String]) -> ExitCode {
    error!("{}", handover_disabled_message("cleanup-manifest-backups"));
    ExitCode::from(2)
}

fn handover_disabled_message(action: &str) -> String {
    format!(
        "Gongzzang `{action}` is disabled: static vector tile artifact lifecycle moved to platform-core Catalog. Gongzzang is consumer only; read /catalog/v1/vector-tiles/manifest instead."
    )
}

#[cfg(test)]
mod tests {
    use super::handover_disabled_message;

    #[test]
    fn manifest_write_path_points_to_platform_core_owner() {
        let message = handover_disabled_message("promote");

        assert!(message.contains("platform-core Catalog"));
        assert!(message.contains("/catalog/v1/vector-tiles/manifest"));
        assert!(message.contains("consumer only"));
    }
}
