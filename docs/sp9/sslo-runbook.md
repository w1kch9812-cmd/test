# SP9 Base Layer — SLO + Runbook (Plan D L7)

> **갱신일**: 2026-05-07
> **Owner**: Platform / SP9
> **연계**: [sp9-base-layer-etl.yml](../../.github/workflows/sp9-base-layer-etl.yml) · [sp9-base-layer-rollback.yml](../../.github/workflows/sp9-base-layer-rollback.yml) · [crates/sp9-base-layer-config](../../crates/sp9-base-layer-config/) (SSOT)

본 문서가 SP9 base layer 의 *서비스 수준 목표 (SLO)* + *사고 대응 절차 (Runbook)* SSOT.
production deploy 또는 incident 대응 시 본 문서를 첫 reference 로.

---

## 1. SLO

### 1.1 클라이언트 측 (사용자 경험)

| 지표 | 목표 | 측정 방법 |
|---|---|---|
| 매물 페이지 첫 폴리곤 render TTI | **p50 ≤ 1.0s, p95 ≤ 2.5s** | Web Vitals + RUM (manifest fetch + 첫 tile fetch) |
| flat tile cache hit ratio | **≥ 95%** | Cloudflare Analytics (R2 origin pull 비율 보수) |
| 폴리곤 render 가시성 (z14+ 부평구) | **100%** | E2E `naver-all-features-probe.spec.ts` 매일 운영 ping |

### 1.2 ETL 측 (데이터 품질)

| 지표 | 목표 | 측정 방법 |
|---|---|---|
| 매월 cron 빌드 성공률 | **≥ 95%** (12개월 rolling) | etl.yml의 success/failure 비율 (Sentry release tracking) |
| Bronze → Gold 빌드 시간 | **≤ 4시간 p50, ≤ 8시간 p99** | etl.yml 의 timestamp 차이 |
| 강남 PNU `1168010100107370000` 등장 | **100%** (모든 prod build) | L2 verify spot-check (`gold/promote` 단계) |
| Manifest atomic flip | **100%** (no partial state in prod) | L3 promote 의 staging spec all-or-nothing |
| Bronze input fingerprint 박제 | **100%** (모든 manifest) | L10 lineage `bronze_inputs` non-empty for parcels |

### 1.3 인프라 (R2 / Cloudflare)

| 지표 | 목표 | 측정 방법 |
|---|---|---|
| R2 GET 5xx | **≤ 0.01%** (월 1M GET 기준 100 미만) | Cloudflare Analytics |
| R2 storage 용량 | **≤ 100GB** (12개월 rolling, L6 lifecycle 활성 기준) | R2 dashboard |
| R2 PUT 비용 | **≤ $5/월** | Cloudflare 청구 |
| CDN cache purge 성공률 | **≥ 99%** (manifest flip 시도) | promote 의 `cdn_purged` 결과 박제 |

### 1.4 Error budget

월 99% availability = error budget 7.2 시간/월. 빌드 실패 1회 = 평균 ~4시간 down (다음
빌드까지) → **월 1회 빌드 실패 까지 허용**. 2회 연속 = budget 소진 → freeze + post-mortem.

---

## 2. Runbook — incident 대응

### 2.1 "매물 페이지 폴리곤 안 보임"

**진단 절차**:

1. 클라이언트 측 일시적 문제 확인:
   ```bash
   curl -I https://r2.gongzzang.dev/gold/manifest.json
   # Cache-Control: no-cache, max-age=0 + 200 OK 여야 함.
   ```
2. manifest 의 `current_version` 확인:
   ```bash
   curl -s https://r2.gongzzang.dev/gold/manifest.json | jq .current_version
   ```
3. 지정 version 의 첫 tile 실재 확인:
   ```bash
   curl -I "https://r2.gongzzang.dev/gold/<version>/parcels/17/111789/50783.pbf"
   # 200 OK + `Content-Encoding: gzip` 여야 함.
   ```
4. 클라 console 에서 `addSource` 에러 확인 (mapbox-gl 에러).

**조치**:

- **manifest 가 최근 version 가리키지만 tile 404**: 이전 version 으로 rollback (§ 2.4).
- **manifest 자체 404**: R2 dashboard 에서 객체 존재 확인. promote 단계 실패한 것
  가능성 → re-run `etl.yml` workflow_dispatch 로 promote 만 다시.
- **클라 에러 (CORS / 404 stream)**: `R2_PUBLIC_URL_BASE` env 가 `apps/web` 에 정확
  설정됐는지 검증.

### 2.2 "ETL 매월 cron 실패 — Sentry alert"

**진단 절차**:

