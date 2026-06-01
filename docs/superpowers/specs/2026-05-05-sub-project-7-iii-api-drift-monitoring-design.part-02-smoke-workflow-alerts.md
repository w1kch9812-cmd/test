## 6. Smoke Test 통합 테스트

### 6.1 Feature flag 패턴

```toml
# crates/data-clients/data-go-kr/Cargo.toml
[features]
real-api = []
```

### 6.2 data.go.kr smoke test

```rust
// crates/data-clients/data-go-kr/tests/smoke_real_api.rs
#![cfg(feature = "real-api")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[tokio::test]
#[ignore]  // 평소 cargo test 에서 skip — feature + ignored 둘 다 필수
async fn smoke_data_go_kr_building_register_alive() {
    // 환경변수 ODP_SERVICE_KEY 필수 (CI secret)
    let client = DataGoKrClient::from_env().expect("ODP_SERVICE_KEY required");

    // 강남파이낸스센터 PNU (검증된 fixture, 항상 존재)
    let pnu_str = std::env::var("GONGZZANG_DRIFT_TEST_PNU")
        .unwrap_or_else(|_| "1168010100107370000".to_owned());
    let pnu = Pnu::try_new(&pnu_str).expect("valid PNU");

    let buildings = client.building_register()
        .fetch_title_info(split(&pnu)).await
        .expect("정부 API 호출 + parse 통과")
        .as_object().cloned().expect("response object");

    let parsed = parse_building_title(&Value::Object(buildings), &pnu, &dummy_polygon(), Utc::now())
        .expect("parser 통과");

    assert!(!parsed.is_empty(), "응답 0건 — endpoint drift 의심");
    let b = &parsed[0];
    assert_eq!(b.main_purpose_code, BuildingPurposeCode::Office,
        "강남파이낸스센터 mainPurpsCd → Office 매핑 검증");
    assert_eq!(b.structure_code, BuildingStructureCode::SteelReinforcedConcrete);
}
```

### 6.3 V-World smoke test

```rust
// crates/data-clients/vworld/tests/smoke_real_api.rs
#![cfg(feature = "real-api")]

#[tokio::test]
#[ignore]
async fn smoke_vworld_parcel_alive() {
    let client = VWorldClient::from_env().expect("VWORLD_API_KEY + VWORLD_DOMAIN required");

    // 강남파이낸스센터 PNU
    let pnu = Pnu::try_new("1168010100107370000").unwrap();

    let parcel = client.parcel_reader().fetch_by_pnu(&pnu).await
        .expect("V-World 호출 + parse 통과")
        .expect("필지 존재");

    assert_eq!(parcel.pnu.as_str(), "1168010100107370000");
    // jiyok_cd 같은 핵심 필드 존재 검증
}
```

### 6.4 simulate_failure 환경변수

`GONGZZANG_DRIFT_TEST_PNU=9999999999999999999` 로 호출 시 응답 0건 → assert fail → 의도된 fail.

---

## 7. GitHub Actions Workflow

### 7.1 `.github/workflows/api-drift-smoke-test.yml`

```yaml
name: api-drift-smoke-test

on:
  schedule:
    - cron: '0 19 * * *'  # 04:00 KST (UTC+9)
  workflow_dispatch:
    inputs:
      simulate_failure:
        description: 'Force fail (drift detection 검증)'
        type: boolean
        default: false

jobs:
  smoke-data-go-kr:
    runs-on: ubuntu-24.04
    timeout-minutes: 10
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: smoke test
        id: smoke
        env:
          ODP_SERVICE_KEY: ${{ secrets.ODP_SERVICE_KEY }}
          GONGZZANG_DRIFT_TEST_PNU: >-
            ${{ inputs.simulate_failure
              && '9999999999999999999'
              || '1168010100107370000' }}
        run: |
          cargo test --features real-api -p data-go-kr-client \
            --test smoke_real_api -- --ignored --nocapture
        continue-on-error: true
      - name: record to DB + alert
        env:
          DATABASE_URL: ${{ secrets.STAGING_DATABASE_URL }}
          API_NAME: data_go_kr.getBrTitleInfo
          STATUS: ${{ steps.smoke.outcome }}
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          CRON_RUN: ${{ github.event_name == 'schedule' }}
        run: ./scripts/api-drift-record-and-alert.sh

  smoke-vworld:
    runs-on: ubuntu-24.04
    # smoke-data-go-kr 와 동일 구조
    ...
```

