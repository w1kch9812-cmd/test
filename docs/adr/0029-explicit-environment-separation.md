# ADR 0029 — `ETL_ENVIRONMENT` 명시 분리 + secret namespace 격리

| | |
|---|---|
| 작성일 | 2026-05-11 |
| 상태 | Accepted |
| 선행 | [ADR 0024](./0024-etl-cancel-protocol-immediate-abort.md), [ADR 0028](./0028-supply-chain-sha-pin-and-cleanup-cron.md) |

## 결정

ETL 의 모든 실행은 *명시적* `ETL_ENVIRONMENT={local|staging|production}` 선언 필수.
미설정 시 즉시 fail-fast. 각 env 별 secret namespace 격리:

```rust
// crates/sp9-base-layer-config/src/types.rs
pub enum Environment {
    Local,
    Staging,
    Production,
}
```

```
local:      R2_LOCAL_ACCOUNT_ID      / R2_LOCAL_ACCESS_KEY      / ...
staging:    R2_STAGING_ACCOUNT_ID    / R2_STAGING_ACCESS_KEY    / ...
production: R2_PRODUCTION_ACCOUNT_ID / R2_PRODUCTION_ACCESS_KEY / ...
```

backward-compat: `R2_ACCOUNT_ID` (namespace 없음) 도 *일시* 허용 — single `legacy` warning
log 출력 + ADR 0030 에서 완전 제거 (다음 sprint).

## 컨텍스트 — Round 5 verify smoke 의 사고

Round 5 verify smoke 실행 중:
- `cargo run -p etl-base-layer -- gold --layer parcels ... ./var/sample/gangnam.geojson`
- 의도: *local-only* 빌드 (사용자가 R2 자격 미설정 가정)
- 실제: 사용자 `.env` 에 `R2_ACCOUNT_ID` / `R2_ACCESS_KEY` / `R2_SECRET_KEY` / `R2_BUCKET=gongzzang` 박제 됨 → `Config::from_env()` 가 자동 활성 → 실 R2 bucket 의 `gold/v1/parcels/` 235 tile 덮어쓰기 (강남 sample 부분 데이터)

**원인** (architectural):
1. `Config::from_env()` 가 R2 자격 4개 *존재* 만 검사 — **의도 추론 trick**
2. local / dev / staging / production 의 secret 이 동일 namespace 공유 — **environment 분리 0**
3. `.env` 가 어떤 env 의 자격인지 박제 0 — operator 가 *모르고* prod credential local 에 박제 가능

**SSS 위반**: AGENTS.md § 1 *자동 강제* + AGENTS.md § 10.1.5 *Security & Privacy* +
AGENTS.md § 6 *사용자 확인 필요한 작업* 모두 침해.

## 검토한 옵션

### A — `--dry-run` CLI flag
- 장점: 호출자가 명시
- 거부: 잊기 쉬움 (default 가 production-modify). *명시 안 한 작업* 이 위험 = SSS 반대

### B — `ETL_ENVIRONMENT` env 강제 + namespace 분리 (본 결정)
- 장점:
  - 모든 실행이 명시 env 선언 필요 — 의도 추론 0
  - `ETL_ENVIRONMENT=local` + `R2_PRODUCTION_*` 만 set → R2 자동 비활성 (secret 격리)
  - operator 가 `.env` 박제 시 어떤 env 의 자격인지 *namespace 자체* 가 표시
- 단점:
  - workflow yml / `.env.example` / runbook 모두 갱신 필요
  - backward-compat 한 sprint 보존 후 제거

### C — AWS Secrets Manager / Doppler 같은 외부 vault
- 장점: 가장 강한 격리
- 거부: scope creep — 본 incident 해결에 외부 dependency 추가 부담 과대.
  *향후 ADR 0030 후속* 검토 가능. 본 ADR 은 *Rust 코드 측 fail-fast* 만.

## 채택 (B)

### Rust 측 — `Config::from_env`

```rust
pub fn from_env() -> Result<Self, ConfigError> {
    // ETL_ENVIRONMENT 필수 — fail-fast.
    let env = Environment::from_env_required()?;

    // env 별 prefix 매핑.
    let r2_prefix = env.r2_secret_prefix();  // "R2_LOCAL_" | "R2_STAGING_" | "R2_PRODUCTION_"
    let r2 = build_r2_config(r2_prefix);

    // backward-compat (1 sprint): R2_* (no namespace) → warning + 활성
    let r2 = r2.or_else(|| build_r2_config_legacy());

    Self { env, r2, ... }
}
```

