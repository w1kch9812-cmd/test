# ADR 0027 — admin / complex layer ETL source 결정 보류 + `is_active_in_etl` SSOT gate

| | |
|---|---|
| 작성일 | 2026-05-08 |
| 상태 | Accepted |
| 선행 | [ADR 0021](./0021-static-vector-tile-decomposition.md), [ADR 0022](./0022-bronze-scraping-isolated-python-service.md), [ADR 0025](./0025-bronze-scraping-workflow-orchestrator-not-rust-spawn.md) |

## 결정

`Layer::Admin` 과 `Layer::Complex` 의 ETL Bronze source 는 **현 sprint 에 결정 보류**.
대신 `Layer::is_active_in_etl(self) -> bool` SSOT 함수를 박제 — workflow matrix
가 본 함수를 통과한 layer 만 빌드. admin/complex 가 parcels 의 dtmk prefix 를
*임시 재사용* 하던 trick 을 *컴파일 시점에* 차단.

```rust
// crates/sp9-base-layer-config/src/lib.rs
pub const fn is_active_in_etl(self) -> bool {
    match self {
        Self::Parcels => true,
        // 별도 source 미준비 — 본 ADR.
        Self::Admin | Self::Complex => false,
    }
}
```

```yaml
# .github/workflows/sp9-base-layer-etl.yml
matrix:
  layer: ${{ fromJson(needs.setup.outputs.active_layers_json) }}
```

## 컨텍스트 — 박제된 trick

Codex Round 4 audit 가 발견한 P0 trick:

> `.github/workflows/sp9-base-layer-etl.yml:278` — admin/complex layer 도 parcel
> dtmk prefix 임시 사용. TODO 주석 박제는 *trick visibility* 만 있을 뿐 fix 아님.

이전 path:
```yaml
matrix:
  layer: [parcels, admin, complex]   # SSOT 의 Layer::ALL
```

빌드 결과:
- `gold/v<N>/parcels/{z}/{x}/{y}.pbf` ← 정확 (V-World dtmk = 필지)
- `gold/v<N>/admin/{z}/{x}/{y}.pbf`   ← *parcels 데이터로 빌드된 admin 라벨 tile* (잘못됨)
- `gold/v<N>/complex/{z}/{x}/{y}.pbf` ← *parcels 데이터로 빌드된 complex tile* (잘못됨)

= 클라이언트 `addLayer({ source-layer: "admin" })` 시 *필지 데이터를 행정구역 라벨로* 표시.
silent partial build = 사용자 SSS 기준 ("trick 1개라도 거부") 명백한 위반.

## 검토한 옵션

### A — 새 source 즉시 구현
- admin: V-World 행정구역 dataset (별도 dsId — 검증 필요)
- complex: 공공데이터포털 산업단지 SHP (별도 다운로드 + .prj 검증)
- `services/scraper-py/` 에 `admin_vworld.py` / `complex_data_go_kr.py` 추가
- ADR 0022 의 isolated Python 패턴 그대로
- **거부 이유**: source dsId / API key / quota / 약관 검증 미완. 한 sprint 안 안전.
  잘못된 source 박제 = 더 큰 trick.

### B — admin/complex 를 enum 에서 제거 (Round 4 직전 상태)
- `Layer` enum 에서 `Admin` / `Complex` variant 삭제
- 향후 추가 시 enum 확장 + 호출처 갱신
- **거부 이유**: 프론트 (`apps/web/components/listings/listing-map.tsx`) 가 이미
  `addSource("admin")` / `addSource("complex")` 를 *조건부* 호출. enum 제거 시 frontend
  도 일괄 변경. 또한 `LayerKind::zoom_range / render_min_zoom / cache_max_age_seconds`
  등 build-time 정책도 함께 박제됨 — *언제 admin/complex 가 active 가 될지* 의
  의도가 enum 에 박혀있는 게 SSS.

