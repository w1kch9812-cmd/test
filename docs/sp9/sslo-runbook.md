# SP9 Base Layer — SLO + Runbook (Plan D L7)

> **갱신일**: 2026-05-08 (Round 4 enterprise audit — secret rotation / backup retention / compliance / DR 박제)
> **Owner**: Platform / SP9
> **연계**: [sp9-base-layer-etl.yml](../../.github/workflows/sp9-base-layer-etl.yml) · [sp9-base-layer-rollback.yml](../../.github/workflows/sp9-base-layer-rollback.yml) · [crates/sp9-base-layer-config](../../crates/sp9-base-layer-config/) (SSOT)
>
> **Platform-core handover**: `gold/manifest.json` promote, rollback, and manifest
> backup cleanup are no longer operated from Gongzzang. Gongzzang is a manifest
> consumer only. Use platform-core Catalog APIs and runbooks for manifest pointer
> changes.

본 문서가 SP9 base layer 의 *서비스 수준 목표 (SLO)* + *사고 대응 절차 (Runbook)* SSOT.
production deploy 또는 incident 대응 시 본 문서를 첫 reference 로.

---

## 1. SLO

### 1.1 클라이언트 측 (사용자 경험)

| 지표 | 목표 | 측정 방법 |
|---|---|---|
| 매물 페이지 첫 폴리곤 render TTI | **p50 ≤ 1.0s, p95 ≤ 2.5s** | Web Vitals + RUM (manifest fetch + 첫 tile fetch) |
| flat tile cache hit ratio | **≥ 95%** | Cloudflare Analytics (R2 origin pull 비율 보수) |
| 폴리곤 render 가시성 (z14+ 부평구) | **100%** | Probe `pnpm --filter @gongzzang/web probe:naver` 매일 운영 ping |

### 1.2 ETL 측 (데이터 품질)

| 지표 | 목표 | 측정 방법 |
|---|---|---|
| 매월 cron 빌드 성공률 | **≥ 95%** (12개월 rolling) | etl.yml의 success/failure 비율 (Sentry release tracking) |
| Bronze → Gold 빌드 시간 | **≤ 4시간 p50, ≤ 8시간 p99** | etl.yml 의 timestamp 차이 |
| 강남 PNU `1168010100107370000` 등장 | **100%** (모든 prod build) | L2 verify spot-check + platform-core manifest promote smoke |
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
- **manifest 자체 404**: R2 dashboard 에서 객체 존재 확인. platform-core Catalog
  outbox publisher / R2 pointer write 상태를 확인하고, 필요 시 platform-core
  `PUT /catalog/v1/vector-tiles/manifest:promote` 로 재승격한다. Gongzzang ETL
  promote workflow_dispatch 는 사용하지 않는다.
- **클라 에러 (CORS / 404 stream)**: `R2_PUBLIC_URL_BASE` env 가 `apps/web` 에 정확
  설정됐는지 검증.

### 2.2 "ETL 매월 cron 실패 — Sentry alert"

**진단 절차**:

1. Sentry 의 incident detail 에서 어느 phase 실패인지 (bronze / gold / platform-core pointer) 확인.
2. GitHub Actions run 의 log 점검 — 보통 다음 케이스:
   - **bronze 실패**: V-World 사이트 maintenance / 자격 만료 / Captcha 발동.
   - **gold 실패**: tippecanoe OOM (runner 격상 필요) / dtmk ZIP 일부 corrupt
     (Bronze re-fetch 필요).
   - **manifest pointer 실패**: platform-core Catalog promote/outbox/R2 publisher 상태 확인.
     Gongzzang promote job 은 ADR 0036 이후 비활성이다.

**조치**:

- **bronze 실패**: 다음 cron 까지 대기 (24시간 retry). 즉시 성공 필요 시:
  ```
  workflow_dispatch sp9-base-layer-etl.yml { bronze_skip: true }
  ```
  bronze 만 별도 실행 후 재시도.