### Workflow 측

```yaml
# .github/workflows/sp9-base-layer-etl.yml
env:
  ETL_ENVIRONMENT: production
  R2_PRODUCTION_ACCOUNT_ID: ${{ secrets.R2_ACCOUNT_ID }}  # GitHub secrets 는 그대로
  R2_PRODUCTION_ACCESS_KEY: ${{ secrets.R2_ACCESS_KEY }}
  R2_PRODUCTION_SECRET_KEY: ${{ secrets.R2_SECRET_KEY }}
  R2_PRODUCTION_BUCKET:     ${{ secrets.R2_BUCKET }}
```

### Local smoke 측

사용자가 명시 `ETL_ENVIRONMENT=local` 설정 시 `R2_LOCAL_*` 만 활성. 즉:

```bash
# .env.local (gitignored)
ETL_ENVIRONMENT=local
# R2_LOCAL_* 비워두면 → R2 비활성 (local-only mode)
# 실 staging R2 로 smoke 하려면 R2_LOCAL_* 만 set.

# .env.production 이 실수로 활성되어도:
ETL_ENVIRONMENT=local        # 명시 local
R2_PRODUCTION_ACCOUNT_ID=... # production 자격 박제됨
# → ETL 이 R2_LOCAL_* 만 읽어서 R2 비활성. 사고 0.
```

## SSS 7기둥 매핑

| 기둥 | 이전 (R2_* shared) | 본 결정 (namespace 격리) |
|---|---|---|
| 일관성 | ❌ — 같은 var 가 4개 env 공유 | ✅ — env 별 별도 var |
| 자동강제 | ❌ — 사고 가능 (Round 5 smoke) | ✅ — env 미선언 = fail-fast |
| 추적성 | △ — log 가 어떤 env 인지 모름 | ✅ — `ETL_ENVIRONMENT` 명시 + Sentry tag |
| 안전성 | ❌ — local 이 prod 모르게 modify | ✅ — namespace 격리로 차단 |
| 가시성 | △ | ✅ — `.env.example` 의 namespace 자체가 의도 표시 |
| SSOT | △ — secret 출처 추적 어려움 | ✅ — env 별 단일 secret store |
| 명확성 | ❌ — operator 추측 의존 | ✅ — env 변수명 자체로 의도 명시 |

## 영향

### 신규
- `crates/sp9-base-layer-config/src/types.rs::Environment` enum
- `crates/sp9-base-layer-config/src/lib.rs::EnvironmentParseError`
- `services/etl-base-layer/src/config.rs` — env-prefixed secret loading + backward-compat warning
- `.env.example` (root) — 새 namespace 패턴 박제
- `docs/adr/0029-explicit-environment-separation.md` (본 파일)
- `docs/sp9/sslo-runbook.md` § 5 갱신 (secret namespace 정책)

### 수정
- `.github/workflows/sp9-base-layer-etl.yml` — `ETL_ENVIRONMENT=production` +
  R2_PRODUCTION_* / V-World 자격은 그대로
- `.github/workflows/sp9-manifest-backup-cleanup.yml` — 동일
- `.github/workflows/sp9-base-layer-rollback.yml` — 동일
- ADR 0024 / 0025 / 0027 / 0028 의 env 언급 갱신 (linkbacks)

### 후속 (ADR 0030 박제 예정)
- `R2_*` (no namespace) backward-compat 완전 제거
- 외부 vault (AWS Secrets Manager / Doppler) 통합 검토

## 재검토 트리거

- Round 5 smoke 같은 사고 *재발* 시 — 본 ADR 의 backward-compat 일찍 제거
- 새 환경 (e.g. canary / qa) 추가 — `Environment` enum 확장
- 외부 vault 도입 결정 — ADR 0030 신설 + 본 ADR 의 fallback path 폐기

## 참고

- AGENTS.md § 1 (자동 강제) + § 6 (사용자 확인) + § 10.1.5 (Security & Privacy)
- 12-Factor App III. Config: https://12factor.net/config
- OWASP ASVS L3 V2.1 (secret management)
