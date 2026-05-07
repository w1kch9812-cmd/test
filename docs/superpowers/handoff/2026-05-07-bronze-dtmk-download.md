# Handoff — V-World dtmk Bronze 다운로드 진행 중 (2026-05-07)

> **상태**: dtmk_vworld.py 작성 완료, R2 다운로드 *background 진행 중* 또는 *진행 예정*.
> **다음 세션**: R2 의 Bronze 데이터 기반 ETL Gold pipeline + parcel-lookup DB 적재.

## 진행 완료 (이번 세션)

- ✅ ADR 0022 — Bronze scraping = 격리 Python service ([0022-bronze-scraping-isolated-python-service.md](../../adr/0022-bronze-scraping-isolated-python-service.md))
- ✅ `services/scraper-py/` 신규 — Scrapling + curl_cffi + boto3 venv
- ✅ `dtmk_vworld.py` — V-World 로그인 + 273 SHP zip → R2 Bronze 자동 다운로드
- ✅ V-World 로그인 + 단일 다운 검증 (충북 충주시 52.3MB ZIP, ZIP magic OK)
- ✅ subprocess pattern — Rust ETL 이 Python script spawn (tippecanoe 와 동일)

## 진행 중 / 진행 예정 — Bronze 다운로드

```bash
cd services/scraper-py
.venv/Scripts/python dtmk_vworld.py
```

- V-World dtmk dsId=30563 (연속지적도_전국, 273 시군구 SHP zip)
- 합계 ~5-10GB
- concurrent 3 parallel → 30-60분 예상
- R2 키: `bronze/<YYYY-MM>/parcel-dtmk-30563/LSMD_CONT_LDREG_<sigungu>.zip`
- idempotent skip (같은 size 면 다운 X) — daily diff cron 패턴

## 다음 세션 작업 (이어서)

### 1. R2 Bronze 검증
```bash
# AWS CLI 또는 wrangler 로 list
aws --endpoint-url https://${R2_ACCOUNT_ID}.r2.cloudflarestorage.com \
    s3 ls s3://gongzzang/bronze/2026-05/parcel-dtmk-30563/
# 273 객체 확인
```

또는 Python 으로:
```python
import boto3
r2 = boto3.client("s3", endpoint_url=f"https://{R2_ACCOUNT_ID}.r2.cloudflarestorage.com", ...)
resp = r2.list_objects_v2(Bucket="gongzzang", Prefix="bronze/2026-05/parcel-dtmk-30563/")
print(len(resp.get("Contents", [])), "objects")
```

### 2. ETL Gold pipeline 확장 — *전국* 필지 빌드

현재 ETL 은 *single GeoJSON input* (`./var/sample/gangnam.geojson`). 전국 273 SHP zip 처리 path:

```
[R2 Bronze] (273 zip)
  ↓ services/etl-base-layer/src/bronze/dtmk.rs (신규)
  ↓ - R2 list_objects_v2 → 273 zip 다운 (또는 선택적 다운)
  ↓ - 임시 dir 에 unzip
[unzipped SHP files]
  ↓ ogr2ogr (이미 구현) — EPSG:5186 → EPSG:4326, SHP → GeoJSON
[GeoJSON files]
  ↓ tippecanoe (단일 빌드 또는 시군구별 부분 빌드)
[parcels.pmtiles]
  ↓ tile-join → flat tiles
[R2 Gold] gold/v<N>/parcels/{z}/{x}/{y}.pbf
  + TileJSON, manifest.json (ADR 0021)
```

CLI 추가 필요:
```bash
cargo run -p etl-base-layer -- gold \
    --layer parcels \
    --bronze-prefix bronze/2026-05/parcel-dtmk-30563/ \
    --output ./var/gold/v_full \
    --extract-and-build  # 신규 flag
```

### 3. `parcel-lookup` crate 의 V-World API 의존 폐기

사용자 정신: *정적 데이터는 우리 저장, runtime API 호출 0*.

현재 [crates/parcel-lookup/src/vworld_lookup.rs](../../../crates/parcel-lookup/src/vworld_lookup.rs) = runtime V-World API 호출. 폐기 path:

1. ETL Silver — `parcel` 테이블 적재 (PNU + 지번 + 좌표 + jiga + geom)
   - `services/etl-base-layer/src/silver/parcel.rs` 신규
   - SHP → sqlx INSERT (273 시군구 합계 ~3.7천만 row)
2. `crates/parcel-lookup` — `DbParcelLookup` 신규 (Postgres)
3. `vworld_lookup.rs` 폐기 또는 *DB miss 시 fallback* 만 유지

### 4. Daily diff cron (T6)

`.github/workflows/sp9-bronze-dtmk-daily.yml`:
- 매일 03:00 KST
- `python dtmk_vworld.py` — fileSize 비교 후 변경 시군구만 부분 다운
- manifest.json 의 `last_known_sizes` 갱신
- 변경 0 = 5초 + 0원
- 변경 시 → ETL Gold 부분 rebuild trigger (또는 별도 workflow)

### 5. 검증 & 시각

- 브라우저 [http://localhost:3000/listings](http://localhost:3000/listings) — 강남 → 부산 → 제주 panning, 끊김 0, 폴리곤 모든 시군구
- Naver SDK + mapbox-gl 자동 TileJSON fetch — 우리 클라 코드 변경 0

## 환경변수 (변경 없음, .env 그대로)

```
VWORLD_USERNAME=kchls9812
VWORLD_PASSWORD=...
R2_ACCOUNT_ID=04c4f2ac7fd14b897ec0a4b0402b9cba
R2_ACCESS_KEY=...
R2_SECRET_KEY=...
R2_BUCKET=gongzzang
R2_PUBLIC_URL_BASE=https://pub-d636264a6ac64192b87addba0464d712.r2.dev
GOLD_VERSION=v1
```

## 진입점 (다음 세션 시작)

```bash
# 1. R2 Bronze 의 273 객체 list 확인 (다운 완료 검증)
ls services/scraper-py/   # script 위치

# 2. ETL 확장 작업 시작
cd services/etl-base-layer
# bronze/dtmk.rs 신규 작성
# main.rs 의 gold 서브커맨드에 --bronze-prefix 추가

# 3. parcel-lookup crate 의 DB 우선 path 작성
cd crates/parcel-lookup
# silver/parcel.rs (ETL) + DbParcelLookup
```

## 관련 문서

- [ADR 0016](../../adr/0016-medallion-base-layer-postgis-silver-pmtiles-gold.md) — Bronze/Gold medallion
- [ADR 0021](../../adr/0021-static-vector-tile-decomposition.md) — flat .pbf SSS path
- [ADR 0022](../../adr/0022-bronze-scraping-isolated-python-service.md) — scraping 격리 Python
- [services/scraper-py/README.md](../../../services/scraper-py/README.md) — Python 운영 가이드
