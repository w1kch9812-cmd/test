# ADR 0022 — Bronze scraping = 격리 Python service (`services/scraper-py/`)

| | |
|---|---|
| 작성일 | 2026-05-07 |
| 상태 | Accepted |
| 선행 | [ADR 0016](./0016-medallion-base-layer-postgis-silver-pmtiles-gold.md), [ADR 0021](./0021-static-vector-tile-decomposition.md), [AGENTS.md § 1](../../AGENTS.md) |

## 결정

*HTML scraping + anti-bot bypass + 정부 사이트 자동 다운* 이 필요한 ETL Bronze 작업은 **격리된 Python service** (`services/scraper-py/`) 에서 [Scrapling](https://github.com/D4Vinci/Scrapling) 라이브러리로 구현. 메인 Rust 시스템 (`crates/`, `services/api`, `services/etl-base-layer`) 에 Python 의존성 0.

ETL Rust orchestrator (`services/etl-base-layer/`) 가 `tippecanoe` / `ogr2ogr` / `tile-join` 와 *동일한 subprocess pattern* 으로 Python script 를 spawn → JSON summary 받음.

## 컨텍스트

### 사용자 needs (2026-05-07 세션)

1. **연속지적도 전국 SHP zip** (V-World dtmk dsId=30563, 273 파일 ~5-10GB) 자동 다운 + R2 영구 저장 → 우리 PMTiles 의 input
2. **runtime API 호출 0** — 정적 데이터 (필지 polygon, 행정구역, 산단) 는 *모두 우리 R2/DB 영구 저장*. 외부 API 의존 0
3. **매일 daily diff cron** — V-World dtmk 의 fileSize 변경 detect → 변경 시군구만 부분 다운
4. **미래: 대법원 경매 scraping** — anti-bot 강함, Scrapling 의 stealth 필수

### 검토한 대안

#### A. Rust 직접 구현 (`reqwest` + `cookie_store` + `scraper` crate)
- 장점: 메인 언어 일관성, single binary, dependency tree 단순
- 단점:
  - **Reinvent the wheel** — Scrapling 의 anti-bot bypass 패치를 우리가 따라잡아야 함 (Cloudflare/Akamai 가 *상시 패치*)
  - 사용자 통찰: *"러스트로 새로 만드는건 별로인거같아, 저거 업데이트해도 우리가만든건 업데이트도 안될거아니야"* — 정확
  - 경매 같은 *anti-bot 강한 사이트* 에서는 우리 재구현이 *항상 뒤처짐*
- **거부 이유**: 미래 대법원 경매 (anti-bot 강함) 에서 무조건 재구현 부담 발생. V-World (지금) 도 정부 정책 변화 시 anti-bot 강화 가능성

#### B. 메인 Rust crate 안에 Python embed (PyO3)
- 장점: single binary 환상
- 단점:
  - Rust binary 가 Python runtime 의존 → 배포 시 *Python 환경 + venv* 동봉
  - 의존성 격리 안 됨 (Python lib 충돌이 Rust 빌드까지 영향)
  - PyO3 의 GIL 핸들링 복잡
- **거부 이유**: AGENTS.md § 1 의 "메인 시스템 의존성 0" 정책과 강하게 충돌

#### C. 격리 Python service + subprocess pattern (본 ADR 채택)
- 장점:
  - Scrapling 그대로 사용 (anti-bot 패치 자동 — `pip install --upgrade scrapling`)
  - 메인 Rust 시스템 영향 0 (`services/scraper-py/` 만 Python)
  - tippecanoe / ogr2ogr 와 *동일 subprocess pattern* — ETL Rust 의 orchestrator 가 spawn
  - 격리 — Python venv 의 의존성이 Rust crate `Cargo.toml` 에 안 섞임
- 단점:
  - Docker 이미지 +50-200MB (Python runtime + Scrapling deps)
  - CI 빌드 +2-5분 (pip install)
- **채택 이유**: anti-bot 패치 자동성 + 격리 + 패턴 일관성. *진짜 SSS = 표준 도구 그대로 + 우리 glue 코드 최소*.

#### D. 외부 SaaS scraping (예: ScrapingBee, Apify)
- **거부**: 비용 + 정부 사이트 약관 + 우리 데이터 제3자 노출 risk

## 검증 (2026-05-07 spike)

V-World dtmk dsId=30563 검증:

1. **Scrapling `Fetcher.get`** → 200 OK, HTML 정상 fetch (0 anti-bot)
2. **`/v4po_main.do`** DynamicFetcher (Playwright) 로 JS 렌더 → `loginFnc.login(...)` 발견
3. **로그인 endpoint**: `POST /v4po_usrlogin_a004.do` (base64 encoded usrIdeE/usrPwdE)
4. **로그인 응답**: `{resultMap: {result: 'success', usrNam: '...'}}` JSON
5. **다운로드 endpoint**: `GET /dtmk/downloadResourceFile.do?ds_id=&fileNo=` (with session cookie)
6. **응답**: `Content-Type: application/download`, `Content-Disposition: filename=LSMD_CONT_LDREG_<sigungu>.zip`, ZIP magic `50 4b 03 04` ✓
7. **검증된 결과**: 충북 충주시 = 52.3MB ZIP 다운 성공

## 채택

### 디렉토리

```
services/scraper-py/
├── .venv/                  # gitignored
├── .gitignore
├── README.md               # 운영 가이드
├── requirements.txt        # scrapling, curl_cffi, boto3
├── dtmk_vworld.py          # 본 ADR 의 첫 구현 — V-World dtmk SHP zip → R2
└── (미래)
    ├── court_auction.py    # 대법원 경매 (anti-bot 강함)
    └── ...
```

### Python 측 책임

- HTML scraping (Scrapling)
- form login + session cookie persist (curl_cffi)
- streaming download → R2 PUT (boto3, S3-compatible)
- idempotent skip (R2 의 같은 size object 면 skip)
- summary JSON stdout (Rust 가 parse)

### Rust 측 책임 (ETL orchestrator)

- subprocess spawn (`tippecanoe` 와 동일 pattern):
  ```rust
  build_command(host, "python", &[
      Arg::Path(scripts_dir.join("services/scraper-py/dtmk_vworld.py").as_path())
  ])
  ```
- stdout JSON parse → manifest 갱신
- R2 Bronze prefix 의 zip → ogr2ogr → tippecanoe → flat tile (Gold)
- TileJSON publish (ADR 0021)

### 데이터 흐름 (전체)

```
[V-World dtmk 사이트]
  ↓ Scrapling (Python 격리 service)
[R2 Bronze archive]
  bronze/<YYYY-MM>/parcel-dtmk-30563/LSMD_CONT_LDREG_<sigungu>.zip
  ↓ Rust ETL (services/etl-base-layer)
  ↓ unzip → ogr2ogr → tippecanoe → tile-join
[R2 Gold]
  gold/v<N>/parcels/{z}/{x}/{y}.pbf  (flat vector tile, ADR 0021)
  gold/v<N>/parcels.json             (TileJSON, ADR 0021)
  gold/manifest.json                 (artifact 메타 + sha256)
  ↓ 클라
[Naver SDK + mapbox-gl 자동 fetch]
  지도에 전국 필지 폴리곤
```

## 영향

### 신규
- `services/scraper-py/` 패키지 (README, requirements.txt, .gitignore, dtmk_vworld.py)
- `docs/adr/0022-bronze-scraping-isolated-python-service.md` (본 파일)

### 수정 (다음 세션)
- `services/etl-base-layer/src/bronze/dtmk.rs` 신규 — Rust 가 Python script spawn
- `services/etl-base-layer/src/main.rs` — `bronze --source dtmk-vworld` subcommand
- `crates/parcel-lookup/` — runtime V-World API 의존 폐기, **DB 우선 lookup** (사용자 정신: *정적 데이터 = 우리 저장, runtime API 호출 0*)

### 폐기 (검토)
- `services/etl-base-layer/scripts/fetch-vworld-sig.mjs` — Node prototype, 본 ADR 채택 후 삭제

## 후속

### Daily diff cron (T6 일부)
- 매일 03:00 KST cron (GitHub Actions)
- dtmk 페이지 fileSize 비교 → 변경 시군구만 부분 다운
- 변경 0 = 5초 + 0원
- 변경 1 시군구 = 그 zip 만 + Gold 부분 rebuild
- manifest.json 의 `last_known_sizes` 가 SSOT

### 다음 sub-project (SP-court-auction)
- `services/scraper-py/court_auction.py`
- 대법원 경매정보 (anti-bot 강함 — Scrapling 의 stealth 필수)
- *우리 platform 의 매물 검색에 경매 매물 포함*

### 메인 시스템 영향 0 검증
- `services/api/` (Rust 백엔드) — Python dep 추가 0
- `services/etl-base-layer/Cargo.toml` — Python 의존 0 (subprocess spawn 만)
- `crates/` — Python dep 0
- `apps/web/` — Python dep 0
- AGENTS.md § 1 정책 부합 ✅

## 참고

- Scrapling repo: https://github.com/D4Vinci/Scrapling
- V-World dtmk dsId=30563 (연속지적도_전국, 273 SHP zip): https://www.vworld.kr/dtmk/dtmk_ntads_s002.do?dsId=30563
- 검증 spike code: `var/scrapling-test/` (gitignored)