### 7.2 `scripts/api-drift-record-and-alert.sh`

이 스크립트가:
1. `psql` 또는 별도 Rust binary 로 `api_health_check` 에 INSERT
2. fail 인 경우:
   - `is_n_cron_runs_failed(api_name, 3)` 쿼리 → true 면 escalation
   - `status="parse_fail"` 또는 `http_4xx` 면 즉시 escalation
   - 기존 open Issue 가 있으면 comment 추가, 없으면 신규 Issue
3. success + 기존 open Issue 있으면 close (자가 복구)

**구현: B 채택 (Rust binary `crates/api-health-recorder/`)**

**이유:**
- 기존 `PgHealthCheckRepository` 재사용 (SSS 일관성)
- type-safe (bash 보다 robust)
- `octocrab` (Rust GitHub API client) 로 Issue 자동 생성 / close
- workflow yml 은 단순 orchestration 만 (`cargo run --bin api-health-recorder -- --api-name X --status Y`)

**대안 (bash + psql + gh CLI) 거부 이유:** bash logic 이 복잡해질 때 type 검증 X. 1500줄 안티패턴 회피 (스크립트가 길어지면 차라리 Rust).

### 7.3 secrets 목록

| Secret | 용도 |
|---|---|
| `ODP_SERVICE_KEY` | data.go.kr 호출 |
| `VWORLD_API_KEY` | V-World 호출 |
| `VWORLD_DOMAIN` | V-World 도메인 등록 (`localhost`) |
| `STAGING_DATABASE_URL` | DB write (별도 staging DB 권장 — production DB 분리) |

**STAGING_DATABASE_URL 결정:**
- 옵션 A: production DB 와 동일 (간단)
- 옵션 B: 별도 staging DB (격리, 비용 ↑)
- → **옵션 A 채택** (1인 단계, drift 결과는 production 에 영향 0). 미래 production scale 시 분리.

---

## 8. 알림 정책 (GitHub Issue)

### 8.1 Issue 자동 생성 조건

| 분류 | trigger 조건 | label |
|---|---|---|
| **drift:5xx-server** | 3 cron runs 연속 fail | `drift`, `drift:5xx-server`, `drift:3-day-streak` |
| **drift:4xx-auth** | 1회 fail (즉시) | `drift`, `drift:4xx-auth` |
| **drift:schema** | 1회 fail (즉시) | `drift`, `drift:schema` |
| **drift:timeout** | 3 cron runs 연속 fail | `drift`, `drift:timeout`, `drift:3-day-streak` |

### 8.2 Issue 본문 포맷

```markdown
## 발견 시각
2026-05-XX 04:00 KST (cron run)

## 분류
schema-mismatch (parse_building_title fail)

## API
data.go.kr.getBrTitleInfo

## 실패 log (secrets masked)
<test stderr>

## 최근 cron run 결과 (DB 쿼리 결과)
| checked_at         | status      | http_code |
| ------------------ | ----------- | --------- |
| 2026-05-05 04:00   | parse_fail  | 200       |
| 2026-05-04 04:00   | parse_fail  | 200       |
| 2026-05-03 04:00   | parse_fail  | 200       |

## 수동 검증
GitHub Actions → "api-drift-smoke-test" → "Run workflow" 버튼

## 자가 복구
다음 cron 정상 응답 시 이 Issue 자동 close (label `drift:auto-recovered` 추가).
```

### 8.3 Assignees / Notification

- Assignees: `w1kch9812` (repo owner)
- Notification: GitHub default (이메일/푸시)
- Slack/Sentry/Discord 통합: SP7-i 이후

### 8.4 자가 복구

workflow success + `find_latest_open_issue(api_name, label="drift")` 가 있으면:
1. comment 추가: "✅ 자가 복구 — 정부 일시 장애였음"
2. label 추가: `drift:auto-recovered`
3. issue close

---
