# scraper-py — 격리 Python scraping service

**역할**: 메인 Rust 시스템 (`crates/`, `services/api`, `services/etl-base-layer`) 와 *격리된* Python service. *anti-bot bypass 가 필요한 정부/HTML 사이트* 의 자동 다운 — Scrapling community 의 stealth/adaptive selectors 그대로 활용.

## 정책 (AGENTS.md § 1 부합)

- 메인 Rust 시스템에 Python dep 추가 0
- Python 격리 = subprocess pattern (`tippecanoe` / `ogr2ogr` 와 동급 외부 도구)
- ETL Rust orchestrator 가 spawn → 결과 (R2 key) 받음

## 구성

- `dtmk_vworld.py` — V-World dtmk SHP zip 자동 다운 → R2 Bronze archive
  - 사용자 ID/PW 로그인 (사이트, api.vworld.kr 와 별개)
  - 273 시군구 별 파일 list 추출 + concurrent 다운 + R2 PUT (streaming)
  - idempotent skip — 같은 size 의 object 가 R2 에 있으면 skip
  - 매일 cron 가능 (size 변경 시군구만 부분 다운)

## 실행

```bash
cd services/scraper-py
.venv/Scripts/pip install -r requirements.txt   # 첫 1회
.venv/Scripts/python dtmk_vworld.py
```

`.env` (repo root) 의 환경변수 자동 로드:
- `VWORLD_USERNAME` / `VWORLD_PASSWORD` — V-World 사이트 계정
- `R2_ACCOUNT_ID` / `R2_ACCESS_KEY` / `R2_SECRET_KEY` / `R2_BUCKET` — Cloudflare R2
- `R2_BRONZE_PREFIX` (default `bronze`)
- `DTMK_PARALLEL` (default 3, V-World 부담 고려)

## R2 key 레이아웃

```
<bronze_prefix>/<YYYY-MM>/parcel-dtmk-<ds_id>/LSMD_CONT_LDREG_<시군구>.zip
                                                ↑
                          273 시군구 (예: 충북_충주시, 강원_원주시)
```

## 다음 작업 (이어지는 ETL Gold pipeline)

`services/etl-base-layer/` (Rust) 가 위 R2 Bronze 를 input 으로 받아:

1. R2 → 임시 디렉토리 다운로드
2. `unzip` → SHP files
3. `ogr2ogr` → GeoJSON (EPSG:5186 → EPSG:4326)
4. `tippecanoe` → `parcels.pmtiles`
5. `tile-join --output-to-directory` → flat `{z}/{x}/{y}.pbf`
6. R2 PUT + TileJSON publish (ADR 0021)

ETL CLI: `cargo run -p etl-base-layer -- gold --layer parcels --bronze-prefix gold/<batch>/parcel-dtmk-30563/ ...` (구현 예정).

## subprocess pattern (Rust 가 spawn)

```rust
// services/etl-base-layer/src/bronze/dtmk.rs (예정)
let cmd = build_command(host, "python", &[
    Arg::Path(scripts_dir.join("dtmk_vworld.py").as_path()),
]);
// .env 자동 로드 (Python 측에서)
let out = cmd.output().await?;
let summary: DtmkSummary = serde_json::from_slice(&out.stdout)?;
```

향후 `services/scraper-py/` 추가 script:
- `court_auction.py` — 대법원 경매 (Scrapling stealth — anti-bot 강한 사이트)
- 다른 정부 HTML 사이트 scraping