- **gold OOM**: etl.yml 의 `runs-on: ubuntu-22.04-large` 확인 + GitHub billing 의
  large runner 활성 확인.
- **manifest pointer 실패**: Gongzzang 에서 재-promote 하지 않는다. platform-core
  Catalog active manifest, outbox event, R2 pointer write를 확인한다. tile artifact 재빌드가
  필요할 때만 해당 layer 를 단독 dispatch:
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

**소요 시간**: platform-core rollback API + outbox publisher R2 pointer write 기준으로
측정한다. Gongzzang `promote` subcommand 는 사용하지 않는다.

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
- [ ] platform-core `PUT /catalog/v1/vector-tiles/manifest:promote` smoke test 완료.
- [ ] platform-core `POST /catalog/v1/vector-tiles/manifest:rollback` smoke test 완료.
- [ ] 본 runbook 의 § 2 의 4 incident scenario 가 실제 staging 에서 1번씩 시뮬레이션
      완료 (특히 § 2.4 rollback 절차).

---

## 5. Secret rotation 정책 (Round 4 enterprise audit)

**원칙**: 모든 production secret 은 *명시 회전 주기* + *dual-secret window* (overlap
window 동안 old/new 양쪽 valid) 박제. 회전 시 ETL 무중단 보장.

| Secret | 회전 주기 | Dual window | 회전 절차 |
|---|---|---|---|
| `R2_ACCESS_KEY` / `R2_SECRET_KEY` | 90일 | 7일 (old/new 동시 활성) | Cloudflare R2 dashboard → 새 token 발급 → GHA secret 갱신 → 다음 cron 검증 → 7일 후 old token 폐기 |
| `VWORLD_USERNAME` / `VWORLD_PASSWORD` | **연 1회 또는 incident 시** | 없음 (계정 자체) | V-World 계정 비밀번호 변경 → GHA secret 갱신 → 즉시 cron dispatch 검증 |
| `CLOUDFLARE_API_TOKEN` | 90일 | 7일 | Cloudflare → API tokens → 새 token (zone purge scope) → GHA secret 갱신 → 다음 promote 검증 → 7일 후 old token revoke |
| `SENTRY_DSN` | 무한 (project 단위) | N/A | 회전 불필요. project 분리 시에만 갱신 |
| `GITHUB_TOKEN` | workflow scope (자동) | N/A | GHA 가 자동 |

**실패 시 fallback**:
- R2 token 회전 후 next cron 실패 → manifest 변경 0 (L3 atomicity) → 클라이언트 영향 0
- V-World password 회전 후 phase 1 실패 → bronze refresh skip (`bronze_skip=true`) +
  기존 R2 zip 재사용 → 다음 manual run 으로 복구

**audit log**: 각 secret 회전 시 `docs/sp9/secret-rotation-history.md` 에 (date, who,
which secret) 박제. PIPC 감사 대비.

---

## 6. Backup retention + RPO / RTO (Round 4 enterprise audit)

### 6.1 Manifest backup chain

`gold/manifest.json` pointer lifecycle 과 backup cleanup 은 platform-core Catalog 가
담당한다. Gongzzang 의 legacy manifest backup cleanup workflow 와 CLI 는 비활성이다.

| 항목 | 정책 |
|---|---|
| 보존 개수 | **최근 12개 manifest** (1년치, 매월 1회 cron 기준) |
| 보존 mechanism | R2 immutable URL — flat tile 은 자동 보존, manifest backup 은 명시 PUT |
| 청소 정책 | platform-core Catalog manifest backup cleanup policy |
| 복구 절차 | platform-core rollback API with `expected_current_version` |

**RPO** (Recovery Point Objective): **0 분** — manifest atomic flip 직전 상태 = R2 의
`manifest.<prev>.json` 그대로. 데이터 손실 0.

