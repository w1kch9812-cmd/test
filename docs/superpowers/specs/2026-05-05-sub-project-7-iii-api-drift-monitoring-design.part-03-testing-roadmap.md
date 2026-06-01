## 9. 검증 / 테스트 전략

### 9.1 `crates/operations/api-health` 단위 테스트

- `HealthStatus::is_hard_fail()` enum behavior
- `HealthCheckRecord` serde / Display
- `NewHealthCheck` 빌더

### 9.2 `crates/db/api_health.rs` 통합 테스트

- `PgHealthCheckRepository::record()` happy path
- `is_n_cron_runs_failed(api_name, 3)` 다양한 시나리오:
  - 3 cron 모두 fail → true
  - 3 cron 중 1 success → false
  - 수동 trigger fail 만 있음 → false (cron 만 카운트)
  - 데이터 없음 → false

### 9.3 smoke test 자체

- 외부 API 직접 호출 = mock 무의미 → 단위 테스트 X
- 검증 방법: workflow_dispatch 수동 trigger (정상 path + simulate_failure path)

### 9.4 workflow yml + script

- 첫 push 후 workflow_dispatch 1회 (정상 path)
- workflow_dispatch + `simulate_failure=true` 1회 (Issue 자동 생성 검증)
- 다음 cron 정상 응답 시 자가 복구 검증

### 9.5 docs/observability/api-drift-smoke-test.md

운영 절차 SSOT — 위 검증 절차 명시 + 실패 분류 표 + escalation 정책.

---

## 10. Migration 진화 path

### 10.1 SP7-i (Sentry) 통합 시점

`api_health_check` 테이블은 그대로. Sentry 는 production code 의 panic / breaker open 등 **별도 이벤트 종류**:

```
SP7-iii: drift 정기 검출 → api_health_check 테이블
SP7-i:   에러 자동 추적   → Sentry SaaS
SP7-ii:  metrics          → Grafana Cloud / Prometheus
```

각 SSOT 가 분류별로 명확. 통합 시 변경 0.

### 10.2 SP-Admin (React Flow 시각화) 통합

미래 admin UI 가 `api_health_check` 쿼리 → React Flow 노드 그래프로 시각화:

```
[강남파이낸스 PNU 호출]
   ↓ ✅ (최근 cron success)
[BuildingRegisterClient]
   ↓ ✅ HTTP 200
[parse_building_title]
   ↓ ✅ Office 매핑
[검증 완료]
```

또는 fail 시 빨간 노드로 표시 + DB 의 error_detail 클릭 시 detail.

### 10.3 production scale 진화

1인 단계: STAGING_DATABASE_URL = production DB
production scale: 별도 staging DB + replication

이건 SP8 (IaC) 영역.

---

## 11. Follow-up 변경

### 11.1 본 sub-project 가 closing 하는 FU

- **FU 45**: 정부 API endpoint URL drift staging-only smoke test → ✅ closed
- **FU 46**: 정부 API JSON Number vs String schema drift 모니터링 → ✅ closed

### 11.2 본 sub-project 가 흡수 안 한 FU

- **FU 47**: V-World 지오코딩 (주소 → PNU) — SP6 frontend 또는 dev tool sub-project 로 분리

### 11.3 본 sub-project 에서 발견될 가능성

- 정부 API 의 추가 endpoint (실거래가 / 법제처) 도 같은 패턴 적용 — SP4-iii-b/c 도입 시 smoke test 자연 추가
- `api_health_check` 테이블에 추가 분류 (예: response_size / schema_version) 발견 시 별도 FU

---

## 12. 작업 단위 (T1-T6)

### T1: DB 마이그레이션 + 도메인 crate
- `30007_api_health_check.sql` 작성
- `crates/operations/api-health/` 신규 crate (entity / status / repository trait)
- 단위 테스트 (HealthStatus enum + record builder)
- 누적 테스트 ≥1259 → ≥1267

### T2: PgHealthCheckRepository
- `crates/db/api_health.rs` 신규
- `record()` / `is_n_cron_runs_failed()` / `find_latest()` 구현
- 통합 테스트 (Postgres 실 DB) — 4-5 시나리오
- workspace 통합 테스트 ≥110 → ≥114

### T3: data.go.kr smoke test
- `crates/data-clients/data-go-kr/Cargo.toml` 에 `real-api` feature
- `tests/smoke_real_api.rs` 신규 (feature-gated)
- 로컬 검증: `cargo test --features real-api -- --ignored` 1회 통과

### T4: V-World smoke test
- 동일 패턴으로 `crates/data-clients/vworld/`
- 로컬 검증

### T5: api-health-recorder Rust binary
- `crates/api-health-recorder/` 신규 crate (binary)
- `octocrab` (GitHub API client) 의존성 추가
- CLI 인자: `--api-name <X> --status <Y> --http-code <Z> --error-detail <log>`
- 동작: PgHealthCheckRepository 로 record + (fail 시) Issue 자동 생성/comment + (success 시) 기존 open Issue 자가 복구
- 단위 테스트: CLI parsing + 분기 로직

### T6: GitHub Actions workflow + secrets + 검증
- `.github/workflows/api-drift-smoke-test.yml`
- secrets 등록 (사용자 작업 — 4개: ODP_SERVICE_KEY / VWORLD_API_KEY / VWORLD_DOMAIN / STAGING_DATABASE_URL)
- workflow_dispatch 정상 path 검증
- workflow_dispatch + simulate_failure path 검증 (Issue 자동 생성 확인)
- 자가 복구 검증 (다음 정상 run 시 Issue 자동 close)

### T6: docs + Issue 자동 생성 검증
- `docs/observability/api-drift-smoke-test.md` 작성
- simulate_failure 1회 → Issue 자동 생성 확인
- 자가 복구 1회 검증
- `roadmap.md` 갱신 (SP7-iii ✅ closed, SP7-i/ii 자리 명시)

---

## 13. 추정

- **작업량**: 4-5일 (분해 됨, T1-T6)
- **신규 crate**: 2 (`crates/operations/api-health` 도메인 + `crates/api-health-recorder` binary)
- **신규 마이그레이션**: 1 (`30007_api_health_check.sql`)
- **신규 workflow**: 1 (`api-drift-smoke-test.yml`)
- **신규 docs**: 1 (`docs/observability/api-drift-smoke-test.md`)
- **누적 통계 변화**: 31 crate → 33 / 1259 tests → ~1285

---

## 14. SSS 자가 평가

| 기둥 | 보장 |
|---|---|
| 1 일관성 | ◎ (Repository pattern + 통합 테스트 패턴 그대로) |
| 2 자동 강제 | ◎ (cron + DB record + Issue 자동) |
| 3 추적성 | ◎ (api_health_check 영구 보존 + workflow run history) |
| 4 안전성 | ◎ (feature flag + production code path 검증) |
| 5 가시성 | ◎ (Issue alert + DB 쿼리 + 미래 admin UI) |
| 6 SSOT | ◎ (drift 결과 = 우리 DB; Issue 는 사람 알림 사본) |
| 7 명확성 | ◎ (docs/observability + simulate_failure 영구 인프라) |

= **근본 SSS 80%+ 달성**.
