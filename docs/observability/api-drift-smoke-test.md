# API Drift Smoke Test (SP7-iii)

> **목적**: 정부 API (data.go.kr / V-World) 의 endpoint URL + JSON schema drift 자동 검출
> **SSOT**: Postgres `api_health_check` 테이블 + GitHub Issue (사람 알림 사본)

## 시스템 개요

```text
[04:00 KST cron]
   ↓
[GitHub Actions: api-drift-smoke-test.yml]
   ├── job: smoke-data-go-kr
   └── job: smoke-vworld
        ↓ (각 job 안)
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
   - 정부 API 점검 페이지 확인 (<https://www.vworld.kr/dev>, <https://www.data.go.kr>)
   - 자가 복구 대기 (다음 cron success 시 Issue 자동 close)

### 수동 trigger (drift 의심 시 즉시 검증)

GitHub → Actions → api-drift-smoke-test → "Run workflow"

체크박스:

- `simulate_failure: false` (default) — 정상 path 검증
- `simulate_failure: true` — 일부러 fail (Issue 자동 생성 검증)

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

1. 기존 open drift Issue (api_name 매치) 에 comment "자가 복구"
2. Issue close (label `drift` 유지)

## 진화 path

- **SP7-i (Sentry)**: production code panic / breaker open 등 — 본 시스템과 별개 dispatch
- **SP7-ii (Grafana)**: `api_health_check` 테이블에서 metrics 추출
- **SP-Admin React Flow**: admin UI 에서 시계열 시각화

## DB Schema 참조

- 마이그레이션: `migrations/30007_api_health_check.sql`
- 도메인: `crates/operations/api-health/`
- 인프라: `crates/db/src/api_health.rs`
- recorder binary: `crates/api-health-recorder/`
- Workflow: `.github/workflows/api-drift-smoke-test.yml`

## Spec / Plan

- Spec: `docs/superpowers/specs/2026-05-05-sub-project-7-iii-api-drift-monitoring-design.md`
- Plan: `docs/superpowers/plans/2026-05-05-sub-project-7-iii-api-drift-monitoring.md`