**RTO** (Recovery Time Objective): **5 분** — § 2.4 workflow run 시간 (manifest 1개
PUT + Cloudflare CDN purge).

### 6.2 Flat tile 보존

`gold/v<N>/<layer>/{z}/{x}/{y}.pbf` — URL versioning 으로 immutable. 1년 cache,
override 불가. tile 손상 = 새 version 빌드 + manifest flip.

**RPO/RTO**: tile 자체는 *항상* 이전 version 의 client cache + R2 두 layer 보존.
tile 손상 = 클라이언트 자동 stale-while-revalidate fallback + 다음 manifest hot-swap
시 새 version fetch.

---

## 7. Compliance (Round 4 enterprise audit)

### 7.1 PIPC / 개인정보보호법

**판정**: SP9 base layer 데이터에 **개인정보 0** — V-World dtmk 의 *연속지적도* 는
필지 polygon + PNU + 지번 정보만 (소유자 정보 X). PIPC 적용 대상 아님.

**감사 trail**: 본 판정은 ADR 또는 별도 compliance log 에 박제 필요. 현재는 본 § 7
이 SSOT. 데이터 schema 가 변경되어 *PII 가 추가되는 경우* (e.g. 소유자 정보 통합)
즉시 PIPC 적용 + 데이터 마스킹 + 별도 ADR.

### 7.2 공공누리 제1유형 출처표시

V-World dtmk 데이터의 라이선스. *클라이언트* 의무:
- 매물 페이지 footer 에 출처 표기 (현재 미박제 — TODO frontend ADR)
- 본 라이선스 텍스트 = `crates/sp9-base-layer-config::DTMK_LICENSE` (Round 4 SSOT 박제)

**lineage**: `BuildLineage.source_license` + `BuildLineage.source_url` 에 박제됨
(Round 3 P1 + Round 4 SSOT). 클라이언트가 manifest 의 lineage 를 읽어 footer 동적
렌더 가능.

### 7.3 GitHub Actions log retention

기본 90일 — `actions/upload-artifact@v4` 의 `retention-days: 90` (SBOM artifacts).
audit trail (manifest 자체) 은 R2 에 영구 박제 — log 의존 0.

---

## 8. Disaster recovery (DR)

### 8.1 R2 region outage

**현재**: 단일 Cloudflare R2 region (CF default). SLA 99.9% (월 ~43 분 down 허용).

**RTO**: Cloudflare 복구 시간 의존 (외부). 통상 < 1 시간.

**Mitigation 옵션** (별도 ADR 미구현):
- 멀티 리전 R2 replication
- manifest CDN 의 secondary origin (e.g. AWS S3 cross-region)
- 클라이언트의 stale-while-revalidate + offline cache (현재 미적용)

### 8.2 V-World 사이트 outage

**영향**: phase 1 (Bronze refresh) 만 — phase 2 (Gold) 가 R2 의 *기존* zip 재사용 가능.

**fallback**:
- 사용자가 `workflow_dispatch { bronze_skip: true }` 로 Gold 만 재빌드
- R2 의 마지막 성공 batch 재사용 (~1개월 stale 까지 허용)
- V-World 가 1개월+ down 시 데이터 stale → 사용자 공지 + ADR 갱신

**RTO**: Bronze refresh 가 *복구된 후* 다음 cron (월 1회) 또는 manual dispatch.
SP9 SLO 침해 incident 로 분류.

### 8.3 GitHub Actions outage

**영향**: ETL cron / manual dispatch 모두 차단.

**fallback**: Gongzzang ETL 은 immutable tile artifact 재빌드까지만 허용한다.
Manifest pointer promote/rollback 은 반드시 platform-core Catalog 에서 수행한다.

**RTO**: GitHub 복구 (외부) 또는 manual local run (~4-8h).

### 8.4 사용자 측 (브라우저) outage

해당 없음 — base layer 는 정적 R2 객체. CDN edge 가 모든 사용자 fetch 처리.
