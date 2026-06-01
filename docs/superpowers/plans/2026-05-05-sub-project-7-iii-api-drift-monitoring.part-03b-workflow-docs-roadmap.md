# SP7-iii API Drift Monitoring - Part 03B: Workflow, Docs, and Roadmap

Parent index: [SP7-iii API Drift Monitoring - Part 03](./2026-05-05-sub-project-7-iii-api-drift-monitoring.part-03.md).
## Phase F: GitHub Actions Workflow + 검증

### Task 6: T6 — `.github/workflows/api-drift-smoke-test.yml` + secrets + 검증 + roadmap

**Files:**
- Create: `.github/workflows/api-drift-smoke-test.yml`
- Create: `docs/observability/api-drift-smoke-test.md`
- Modify: `docs/superpowers/roadmap.md`

#### Step 6.1: workflow yml 작성

- [ ] **Step**: `.github/workflows/api-drift-smoke-test.yml` 작성

```yaml
name: api-drift-smoke-test

on:
  schedule:
    # 04:00 KST = 19:00 UTC (전날)
    - cron: '0 19 * * *'
  workflow_dispatch:
    inputs:
      simulate_failure:
        description: 'Force fail (drift detection 검증)'
        type: boolean
        default: false

jobs:
  smoke-data-go-kr:
    runs-on: ubuntu-24.04
    timeout-minutes: 15
    permissions:
      issues: write
      contents: read
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2

      - name: Determine PNU (simulate_failure → invalid PNU)
        id: pnu
        run: |
          if [ "${{ inputs.simulate_failure }}" = "true" ]; then
            echo "value=9999999999999999999" >> $GITHUB_OUTPUT
          else
            echo "value=1168010100107370000" >> $GITHUB_OUTPUT
          fi

      - name: Run smoke test
        id: smoke
        env:
          ODP_SERVICE_KEY: ${{ secrets.ODP_SERVICE_KEY }}
          GONGZZANG_DRIFT_TEST_PNU: ${{ steps.pnu.outputs.value }}
        run: |
          set +e
          START_MS=$(date +%s%3N)
          cargo test --features real-api -p data-go-kr-client \
              --test smoke_real_api -- --ignored --nocapture 2>&1 | tee smoke.log
          STATUS_CODE=$?
          END_MS=$(date +%s%3N)
          DURATION=$((END_MS - START_MS))
          echo "duration_ms=$DURATION" >> $GITHUB_OUTPUT

          if [ $STATUS_CODE -eq 0 ]; then
              echo "status=success" >> $GITHUB_OUTPUT
              echo "http_code=200" >> $GITHUB_OUTPUT
          else
              if grep -q "endpoint URL drift" smoke.log; then
                  echo "status=parse_fail" >> $GITHUB_OUTPUT
                  echo "http_code=200" >> $GITHUB_OUTPUT
              elif grep -q "5xx" smoke.log; then
                  echo "status=http_5xx" >> $GITHUB_OUTPUT
                  echo "http_code=502" >> $GITHUB_OUTPUT
              else
                  echo "status=connection_fail" >> $GITHUB_OUTPUT
              fi
          fi

      - name: Sanitize error log (mask secrets)
        id: sanitize
        if: always()
        run: |
          # ServiceKey 부분 마스킹
          sed -i 's/ServiceKey=[^&"]*/ServiceKey=***/g' smoke.log || true
          # 마지막 80줄만 (Issue body 압축)
          tail -n 80 smoke.log > smoke.log.short
          ESCAPED=$(jq -Rs . < smoke.log.short)
          echo "log_json=$ESCAPED" >> $GITHUB_OUTPUT

      - name: Record + Issue orchestration
        env:
          DATABASE_URL: ${{ secrets.STAGING_DATABASE_URL }}
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          GITHUB_REPOSITORY: ${{ github.repository }}
        run: |
          IS_CRON="${{ github.event_name == 'schedule' }}"
          cargo run --bin api-health-recorder -- \
              --api-name "data_go_kr.getBrTitleInfo" \
              --status "${{ steps.smoke.outputs.status }}" \
              --duration-ms "${{ steps.smoke.outputs.duration_ms }}" \
              --cron-run "$IS_CRON" \
              ${HTTP_CODE:+--http-code $HTTP_CODE} \
              --error-detail "$(cat smoke.log.short)"
        env:
          HTTP_CODE: ${{ steps.smoke.outputs.http_code }}

  smoke-vworld:
    runs-on: ubuntu-24.04
    timeout-minutes: 15
    permissions:
      issues: write
      contents: read
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2

      - name: Determine PNU
        id: pnu
        run: |
          if [ "${{ inputs.simulate_failure }}" = "true" ]; then
            echo "value=9999999999999999999" >> $GITHUB_OUTPUT
          else
            echo "value=1168010100107370000" >> $GITHUB_OUTPUT
          fi

      - name: Run smoke test
        id: smoke
        env:
          VWORLD_API_KEY: ${{ secrets.VWORLD_API_KEY }}
          VWORLD_DOMAIN: ${{ secrets.VWORLD_DOMAIN }}
          GONGZZANG_DRIFT_TEST_PNU: ${{ steps.pnu.outputs.value }}
        run: |
          set +e
          START_MS=$(date +%s%3N)
          cargo test --features real-api -p vworld-client \
              --test smoke_real_api -- --ignored --nocapture 2>&1 | tee smoke.log
          STATUS_CODE=$?
          END_MS=$(date +%s%3N)
          echo "duration_ms=$((END_MS - START_MS))" >> $GITHUB_OUTPUT

          if [ $STATUS_CODE -eq 0 ]; then
              echo "status=success" >> $GITHUB_OUTPUT
              echo "http_code=200" >> $GITHUB_OUTPUT
          else
              echo "status=connection_fail" >> $GITHUB_OUTPUT
          fi

      - name: Sanitize log
        if: always()
        run: |
          sed -i 's/key=[^&"]*/key=***/g' smoke.log || true
          tail -n 80 smoke.log > smoke.log.short

      - name: Record + Issue orchestration
        env:
          DATABASE_URL: ${{ secrets.STAGING_DATABASE_URL }}
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          GITHUB_REPOSITORY: ${{ github.repository }}
          HTTP_CODE: ${{ steps.smoke.outputs.http_code }}
        run: |
          IS_CRON="${{ github.event_name == 'schedule' }}"
          cargo run --bin api-health-recorder -- \
              --api-name "vworld.getFeature" \
              --status "${{ steps.smoke.outputs.status }}" \
              --duration-ms "${{ steps.smoke.outputs.duration_ms }}" \
              --cron-run "$IS_CRON" \
              ${HTTP_CODE:+--http-code $HTTP_CODE} \
              --error-detail "$(cat smoke.log.short)"
```

