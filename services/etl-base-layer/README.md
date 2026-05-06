# etl-base-layer

PMTiles base layer ETL — Bronze SHP 다운로드 / Gold PMTiles 빌드 / R2 업로드.

ADR [0016 PMTiles 100%](../../docs/adr/0016-medallion-base-layer-postgis-silver-pmtiles-gold.md) 의 ETL 측 구현. SP9 plan T3.

## 진행 단계

- ✅ **T3a** — Bronze 다운로드 + sha256 + 로컬 manifest.json
- ✅ **T3b.1 (현재)** — R2 업로드 path (`aws-sdk-s3`) + Bronze archive + manifest R2 upload + GoldManifest 데이터 모델
- ⏳ **T3b.2** — SHP → GeoJSON (`ogr2ogr`, EPSG:5179 → EPSG:4326), `tippecanoe` spawn (parcels Z14-17 / admin Z6-12 / complex Z10-15), Gold artifact R2 upload, verify (강남 PNU 1168010100107370000 + row count Δ < 5%), Gold manifest hot-swap (`current_version` 갱신)

## 환경변수

### Bronze (T3a)

| 변수 | 필수 | 기본 |
|---|---|---|
| `BRONZE_DIR` | — | `./var/bronze` |
| `BRONZE_BATCH_LABEL` | — | UTC `%Y-%m` |
| `BRONZE_PARCEL_SHP_URL` | optional | — |
| `BRONZE_ADMIN_SHP_URL` | optional | — |
| `BRONZE_COMPLEX_GEOJSON_URL` | optional | — |

세 URL 중 하나도 없으면 fail-fast (exit 2).

### R2 업로드 (T3b.1)

R2 4-tuple 이 *모두* 설정되어야 활성. 부분 설정은 fallback 으로 *비활성* (실수로 partial 자격이 commit 되어도 silent skip — 비밀 유출 방지).

| 변수 | 필수 | 기본 |
|---|---|---|
| `R2_ACCOUNT_ID` | R2 활성 시 | — |
| `R2_ACCESS_KEY` | R2 활성 시 | — |
| `R2_SECRET_KEY` | R2 활성 시 | — |
| `R2_BUCKET` | R2 활성 시 | — |
| `R2_BRONZE_PREFIX` | — | `bronze` |
| `R2_GOLD_PREFIX` | — | `gold` |
| `GOLD_VERSION` | — | (T3b.2 의 빌드 시점에 결정) |

### R2 KEY 레이아웃 (ADR 0016)

```text
<bucket>/<bronze_prefix>/<batch_label>/parcel.shp.zip
<bucket>/<bronze_prefix>/<batch_label>/admin.shp.zip
<bucket>/<bronze_prefix>/<batch_label>/industrial-complex.geojson
<bucket>/<bronze_prefix>/<batch_label>/manifest.json     # Bronze manifest (hot-cache)

<bucket>/<gold_prefix>/<version>/parcels.pmtiles         # T3b.2
<bucket>/<gold_prefix>/<version>/admin.pmtiles           # T3b.2
<bucket>/<gold_prefix>/<version>/complex.pmtiles         # T3b.2
<bucket>/<gold_prefix>/manifest.json                     # T3b.2 (hot-swap pointer)
```

## 로컬 실행

R2 미설정 (T3a 호환) — 로컬 파일만 만듦:

```bash
BRONZE_PARCEL_SHP_URL=https://www.data.go.kr/.../parcel.shp.zip \
BRONZE_DIR=./var/bronze \
cargo run -p etl-base-layer
```

R2 활성:

```bash
BRONZE_PARCEL_SHP_URL=https://www.data.go.kr/.../parcel.shp.zip \
BRONZE_ADMIN_SHP_URL=https://www.data.go.kr/.../admin.shp.zip \
BRONZE_DIR=./var/bronze \
R2_ACCOUNT_ID=<cloudflare_account> \
R2_ACCESS_KEY=<r2_access_key> \
R2_SECRET_KEY=<r2_secret_key> \
R2_BUCKET=gongzzang-static \
cargo run -p etl-base-layer
```

결과 (R2 활성):

```text
# 로컬
./var/bronze/<batch_label>/parcel.shp.zip
./var/bronze/<batch_label>/admin.shp.zip
./var/bronze/<batch_label>/manifest.json

# R2
gongzzang-static/bronze/<batch_label>/parcel.shp.zip       (Content-Type: application/zip)
gongzzang-static/bronze/<batch_label>/admin.shp.zip
gongzzang-static/bronze/<batch_label>/manifest.json        (Cache-Control: no-cache)
```

## 검증

- `cargo test -p etl-base-layer` — 12 unit tests (manifest serde, env config, R2 mock, bronze key layout)
- `cargo clippy -p etl-base-layer --all-targets -- -D warnings` — 통과
- 로컬 smoke (T3a 호환) — 임의의 작은 url 다운로드 + sha256 계산 + manifest 저장 확인
- R2 smoke (T3b.1) — wiremock 으로 PUT 1회 검증, 5xx 응답 propagation 검증

## 의존성 핀

`aws-sdk-s3 1.110+` / `aws-config 1.8.6+` / `aws-smithy-async 1.2.11+` 가 rustc 1.91 을 요구.
workspace `[rust-toolchain.toml]` = 1.88.0 → pre-MSRV-bump 으로 핀:

| crate | 핀 | 사유 |
|---|---|---|
| `aws-sdk-s3` | `=1.86.0` | rust-version 1.82 |
| `aws-config` | `=1.8.5` | rust-version 1.86 |
| `aws-credential-types` | `=1.2.5` | resolver 호환 |
| `aws-smithy-types` | `=1.3.4` | resolver 호환 |
| `aws-smithy-async` (Cargo.lock) | `=1.2.10` | rust-version 1.88 |

workspace MSRV 가 1.91 로 올라가면 모두 latest 풀어도 됨.

## 후속 (T3b.2 / T6)

T3b.2 추가 의존:

- `tokio::process::Command` (ogr2ogr / tippecanoe spawn)
- pmtiles parser (`pmtiles-rs` 또는 ogrinfo 결과 파싱) — verify 단계의 강남 PNU lookup
- CI: `apt install gdal-bin`, tippecanoe build from `felt/tippecanoe` source

T6 (`.github/workflows/sp9-base-layer-etl.yml`):

- 매월 1일 03:00 KST cron
- `ubuntu-22.04-large` (32GB RAM — 1.4억 polygon)
- timeout 720분
- 실패 시 Sentry 알림
