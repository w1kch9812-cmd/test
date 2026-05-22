use tracing::{info, warn};

use super::PromoteError;
use crate::r2_upload::{R2Uploader, RemoteObject, UploadError};

/// Round 5 P1 — manifest backup chain cleanup (ADR 0028, runbook § 6).
///
/// `gold/manifest.<version>.json` backup 들을 list → 오래된 것부터 삭제 → 최근 `keep`
/// 개만 보존. monthly cron 으로 호출 (`.github/workflows/sp9-manifest-backup-cleanup.yml`).
///
/// 정렬 기준: R2 `LastModified` (object 메타). version 라벨 자체로 sort 도 가능하지만
/// (`v_2026_05` < `v_2026_06`) external 변경 (예: 수동 backup) 도 자연 처리하려면
/// modification time 이 더 안전.
///
/// # Errors
///
/// - R2 list / delete API 실패
/// - `keep` 가 0 이면 [`PromoteError::InvalidCleanupKeep`] (실수 방지)
#[cfg(test)]
pub async fn cleanup_manifest_backups(
    uploader: &R2Uploader,
    keep: usize,
) -> Result<CleanupResult, PromoteError> {
    validate_cleanup_keep(keep)?;

    let backups = list_manifest_backups(uploader).await?;
    let total_found = backups.len();

    log_manifest_backup_chain_listed(total_found, keep);

    if total_found <= keep {
        return Ok(cleanup_not_needed(total_found, keep));
    }

    let delete_targets = manifest_backup_delete_targets(backups, keep);
    let report = delete_manifest_backup_targets(uploader, &delete_targets).await;

    log_manifest_backup_cleanup_attempted(total_found, &report);

    cleanup_result(total_found, report)
}

#[cfg(test)]
fn log_manifest_backup_chain_listed(total_found: usize, keep: usize) {
    info!(
        backup_count = total_found,
        keep, "manifest backup chain listed"
    );
}

#[cfg(test)]
fn log_manifest_backup_cleanup_attempted(total_found: usize, report: &BackupDeletionReport) {
    info!(
        total = total_found,
        kept = total_found - report.deleted,
        deleted = report.deleted,
        failures = report.failures.len(),
        "manifest backup cleanup attempted"
    );
}

#[cfg(test)]
const fn validate_cleanup_keep(keep: usize) -> Result<(), PromoteError> {
    if keep == 0 {
        return Err(PromoteError::InvalidCleanupKeep);
    }
    Ok(())
}

#[cfg(test)]
async fn list_manifest_backups(uploader: &R2Uploader) -> Result<Vec<RemoteObject>, PromoteError> {
    // backup key 형식 — `<gold_prefix>/manifest.<version>.json`. prefix 는 manifest_key
    // 의 dirname + `manifest.` glob.
    let backup_prefix = manifest_backup_prefix(uploader);
    let manifest_key = uploader.config().manifest_key();
    let listed = uploader.list_objects(&backup_prefix).await?;

    Ok(listed
        .into_iter()
        .filter(|obj| is_manifest_backup_object(obj, &backup_prefix, &manifest_key))
        .collect())
}

#[cfg(test)]
fn manifest_backup_prefix(uploader: &R2Uploader) -> String {
    format!("{}/manifest.", uploader.config().gold_prefix)
}

#[cfg(test)]
fn is_manifest_backup_object(obj: &RemoteObject, backup_prefix: &str, manifest_key: &str) -> bool {
    // backup 파일만 — `manifest.json` 자체는 제외 (`manifest.<version>.json` 만).
    // 패턴: `<gold_prefix>/manifest.<라벨>.json` 의 `.` 가 정확히 2개 (`manifest`,
    // `<라벨>`, `json`). `manifest.json` 은 `.` 1개.
    std::path::Path::new(&obj.key)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
        && obj.key.starts_with(backup_prefix)
        && obj.key != manifest_key
}

#[cfg(test)]
fn cleanup_not_needed(total_found: usize, keep: usize) -> CleanupResult {
    info!(
        backup_count = total_found,
        keep, "no cleanup needed (within retention)"
    );
    CleanupResult {
        total_found,
        kept: total_found,
        deleted: 0,
    }
}

#[cfg(test)]
fn manifest_backup_delete_targets(
    mut backups: Vec<RemoteObject>,
    keep: usize,
) -> Vec<RemoteObject> {
    // 오래된 것 먼저 — backup key 의 *문자열* 정렬 으로 충분 (version 라벨이
    // `v_YYYY_MM` 형식이라 lexicographic = chronological).
    // 단 외부 변경에 안전하려면 `etag` 또는 별도 LastModified field 가 더 정확.
    // 본 단계는 lexicographic 으로 충분 — runbook § 6 에서 "외부 변경 0" 가정.
    backups.sort_by(|a, b| a.key.cmp(&b.key));
    backups.truncate(backups.len() - keep);
    backups
}

#[derive(Debug, Default)]
#[cfg(test)]
struct BackupDeletionReport {
    deleted: usize,
    failures: Vec<(String, String)>,
}

#[cfg(test)]
async fn delete_manifest_backup_targets(
    uploader: &R2Uploader,
    targets: &[RemoteObject],
) -> BackupDeletionReport {
    let mut report = BackupDeletionReport::default();
    for obj in targets {
        match delete_manifest_backup(uploader, obj).await {
            Ok(()) => report.deleted += 1,
            Err(failure) => report.failures.push(failure),
        }
    }
    report
}

#[cfg(test)]
async fn delete_manifest_backup(
    uploader: &R2Uploader,
    obj: &RemoteObject,
) -> Result<(), (String, String)> {
    match uploader.delete_object(&obj.key).await {
        Ok(()) => {
            log_manifest_backup_deleted(&obj.key);
            Ok(())
        }
        Err(e) => Err(log_manifest_backup_delete_failed(obj, &e)),
    }
}

#[cfg(test)]
fn log_manifest_backup_deleted(key: &str) {
    info!(key = %key, "manifest backup deleted (cleanup)");
}

#[cfg(test)]
fn log_manifest_backup_delete_failed(obj: &RemoteObject, error: &UploadError) -> (String, String) {
    // Round 5 (final stop-hook) — partial cleanup 실패는 typed `Err` 로
    // 전파. 이전엔 warn 후 `Ok(())` 반환 — silent partial = SSS 위반.
    // 진행은 계속 (다른 backup 도 시도) — 모든 실패 모은 후 `Err` 박제.
    warn!(key = %obj.key, error = %error, "backup delete failed — collecting for typed Err");
    (obj.key.clone(), error.to_string())
}

#[cfg(test)]
fn cleanup_result(
    total_found: usize,
    report: BackupDeletionReport,
) -> Result<CleanupResult, PromoteError> {
    if !report.failures.is_empty() {
        return Err(PromoteError::PartialCleanup {
            attempted: report.deleted + report.failures.len(),
            deleted: report.deleted,
            failures: report.failures,
        });
    }

    Ok(CleanupResult {
        total_found,
        kept: total_found - report.deleted,
        deleted: report.deleted,
    })
}

/// `cleanup_manifest_backups` 결과.
#[derive(Debug, Clone, Copy)]
#[cfg(test)]
pub struct CleanupResult {
    /// 발견한 backup 총 개수.
    pub total_found: usize,
    /// 보존한 개수 (보통 `keep`).
    pub kept: usize,
    /// 삭제한 개수.
    pub deleted: usize,
}
