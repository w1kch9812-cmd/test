use sp9_base_layer_config::{R2PublicBase, Version};

#[derive(Debug, Clone)]
pub struct R2Config {
    /// Cloudflare account id — endpoint 구성 (`<id>.r2.cloudflarestorage.com`).
    pub account_id: String,
    /// R2 access key (S3-호환 access key id).
    pub access_key: String,
    /// R2 secret key (S3-호환 secret).
    pub secret_key: String,
    /// 대상 버킷 이름 (예: `gongzzang-static`).
    pub bucket: String,
    /// Bronze archive key prefix (예: `bronze`). 끝 `/` 제외.
    pub bronze_prefix: String,
    /// Gold artifact key prefix (예: `gold`). 끝 `/` 제외.
    /// T3b.1 에서는 미사용 — T3b.2 의 ogr2ogr/tippecanoe 출력 PUT key 에 사용.
    #[allow(dead_code)]
    pub gold_prefix: String,
}

impl R2Config {
    /// `https://<account_id>.r2.cloudflarestorage.com` URL 빌드.
    #[must_use]
    pub fn endpoint_url(&self) -> String {
        format!("https://{}.r2.cloudflarestorage.com", self.account_id)
    }

    /// Gold layer flat tile prefix: `<gold_prefix>/<version>/<layer>`.
    /// **SSOT** — 모든 gold key 생성이 이 helper 를 통해야 함.
    /// `version` 은 검증된 [`Version`] 만 받음 — 잘못된 라벨 생성 시점 차단.
    #[must_use]
    pub fn gold_layer_prefix(&self, version: &Version, layer_name: &str) -> String {
        format!("{}/{}/{}", self.gold_prefix, version, layer_name)
    }

    /// Gold layer `TileJSON` key: `<gold_prefix>/<version>/<layer>.json`.
    #[must_use]
    pub fn tilejson_key(&self, version: &Version, layer_name: &str) -> String {
        format!("{}/{}/{}.json", self.gold_prefix, version, layer_name)
    }

    /// Gold manifest key: `<gold_prefix>/manifest.json`.
    #[must_use]
    #[cfg(test)]
    pub fn manifest_key(&self) -> String {
        format!("{}/manifest.json", self.gold_prefix)
    }

    /// Gold manifest backup key: `<gold_prefix>/manifest.<version>.json`.
    /// `version` 은 [`Version`] — 백업 키도 동일 검증 통과.
    #[must_use]
    #[cfg(test)]
    pub fn manifest_backup_key(&self, version: &Version) -> String {
        format!("{}/manifest.{}.json", self.gold_prefix, version)
    }

    /// Gold staging spec key: `<gold_prefix>/staging/<version>/<layer>.spec.json`.
    #[must_use]
    pub fn staging_spec_key(&self, version: &Version, layer_name: &str) -> String {
        format!(
            "{}/staging/{}/{}.spec.json",
            self.gold_prefix, version, layer_name
        )
    }

    /// Tiles URL template for `TileJSON` / manifest:
    /// `<public_base>/<gold_prefix>/<version>/<layer>/{z}/{x}/{y}.pbf`.
    /// `public_base` / `version` 모두 newtype — invalid scheme/host/format 시점 차단.
    #[must_use]
    pub fn tiles_url_template(
        &self,
        public_base: &R2PublicBase,
        version: &Version,
        layer_name: &str,
    ) -> String {
        let raw = public_base.as_str();
        let base = if raw.ends_with('/') {
            raw.to_owned()
        } else {
            format!("{raw}/")
        };
        #[allow(clippy::literal_string_with_formatting_args)]
        {
            format!(
                "{base}{}/{}/{}/{{z}}/{{x}}/{{y}}.pbf",
                self.gold_prefix, version, layer_name
            )
        }
    }
}
