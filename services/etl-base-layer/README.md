# etl-base-layer

PMTiles base layer ETL — Bronze SHP 다운로드 / Gold PMTiles 빌드 / R2 업로드.

ADR [0016 PMTiles 100%](../../docs/adr/0016-medallion-base-layer-postgis-silver-pmtiles-gold.md) 의 ETL 측 구현. SP9 plan T3.

## 진행 단계

- ✅ **T3a (현재)** — Bronze 다운로드 + sha256 + 로컬 manifest.json
- ⏳ **T3b** — SHP → GeoJSON (`ogr2ogr` spawn, EPSG:5179 → EPSG:4326)
- ⏳ **T3b** — `tippecanoe` spawn → `parcels.pmtiles` / `admin.pmtiles` / `complex.pmtiles`
- ⏳ **T3b** — R2 업로드 (Bronze archive + Gold artifacts)
- ⏳ **T3b** — Gold manifest hot-swap (`current_version` 갱신)
- ⏳ **T3b** — 검증 (강남 PNU 존재 + sha256 + row count 변동 < 5%)

## 환경변수 (T3a)

| 변수 | 필수 | 기본 |
|---|---|---|
| `BRONZE_DIR` | — | `./var/bronze` |
| `BRONZE_BATCH_LABEL` | — | UTC `%Y-%m` |
| `BRONZE_PARCEL_SHP_URL` | optional | — |
| `BRONZE_ADMIN_SHP_URL` | optional | — |
| `BRONZE_COMPLEX_GEOJSON_URL` | optional | — |

세 URL 중 하나도 없으면 fail-fast (exit 2).

## 로컬 실행

```bash
BRONZE_PARCEL_SHP_URL=https://www.data.go.kr/.../parcel.shp.zip \
BRONZE_ADMIN_SHP_URL=https://www.data.go.kr/.../admin.shp.zip \
BRONZE_DIR=./var/bronze \
cargo run -p etl-base-layer
```

결과:
```
./var/bronze/<batch_label>/parcel.shp.zip
./var/bronze/<batch_label>/admin.shp.zip
./var/bronze/<batch_label>/manifest.json
```

`manifest.json` 포맷:
```json
{
  "batch_label": "2026-05",
  "batch_started_at": "2026-05-06T...",
  "sources": {
    "admin": { "url": "...", "filename": "admin.shp.zip", "bytes": 32768, "sha256": "...", "downloaded_at": "..." },
    "parcel": { "url": "...", "filename": "parcel.shp.zip", "bytes": 524288, "sha256": "...", "downloaded_at": "..." }
  }
}
```

## 검증 (T3a)

- `cargo test -p etl-base-layer` — manifest serde 2 단위 테스트
- `cargo clippy -p etl-base-layer --all-targets -- -D warnings` — 통과
- 로컬 smoke — `https://www.google.com/robots.txt` 다운로드 + sha256 계산 + manifest 저장 확인

## 후속 (T3b/T6)

T3b 추가 의존:
- `aws-sdk-s3` (R2 업로드, S3-compatible API)
- `tokio::process::Command` (ogr2ogr / tippecanoe spawn)
- CI: `apt install gdal-bin`, tippecanoe build from `felt/tippecanoe` source

T6 (`.github/workflows/sp9-base-layer-etl.yml`):
- 매월 1일 03:00 KST cron
- `ubuntu-22.04-large` (32GB RAM — 1.4억 polygon)
- timeout 720분
- 실패 시 Sentry 알림