#### Step 6.2: secrets 등록 (사용자 작업)

- [ ] **Step (사용자)**: GitHub Settings → Secrets and variables → Actions

```
ODP_SERVICE_KEY        = (data.go.kr 키)
VWORLD_API_KEY         = (V-World 키)
VWORLD_DOMAIN          = localhost
STAGING_DATABASE_URL   = (production DB url, 1인 단계는 동일 DB OK)
```

#### Step 6.3: docs/observability/api-drift-smoke-test.md 작성

- [ ] **Step**: `docs/observability/api-drift-smoke-test.md` 작성

````markdown
# API Drift Smoke Test (SP7-iii)

> **목적**: 정부 API (data.go.kr / V-World) 의 endpoint URL + JSON schema drift 자동 검출
> **SSOT**: Postgres `api_health_check` 테이블 + GitHub Issue (사람 알림 사본)

## 시스템 개요

```
[04:00 KST cron]
   ↓
[GitHub Actions: api-drift-smoke-test.yml]
   ├── job: smoke-data-go-kr
   └── job: smoke-vworld
        ↓ (각 job 안에서)
   [cargo test --features real-api -- --ignored]
        ↓
   [api-health-recorder Rust binary]
        ├── PgHealthCheckRepository::record() → DB INSERT
        └── Issue orchestration (escalation / 자가 복구)
```

## 분류 (HealthStatus)

| Status | HTTP | 분류 | Escalation |
|---|---|---|---|
| `success` | 200 | OK | (자가 복구 trigger) |
| `http_5xx` | 5xx | soft-fail | 3일 연속 cron fail 시 Issue |
| `http_4xx` | 4xx | hard-fail | 즉시 Issue (키/quota 문제) |
| `parse_fail` | 200 | hard-fail | 즉시 Issue (schema drift) |
| `timeout` | - | soft-fail | 3일 연속 cron fail 시 Issue |
| `connection_fail` | - | soft-fail | 3일 연속 cron fail 시 Issue |

## 운영 절차

### 매일 정상 응답 확인

GitHub → Actions → api-drift-smoke-test → 최근 cron run 결과.

### Issue 생성 시 처리

1. Issue body 의 "분류" 확인
2. `parse_fail` (schema drift):
   - 정부 API 응답 schema 확인 (curl 또는 staging 검증)
   - parser 코드 (`parse_building_title` 등) 갱신 PR
3. `http_4xx`:
   - 키 만료 또는 quota 초과 확인
   - secrets 갱신 또는 quota 증액 신청