1. Sentry 의 incident detail 에서 어느 phase 실패인지 (bronze / gold / promote) 확인.
2. GitHub Actions run 의 log 점검 — 보통 다음 케이스:
   - **bronze 실패**: V-World 사이트 maintenance / 자격 만료 / Captcha 발동.
   - **gold 실패**: tippecanoe OOM (runner 격상 필요) / dtmk ZIP 일부 corrupt
     (Bronze re-fetch 필요).
   - **promote 실패**: staging spec 누락 (matrix 의 한 layer 가 fail-fast 로 silent
     skip 된 것 의심) / R2 권한 만료.

**조치**:

- **bronze 실패**: 다음 cron 까지 대기 (24시간 retry). 즉시 성공 필요 시:
  ```
  workflow_dispatch sp9-base-layer-etl.yml { bronze_skip: true }
  ```
  bronze 만 별도 실행 후 재시도.
- **gold OOM**: etl.yml 의 `runs-on: ubuntu-22.04-large` 확인 + GitHub billing 의
  large runner 활성 확인.
- **promote 실패 (한 layer staging 누락)**: gold matrix log 에서 어느 layer fail —
  해당 layer 만 단독 dispatch:
  ```
  workflow_dispatch sp9-base-layer-etl.yml { layers: "complex", bronze_skip: true }
  ```

### 2.3 "신규 build 가 잘못된 데이터 publish (예: 잘못된 SRS)"

**즉시 조치**: § 2.4 rollback.

**RCA**: L10 lineage 의 `source_srs` / `bronze_inputs` 검증 — 빌드 시점의 입력 fingerprint
가 manifest 에 박혀있어야 함.

### 2.4 Rollback — 이전 안정 버전으로 즉시 복구

**전제**:
- 이전 안정 version 의 staging spec 이 R2 에 *아직* 존재해야 함 (L6 lifecycle 가
  최소 2개 version 보존).
- 빌드 결과는 `gold/<version>/...` 에 immutable URL 로 남아있음 → manifest pointer
  만 변경하면 즉시 활성.

**실행**:

```
GitHub UI → Actions → "SP9 Base Layer Rollback" → Run workflow
  target_version: <이전 안정 버전, 예: v_2026_04>
  reason: <incident 식별자, 예: "INC-2026-005 매물 페이지 폴리곤 누락">
```

**확인**:

```bash
curl -s https://r2.gongzzang.dev/gold/manifest.json | jq .current_version
# → 입력한 target_version 과 일치 여야 함.
```

**소요 시간**: ~2 분 (promote subcommand + CDN purge).

### 2.5 Cloudflare R2 outage

**조치 우선순위**: 본 incident 는 SP9 단독 해결 불가 — Cloudflare status 의존.

1. https://www.cloudflarestatus.com 확인.
2. status page 에 reported = SP9 SLO 일시 침해 incident open + Sentry suppress.
3. status 복구 후: `gold/manifest.json` 자동 fetch 회복 (CDN 가 stale-while-revalidate).

**미래 대응 옵션 (별도 ADR)**: 멀티 리전 (R2 + AWS S3) replication, manifest CDN 의
secondary origin failover. 현재 미구현 (R2 SLA 99.9% 신뢰).

---

## 3. On-call 책임

- **1차 (월 cron)**: Platform team. 매월 1일 03:00 KST 직후 30분간 채널 대기.
- **2차 (즉시 대응)**: Sentry alert → on-call rotation (별도 oncall.md).
- **사용자 신고**: `# product-issues` channel → tier 1 triage → Platform 1차.

---

## 4. 운영 전 checklist (production go-live)

- [ ] `R2_*` secrets (Account ID / Access Key / Secret / Bucket) GitHub Actions 에 설정.
- [ ] `R2_PUBLIC_URL_BASE` 가 실 R2 public domain (또는 r2.dev subdomain) 으로 설정.
- [ ] `CLOUDFLARE_API_TOKEN` + `CLOUDFLARE_ZONE_ID` (선택, manifest CDN purge 활성).
- [ ] `SENTRY_DSN` 이 SP9 전용 project 또는 product project 의 SP9 tag 환경.
- [ ] `VWORLD_USERNAME` / `VWORLD_PASSWORD` 가 운영 계정 (개인 dev 계정 X).
- [ ] `ubuntu-22.04-large` runner 가 GitHub billing 에 활성.
- [ ] 첫 빌드는 `workflow_dispatch { target_version: v_dryrun_2026_05 }` 로 staging
      검증 후 `gold/manifest.json` 미수정. 이후 manual `promote` 호출.
- [ ] § 2.4 rollback workflow 가 dispatch 가능한 권한 (Actions write) 부여 확인.
- [ ] 본 runbook 의 § 2 의 4 incident scenario 가 실제 staging 에서 1번씩 시뮬레이션
      완료 (특히 § 2.4 rollback 절차).