### C — `is_active_in_etl` SSOT gate (본 ADR 채택)
- `Layer` enum 의 모든 variant 유지 (zoom range / cache 정책 등 미래 박제 보존)
- `is_active_in_etl()` 분리 함수 — *현재 build 가능* 한 layer 만 표시
- workflow matrix 가 `active_layers` JSON output 만 소비 → admin/complex 자동 제외
- 미래 source 결정 시 `is_active_in_etl()` 만 변경 + 본 ADR 갱신
- **장점**: drift 0, build-time 정책 보존, 프론트 영향 0, 의도 명시

## SSS 7기둥 매핑

| 기둥 | 이전 (parcel prefix 재사용) | C (본 결정) |
|---|---|---|
| 일관성 | ❌ — admin/complex tile 이 parcels 데이터 | ✅ — active layer 만 빌드 |
| 자동강제 | ❌ — TODO 주석은 검증 0 | ✅ — `Layer::is_active_in_etl` 컴파일 박제 + matrix gate |
| 추적성 | ❌ — silent partial = 사후 분석 어려움 | ✅ — 본 ADR 박제 + active_layers JSON output |
| 안전성 | ❌ — 잘못된 데이터가 prod publish | ✅ — admin/complex 미빌드 = manifest 에 없음 = client 스킵 |
| 가시성 | ❌ — workflow log 만으로는 잘못 빌드인지 모름 | ✅ — matrix 가 `[parcels]` 만 표시 |
| SSOT | ❌ — workflow 가 `[parcels, admin, complex]` 박제 | ✅ — Rust const 단일 출처 |
| 명확성 | ❌ — TODO 가 *언젠가* 의미 | ✅ — ADR 0027 + `is_active_in_etl` 함수 |

## 클라이언트 영향

`apps/web/components/listings/listing-map.tsx` 의 admin/complex source 추가:

```ts
// listing-map.tsx:134, :157
mb.addSource("admin", { type: "vector", url: tileJsonUrl("admin") });
mb.addSource("complex", { type: "vector", url: tileJsonUrl("complex") });
```

본 호출은 `tileJsonUrl(layer)` 가 fetch 실패하면 graceful skip — manifest 에 admin/complex
artifact 가 없으면 `addSource` 시도 자체가 4xx 로 실패하고 client 가 layer 미렌더.
파일 자체에 명시적 source 존재 검사 추가 권장 (별도 sprint, 본 ADR 의 영향 범위 외).

## 향후 admin/complex source 결정 시

1. 새 ADR (0027 등) — 각 layer 의 source dsId / API endpoint / 라이선스 박제
2. `services/scraper-py/admin_vworld.py` 또는 동등 path 구현 (ADR 0022 패턴)
3. `Layer::is_active_in_etl()` 의 분기를 `true` 로 변경
4. workflow gold step 에 layer 별 BRONZE_PREFIX 분기 (parcels = dtmk, admin = 새 source, complex = 새 source)
5. 본 ADR 갱신 — *Status: Superseded by 0027*

## 영향

### 신규
- `docs/adr/0027-admin-complex-layer-source-deferred.md` (본 파일)
- `Layer::is_active_in_etl(self) -> bool` (`crates/sp9-base-layer-config/src/lib.rs`)
- `ConfigSnapshot.active_layers: Vec<String>` 필드
- `sp9-config-print active_layers` subcommand

### 수정
- `.github/workflows/sp9-base-layer-etl.yml`:
  - setup outputs 에 `active_layers_json` 추가
  - gold matrix 가 `active_layers_json` 사용 (이전 `layers_json`)
- `docs/adr/README.md` 인덱스

### 변경 없음
- 프론트 (`apps/web/components/listings/listing-map.tsx`) — manifest 의 `artifacts`
  에 admin/complex 가 없으면 자동 스킵 (graceful)
- 클라이언트 fetch 흐름 — manifest hot-swap 패턴 그대로

## 참고

- ADR 0021 — flat tile decomposition (admin/complex 도 같은 패턴 적용 예정)
- ADR 0022 — isolated Python scraper (admin/complex 새 script 도 동일 boundary)
- 사용자 메모리 (`feedback_sss_grade.md`) — "trick 1개라도 거부" 박제. 본 ADR 이
  trick 의 *공식 인정 + 차단 mechanism* 으로 fix.