4. `http_5xx` / `timeout` / `connection_fail`:
   - 정부 API 점검 페이지 확인 (https://www.vworld.kr/dev, https://www.data.go.kr)
   - 자가 복구 대기 (다음 cron success 시 Issue 자동 close)

### 수동 trigger (drift 의심 시 즉시 검증)

GitHub → Actions → api-drift-smoke-test → "Run workflow"

체크박스:
- `simulate_failure: false` (default) — 정상 path 검증
- `simulate_failure: true` — 일부러 fail (Issue 자동 생성 검증)

### simulate_failure 사용 케이스

- 새 endpoint 추가 후 alert 시스템 작동 검증
- Issue 자동 생성 / close 로직 변경 후 검증
- Issue label / body 포맷 검증

## 알림 정책

### Issue 자동 생성 조건

| Trigger | Label |
|---|---|
| `parse_fail` 1회 | `drift`, `drift:schema` |
| `http_4xx` 1회 | `drift`, `drift:4xx-auth` |
| `http_5xx` 3 cron 연속 | `drift`, `drift:5xx-server` |
| `timeout` 3 cron 연속 | `drift`, `drift:timeout` |
| `connection_fail` 3 cron 연속 | `drift`, `drift:connection` |

### 자가 복구

다음 cron 의 `success` 응답 시:
1. 기존 open drift Issue (api_name 매치) 에 comment "✅ 자가 복구"
2. Issue close (label `drift` 유지 — 검색 가능)

### 수동 close

문제 해결 후 수동으로 Issue close — 차후 cron success 시 본 시스템이 영향 X.

## 진화 path

- **SP7-i (Sentry)**: production code panic / breaker open 등 — 본 시스템과 별개 dispatch
- **SP7-ii (Grafana)**: `api_health_check` 테이블에서 metrics 추출 (성공률 / 분류 분포 / latency)
- **SP-Admin React Flow**: admin UI 에서 `api_health_check` 시계열 시각화

## DB Schema 참조

- 마이그레이션: `migrations/30007_api_health_check.sql`
- 도메인: `crates/operations/api-health/`
- 인프라: `crates/db/src/api_health.rs`
- recorder binary: `crates/api-health-recorder/`

## Spec / Plan

- Spec: `docs/superpowers/specs/2026-05-05-sub-project-7-iii-api-drift-monitoring-design.md`
- Plan: `docs/superpowers/plans/2026-05-05-sub-project-7-iii-api-drift-monitoring.md`
````

#### Step 6.4: roadmap.md 갱신

- [ ] **Step**: `docs/superpowers/roadmap.md` 의 header / 완료 표 / follow-up 갱신

다음 변경 적용:

**Header:**
```markdown
> **갱신일**: 2026-05-05 (SP7-iii 종료 직후)
> **현재 main**: `<T6 commit hash>` (SP7-iii — drift 자동 검출 시스템)
```

**완료 표 (SP7-iii 추가):**
```markdown
| **7-iii** | 정부 API drift 자동 검출 시스템 | crates/operations/api-health (도메인) + crates/db/src/api_health.rs (PgImpl) + 2 smoke test crate (data.go.kr + V-World, feature-gated) + crates/api-health-recorder (octocrab binary) + .github/workflows/api-drift-smoke-test.yml (nightly cron + workflow_dispatch) + docs/observability/api-drift-smoke-test.md (운영 SSOT). FU 45/46 closed. SSS 7기둥 모두 ◎ | ✅ |
```

**누적 통계 갱신:**
```markdown
**누적**: 33 crate, ~1285 tests, 4 CI workflow 그린 (CI / db-migrations / walking-skeleton / api-drift-smoke-test).
```

**Follow-up 갱신:**
- ~~FU 45 (제안): endpoint URL drift staging-only smoke test~~ → ✅ closed by SP7-iii (`<T6 commit>`)
- ~~FU 46 (제안): JSON Number vs String schema drift 모니터링~~ → ✅ closed by SP7-iii (`<T6 commit>`)
- FU 47: V-World 지오코딩 — 미해소, SP6 frontend 또는 dev tool sub-project

**SP7-i / SP7-ii 자리:**
```markdown
### SP7 시리즈 (관측성)
- ✅ SP7-iii: drift 자동 검출 (2026-05-05, `<commit>`)
- 미착수 SP7-i: Sentry — 에러 자동 추적 (services/api 통합, 1-2일)
- 미착수 SP7-ii: Grafana metrics + Outbox publisher metrics (2-3일)
```

#### Step 6.5: 워크플로우 검증 (사용자 체크포인트)

- [ ] **Step (사용자)**: secrets 등록 후 push → GitHub Actions 페이지에서 수동 trigger
  - workflow_dispatch (정상 path) — 모든 job pass + DB record 확인
  - workflow_dispatch (`simulate_failure=true`) — fail + Issue 자동 생성 확인 (label: drift, drift:schema)

- [ ] **Step (사용자)**: simulate_failure 후 수동 trigger 정상 응답 → 기존 Issue 자동 close + comment "✅ 자가 복구" 확인

#### Step 6.6: cargo check / clippy / test workspace 그린

- [ ] **Step**: workspace 검증

```bash
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --lib
cargo fmt --all -- --check
```

Expected: 모두 pass.

#### Step 6.7: T6 commit + push

- [ ] **Step**: commit

```bash
git add .github/workflows/api-drift-smoke-test.yml \
        docs/observability/api-drift-smoke-test.md \
        docs/superpowers/roadmap.md

git commit -m "$(cat <<'EOF'
feat(sp7-iii-t6): GitHub Actions cron workflow + docs + roadmap

T6 of SP7-iii (마지막):
- .github/workflows/api-drift-smoke-test.yml — nightly cron 04:00 KST + workflow_dispatch
  - 2 jobs (smoke-data-go-kr / smoke-vworld) 병렬
  - simulate_failure input 으로 fail 의도 검증 가능
  - secrets 마스킹 + tail 80 lines log 압축
  - api-health-recorder binary 호출 → DB record + Issue orchestration
- docs/observability/api-drift-smoke-test.md — 운영 SSOT
  - 분류 표 (HealthStatus)
  - 운영 절차 (Issue 처리 / 수동 trigger / simulate_failure)
  - 알림 정책 (label / 자가 복구)
  - 진화 path (SP7-i/ii / SP-Admin)
- docs/superpowers/roadmap.md — SP7-iii ✅ closed
  - FU 45/46 closed 표기
  - 누적 33 crate / ~1285 tests / 4 CI workflow

SP7 시리즈 첫 sub-project 완료. SP7-i (Sentry) / SP7-ii (Grafana) 자리 명시.
EOF
)"

git push origin main
```

**사용자 체크포인트**: T6 commit 확인 + 4 CI workflow 그린 확인 + 다음 sub-project 결정.

---

## 위험 요소

- **V-World 일시 장애**: brainstorming 시점에 V-World 502 발견. 첫 cron 결과가 fail 일 수 있음 — 일시 장애로 분류 (`http_5xx`), 3일 연속 fail 시에만 escalation 이라 시스템은 정상 작동
- **octocrab 0.46 API 변경**: docs.rs 시점 따라 `issues().list()` / `update()` API signature 다를 수 있음 — plan 작성 시점에 확인하고 수정
- **데이터.go.kr 도메인 등록**: ODP_SERVICE_KEY 가 발급 시 도메인 등록한 경우 GitHub Actions IP 가 다른 도메인 일 수 있음 — 일반 키 (도메인 검증 X) 면 OK
- **STAGING_DATABASE_URL 결정**: 1인 단계 = production DB 와 동일. 미래 production scale 시 분리 (SP8 IaC)

## 추정

- T1: 1 commit, 1-2시간
- T2: 1 commit, 2-3시간
- T3: 1 commit, 1-2시간
- T4: 1 commit, 1-2시간
- T5: 1 commit, 4-6시간 (octocrab 학습 + Issue orchestration)
- T6: 1 commit, 2-3시간 (workflow yml + docs)

총: 4-5일 (각 task 끝 사용자 체크포인트 포함)

## 완료 후 다음

- SP7-i (Sentry) brainstorming
- 또는 SP4-iii-b (data.go.kr 실거래가) — drift smoke test 자연 추가 가능
- 또는 SP6 (Frontend) brainstorming
- 또는 SP-FU-OCC (FU 14/15/16 OCC API)

---

## 자가 평가 — Spec coverage

Spec 의 모든 § 가 plan task 로 커버됐는지 확인:

- § 1 배경 — context only, plan task X
- § 2 목표 — T1-T6 전체
- § 3 SSS 7기둥 — T1-T6 누적
- § 4 아키텍처 (4.1 그림 / 4.2 컴포넌트 / 4.3 책임) → T1-T6 파일 구조 그대로
- § 5 데이터 모델 (5.1 SQL / 5.2 entity / 5.3 trait) → T1
- § 6 Smoke test 통합 테스트 (6.1 feature flag / 6.2 data.go.kr / 6.3 V-World / 6.4 simulate) → T3 + T4
- § 7 GitHub Actions workflow → T6
- § 8 알림 정책 → T5 (Issue orchestration) + T6 (workflow)
- § 9 검증 / 테스트 전략 → T1-T6 의 단위 + 통합 테스트
- § 10 Migration 진화 path → T6 docs/observability/
- § 11 Follow-up → T6 roadmap.md
- § 12 작업 단위 → T1-T6 그대로
- § 13 추정 → 본 plan 의 추정
- § 14 SSS 자가 평가 → T1-T6 누적

**모든 § 가 task 로 covered.** ✅
