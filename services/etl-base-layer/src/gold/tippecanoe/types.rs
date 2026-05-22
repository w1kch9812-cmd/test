use sp9_base_layer_config::Layer as Sp9Layer;

/// tippecanoe 빌드 한 번 = 한 layer.
#[derive(Debug, Clone, Copy)]
pub enum LayerKind {
    /// 필지 (parcels) Z14-17, layer 이름 `parcels`.
    Parcels,
    /// 행정구역 (admin) Z6-12, layer 이름 `admin`.
    Admin,
    /// 산업단지 (complex) Z0-16, layer 이름 `complex`. 모든 zoom 에서 visible.
    Complex,
}

impl LayerKind {
    /// 모든 variant — `sp9_base_layer_config::Layer::ALL` 로부터 derive.
    /// **SSOT**: 새 layer 추가 시 `sp9_base_layer_config::Layer` 에만 추가하면 됨.
    /// `From<Sp9Layer>` 가 exhaustive match 라 compiler 가 누락 차단.
    /// **주의**: ETL matrix / promote 검증은 [`Self::active_vec`] 사용 — 본 iterator 는
    /// inactive layer (admin/complex) 도 포함. registry / 전수 검증 용도.
    #[allow(dead_code)] // active_vec() 가 동일 path 사용 — registry 용도 보존
    pub fn all() -> impl Iterator<Item = Self> {
        Sp9Layer::ALL.iter().map(|l| Self::from(*l))
    }

    /// 모든 variant 의 owned vec — registry / 전수 iterate 가 필요한 callsite.
    /// **주의**: ETL matrix / promote 검증은 [`Self::active_vec`] 사용 (admin/complex 같은
    /// inactive layer 제외). 본 함수는 *registry* 용도 (Layer enum 의 모든 variant 표시).
    #[must_use]
    #[allow(dead_code)] // active_vec() 가 ETL path 의 main caller — 본 함수는 registry 보존
    pub fn all_vec() -> Vec<Self> {
        Self::all().collect()
    }

    /// **현재 ETL build-active** layer 의 owned vec — Round 4 stop-hook fix.
    /// `Sp9Layer::is_active_in_etl()` SSOT 통과한 variant 만. promote 의 staging spec
    /// 검증 / matrix iteration 이 본 함수 사용 — admin/complex 같은 inactive layer 의
    /// `MissingLineage` false-positive 차단 (ADR 0027).
    #[must_use]
    #[cfg(test)]
    pub fn active_vec() -> Vec<Self> {
        Sp9Layer::ALL
            .iter()
            .filter(|l| l.is_active_in_etl())
            .map(|l| Self::from(*l))
            .collect()
    }

    /// PMTiles 안의 layer 이름 (프론트 `addLayer({ "source-layer": ... })` 에 매칭).
    /// **SSOT** — 프론트 `LAYER_IDS` 가 본 enum 의 reflection.
    #[must_use]
    pub const fn layer_name(self) -> &'static str {
        match self {
            Self::Parcels => "parcels",
            Self::Admin => "admin",
            Self::Complex => "complex",
        }
    }

    /// PMTiles 빌드 zoom range `(min, max)` — tippecanoe `-Z`/`-z` 인자 + manifest 박제.
    /// **SSOT** — 프론트 source 의 minzoom/maxzoom 이 본 값을 따라야 함 (manifest fetch).
    #[must_use]
    pub const fn zoom_range(self) -> (u8, u8) {
        match self {
            Self::Parcels => (14, 17),
            Self::Admin => (6, 12),
            // 산업단지: 사용자 명시 요구 — "모든 zoom level 에서 visible" (SSS).
            // tippecanoe 가 z0-5 에서 sub-pixel polygon coalesce 처리.
            Self::Complex => (0, 16),
        }
    }

    /// 프론트 `addLayer({ minzoom })` 권장값 — *render* 시작 zoom.
    /// PMTiles `min_zoom` 보다 *클* 수 있음 (e.g. parcels tile 14 부터 있지만 render 는 16+).
    #[must_use]
    #[cfg(test)]
    pub const fn render_min_zoom(self) -> u8 {
        match self {
            Self::Parcels => 16,
            // admin: outline 은 z0 부터 visible. complex (산업단지): 사용자 요구 — 모든 zoom 에서
            // render. 둘 다 0 이라 같은 arm.
            Self::Admin | Self::Complex => 0,
        }
    }

    /// 프론트 `addLayer({ maxzoom })` 권장값 (render 종료). `None` = mapbox-gl default 24.
    #[must_use]
    #[cfg(test)]
    pub const fn render_max_zoom(self) -> Option<u8> {
        match self {
            Self::Admin => Some(16),
            _ => None,
        }
    }

    /// CDN `Cache-Control: max-age=<seconds>` — layer 별 차별화 (gongzzang-develop 차용).
    /// flat tile 은 immutable (URL versioning 으로 무효화) → 1년.
    /// 향후 layer 별 차등 (e.g. complex 일 6시간) 가능성 위해 `self` 인자 보존.
    #[must_use]
    #[cfg(test)]
    #[allow(clippy::unused_self)]
    pub const fn cache_max_age_seconds(self) -> u32 {
        // 31_536_000s = 365일. immutable + URL versioning 패턴 (ADR 0021 § Tier A).
        31_536_000
    }
}

/// SSOT 브리지 — `sp9_base_layer_config::Layer` → `LayerKind` 자동 변환.
/// `Layer::ALL` 이 추가되면 컴파일러가 이 match 에서 누락 variant 를 차단.
impl From<Sp9Layer> for LayerKind {
    fn from(l: Sp9Layer) -> Self {
        match l {
            Sp9Layer::Parcels => Self::Parcels,
            Sp9Layer::Admin => Self::Admin,
            Sp9Layer::Complex => Self::Complex,
        }
    }
}
