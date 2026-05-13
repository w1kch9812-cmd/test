//! Regression tests for the platform-core vector tile manifest handover.

use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn etl_binary_has_no_legacy_manifest_write_entrypoints() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let main_rs = fs::read_to_string(manifest_dir.join("src/main.rs"))?;

    for forbidden in [
        "run_legacy_promote_cli",
        "run_legacy_cleanup_backups_cli",
        "promote::run(",
        "promote::cleanup_manifest_backups(",
    ] {
        assert!(
            !main_rs.contains(forbidden),
            "Gongzzang etl-base-layer must not retain legacy manifest write entrypoint `{forbidden}`; platform-core Catalog owns manifest promote/rollback/cleanup"
        );
    }
    Ok(())
}

#[test]
fn manifest_write_subcommands_exit_with_platform_core_handover_notice(
) -> Result<(), Box<dyn std::error::Error>> {
    let binary = env!("CARGO_BIN_EXE_etl-base-layer");

    for subcommand in ["promote", "cleanup-manifest-backups"] {
        let output = Command::new(binary).arg(subcommand).output()?;

        assert_eq!(
            output.status.code(),
            Some(2),
            "`{subcommand}` must be disabled in Gongzzang after platform-core handover"
        );
        let logs = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(logs.contains("platform-core Catalog"), "{logs}");
        assert!(logs.contains("consumer only"), "{logs}");
    }
    Ok(())
}
