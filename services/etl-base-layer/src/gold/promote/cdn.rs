use sp9_base_layer_config::Environment;
use tracing::info;

use super::PromoteError;

/// Round 4 #5 — CDN purge 결과의 typed outcome (이전 `Option<bool>` 의 ambiguity 제거).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg(test)]
pub enum CdnPurgeOutcome {
    /// Cloudflare API 200 OK — manifest URL purge 완료.
    Purged,
    /// dev / staging env (`ETL_ENVIRONMENT != "production"`) 에서 CDN config 누락 시
    /// silent skip — manifest 의 `Cache-Control: no-cache` 가 fallback.
    SkippedDevMode,
    /// CDN config 누락. production 에서는 본 variant 가 `PromoteError::CdnPurgeMissingConfig`
    /// 로 변환 — 본 variant 는 `Promote` 단계 도달 안 함 (fail-fast).
    #[allow(dead_code)]
    SkippedNoConfig,
    /// Cloudflare API 호출은 했으나 transient HTTP / 4xx 응답. promote 자체는 성공
    /// (manifest 는 publish), CDN purge 만 실패. 상위 `PromoteError::CdnPurge` 로 박제.
    Failed,
}

/// Round 5 P0 — promote 의 step 0 pre-flight. production env 에서 CDN config
/// 누락이면 manifest 만지기 전 즉시 abort.
///
/// 본 check 는 `cloudflare_purge` 의 분기와 *동일 로직* 으로 wired — drift 차단.
/// dev/staging 에서는 silent OK (`SkippedDevMode` 가 step 5 에서 자연 발생).
#[cfg(test)]
pub(super) fn preflight_cdn_config() -> Result<(), PromoteError> {
    // ADR 0029 — `Environment::is_production_from_env()` SSOT (ETL_ENVIRONMENT 또는
    // ADR 0035 — `ETL_ENVIRONMENT` SSOT only.
    if !Environment::is_production_from_env() {
        return Ok(());
    }
    cdn_config_missing_error(&current_missing_cdn_config_names())
}

#[cfg(test)]
fn env_nonempty_value(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
fn current_missing_cdn_config_names() -> Vec<&'static str> {
    missing_cdn_config_names(
        env_nonempty_value("CLOUDFLARE_API_TOKEN").is_some(),
        env_nonempty_value("CLOUDFLARE_ZONE_ID").is_some(),
        env_nonempty_value("R2_PUBLIC_URL_BASE").is_some(),
    )
}

#[cfg(test)]
fn missing_cdn_config_names(
    token_present: bool,
    zone_id_present: bool,
    public_base_present: bool,
) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if !token_present {
        missing.push("CLOUDFLARE_API_TOKEN");
    }
    if !zone_id_present {
        missing.push("CLOUDFLARE_ZONE_ID");
    }
    if !public_base_present {
        missing.push("R2_PUBLIC_URL_BASE");
    }
    missing
}

#[cfg(test)]
fn cdn_config_missing_error(missing: &[&str]) -> Result<(), PromoteError> {
    if missing.is_empty() {
        return Ok(());
    }
    Err(PromoteError::CdnPurgeMissingConfig {
        missing: missing.join(", "),
    })
}

#[derive(Debug)]
#[cfg(test)]
struct CdnPurgeConfig {
    token: String,
    zone_id: String,
    public_base: String,
}

/// Cloudflare CDN purge — `CLOUDFLARE_API_TOKEN` + `CLOUDFLARE_ZONE_ID` + `R2_PUBLIC_URL_BASE`
/// 모두 set 시 활성. manifest 객체 URL 만 purge.
///
/// Round 4 #5 (Codex audit): production env 에서 config 누락 = fail-fast (이전: silent
/// `Ok(false)` 로 manifest 가 stale CDN 으로 publish 되는 trick). dev / staging 은
/// silent skip 그대로 (`SkippedDevMode`).
#[cfg(test)]
pub(super) async fn cloudflare_purge(manifest_key: &str) -> Result<CdnPurgeOutcome, PromoteError> {
    let Some(config) = read_cdn_purge_config()? else {
        return Ok(CdnPurgeOutcome::SkippedDevMode);
    };

    let url = cloudflare_purge_api_url(&config.zone_id);
    let target_url = cdn_manifest_url(&config.public_base, manifest_key);
    let body = serde_json::json!({ "files": [target_url] });
    let resp = send_cdn_purge_request(&config, &url, &body).await?;

    ensure_cdn_purge_success(resp).await?;
    info!(target = %target_url, "CDN cache purged");
    Ok(CdnPurgeOutcome::Purged)
}

#[cfg(test)]
fn read_cdn_purge_config() -> Result<Option<CdnPurgeConfig>, PromoteError> {
    let token = env_nonempty_value("CLOUDFLARE_API_TOKEN");
    let zone_id = env_nonempty_value("CLOUDFLARE_ZONE_ID");
    let public_base = env_nonempty_value("R2_PUBLIC_URL_BASE");

    match (token, zone_id, public_base) {
        (Some(token), Some(zone_id), Some(public_base)) => Ok(Some(CdnPurgeConfig {
            token,
            zone_id,
            public_base,
        })),
        (token, zone_id, public_base) => {
            handle_incomplete_cdn_config(token.is_some(), zone_id.is_some(), public_base.is_some())
        }
    }
}

#[cfg(test)]
fn handle_incomplete_cdn_config(
    token_present: bool,
    zone_id_present: bool,
    public_base_present: bool,
) -> Result<Option<CdnPurgeConfig>, PromoteError> {
    let missing = missing_cdn_config_names(token_present, zone_id_present, public_base_present);
    // ADR 0029 — preflight 와 동일 SSOT 검사.
    if Environment::is_production_from_env() {
        cdn_config_missing_error(&missing)?;
    } else {
        info!(missing = ?missing, "CDN purge skipped in non-production env");
    }
    Ok(None)
}

#[cfg(test)]
fn cloudflare_purge_api_url(zone_id: &str) -> String {
    format!("https://api.cloudflare.com/client/v4/zones/{zone_id}/purge_cache")
}

#[cfg(test)]
fn cdn_manifest_url(public_base: &str, manifest_key: &str) -> String {
    if public_base.ends_with('/') {
        return format!("{public_base}{manifest_key}");
    }
    format!("{public_base}/{manifest_key}")
}

#[cfg(test)]
async fn send_cdn_purge_request(
    config: &CdnPurgeConfig,
    url: &str,
    body: &serde_json::Value,
) -> Result<reqwest::Response, PromoteError> {
    let client = reqwest::Client::new();
    Ok(client
        .post(url)
        .bearer_auth(&config.token)
        .json(body)
        .send()
        .await?)
}

#[cfg(test)]
async fn ensure_cdn_purge_success(resp: reqwest::Response) -> Result<(), PromoteError> {
    let status = resp.status();
    if !status.is_success() {
        // Round 4 #6 — body read 실패도 typed 박제. 이전 `unwrap_or_default()` silent loss 제거.
        return Err(cdn_purge_failure(status, resp).await);
    }
    Ok(())
}

#[cfg(test)]
async fn cdn_purge_failure(status: reqwest::StatusCode, resp: reqwest::Response) -> PromoteError {
    let (body, body_read_error) = match resp.text().await {
        Ok(text) => (text.chars().take(1024).collect::<String>(), None),
        Err(e) => (String::new(), Some(format!("body read failed: {e}"))),
    };
    PromoteError::CdnPurge {
        status: status.as_u16(),
        body,
        body_read_error,
    }
}
