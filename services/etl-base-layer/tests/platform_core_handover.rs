//! Regression tests for the platform-core vector tile manifest handover.

use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn etl_binary_has_no_legacy_vector_tile_write_entrypoints() -> Result<(), Box<dyn std::error::Error>>
{
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let main_rs = fs::read_to_string(manifest_dir.join("src/main.rs"))?;

    for forbidden in [
        "mod bronze;",
        "mod bronze_cli;",
        "mod config;",
        "mod dtmk_prepare;",
        "mod error;",
        "mod gold;",
        "mod gold_cli;",
        "mod gold_upload;",
        "mod manifest;",
        "mod r2_upload;",
        "mod verify_cli;",
        "run_legacy_promote_cli",
        "run_legacy_cleanup_backups_cli",
        "promote::run(",
        "promote::cleanup_manifest_backups(",
    ] {
        assert!(
            !main_rs.contains(forbidden),
            "Gongzzang etl-base-layer must not retain legacy vector tile write entrypoint `{forbidden}`; platform-core Catalog owns static vector tile artifact lifecycle"
        );
    }
    Ok(())
}

#[test]
fn legacy_vector_tile_implementation_sources_are_absent() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    for forbidden in [
        "src/bronze",
        "src/bronze_cli.rs",
        "src/config.rs",
        "src/dtmk_prepare.rs",
        "src/error.rs",
        "src/gold",
        "src/gold_cli.rs",
        "src/gold_upload.rs",
        "src/manifest.rs",
        "src/r2_upload.rs",
        "src/r2_upload",
        "src/test_support.rs",
        "src/verify_cli.rs",
    ] {
        assert!(
            !manifest_dir.join(forbidden).exists(),
            "Gongzzang etl-base-layer must not retain `{forbidden}` after platform-core Catalog handover"
        );
    }
}

#[test]
fn manifest_write_subcommands_exit_with_platform_core_handover_notice(
) -> Result<(), Box<dyn std::error::Error>> {
    let binary = env!("CARGO_BIN_EXE_etl-base-layer");

    for subcommand in ["bronze", "gold", "promote", "cleanup-manifest-backups"] {
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
