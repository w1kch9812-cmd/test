use std::process::ExitCode;

use tracing::{error, warn};

use crate::handover::{run_bronze_cli, run_cleanup_backups_cli, run_gold_cli, run_promote_cli};

pub async fn async_main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = match parse_cli_command(&args) {
        Ok(command) => command,
        Err(error) => return unknown_subcommand_exit(&error),
    };
    wait_for_cli_task_or_shutdown(spawn_cli_task(command)).await
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CliCommand {
    Bronze,
    Gold(Vec<String>),
    Promote(Vec<String>),
    CleanupManifestBackups(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UnknownSubcommand(String);

fn parse_cli_command(args: &[String]) -> Result<CliCommand, UnknownSubcommand> {
    match args.first().map_or("bronze", String::as_str) {
        "bronze" | "" => Ok(CliCommand::Bronze),
        "gold" => Ok(CliCommand::Gold(subcommand_args(args))),
        "promote" => Ok(CliCommand::Promote(subcommand_args(args))),
        "cleanup-manifest-backups" => Ok(CliCommand::CleanupManifestBackups(subcommand_args(args))),
        other => Err(UnknownSubcommand(other.to_owned())),
    }
}

fn subcommand_args(args: &[String]) -> Vec<String> {
    args.get(1..).map_or_else(Vec::new, <[String]>::to_vec)
}

fn spawn_cli_task(command: CliCommand) -> tokio::task::JoinHandle<ExitCode> {
    match command {
        CliCommand::Bronze => tokio::spawn(async move { run_bronze_cli(&[]) }),
        CliCommand::Gold(args) => tokio::spawn(async move { run_gold_cli(&args) }),
        CliCommand::Promote(args) => tokio::spawn(async move { run_promote_cli(&args) }),
        CliCommand::CleanupManifestBackups(args) => {
            tokio::spawn(async move { run_cleanup_backups_cli(&args) })
        }
    }
}

fn unknown_subcommand_exit(error: &UnknownSubcommand) -> ExitCode {
    error!(
        subcommand = %error.0,
        "unknown subcommand -- use `bronze` | `gold` | `promote` | `cleanup-manifest-backups`"
    );
    ExitCode::from(2)
}

async fn wait_for_cli_task_or_shutdown(task: tokio::task::JoinHandle<ExitCode>) -> ExitCode {
    // Handover stubs still keep the old immediate-abort shutdown behavior so
    // retired scheduled jobs cannot hang during shutdown.
    tokio::select! {
        biased;
        result = task => {
            task_exit_code(result)
        }
        () = shutdown_signal() => {
            warn!("shutdown signal received; aborting handover stub");
            ExitCode::from(130)
        }
    }
}

fn task_exit_code(result: Result<ExitCode, tokio::task::JoinError>) -> ExitCode {
    match result {
        Ok(code) => code,
        Err(error) => {
            error!(error = %error, "task panicked or aborted");
            ExitCode::FAILURE
        }
    }
}

/// Listen for Ctrl+C and, on Unix, SIGTERM.
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        if let Err(error) = shutdown_signal_unix().await {
            warn!(error = %error, "unix signal handler install failed; falling back to ctrl-c");
            let _ = tokio::signal::ctrl_c().await;
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

#[cfg(unix)]
async fn shutdown_signal_unix() -> Result<(), std::io::Error> {
    use tokio::signal::unix::{signal, SignalKind};
    let mut term = signal(SignalKind::terminate())?;
    let mut int = signal(SignalKind::interrupt())?;
    tokio::select! {
        _ = term.recv() => {}
        _ = int.recv() => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{parse_cli_command, CliCommand};

    #[test]
    fn cli_parser_preserves_platform_core_handover_subcommands() {
        let promote_args = vec!["promote".to_owned(), "--dry-run".to_owned()];
        let promote = parse_cli_command(&promote_args);
        assert!(matches!(promote, Ok(CliCommand::Promote(args)) if args == ["--dry-run"]));

        let cleanup_args = vec!["cleanup-manifest-backups".to_owned()];
        let cleanup = parse_cli_command(&cleanup_args);
        assert!(matches!(
            cleanup,
            Ok(CliCommand::CleanupManifestBackups(args)) if args.is_empty()
        ));
    }
}
