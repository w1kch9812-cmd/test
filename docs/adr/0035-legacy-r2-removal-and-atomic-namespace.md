# ADR 0030 — Legacy `R2_*` 완전 제거 + atomic namespace 강제

| | |
|---|---|
| 작성일 | 2026-05-11 |
| 상태 | Accepted |
| Supersedes | [ADR 0029](./0029-explicit-environment-separation.md) § "Backward-compat (1 sprint)" 의 backward-compat path 만 |
| 선행 | [ADR 0029](./0029-explicit-environment-separation.md) (Environment 명시 분리) |

## 결정

ADR 0029 의 **1-sprint backward-compat** (`R2_*` namespace 없음 + `ETL_BUILD_ENV`
alias) 을 **즉시 완전 제거**. R2 자격은 *항상* `R2_<ENV>_*` namespace + `ETL_ENVIRONMENT`
SSOT 통과:

```rust
// crates/sp9-base-layer-config/src/types.rs::Environment::is_production_from_env
pub fn is_production_from_env() -> bool {
    std::env::var("ETL_ENVIRONMENT")
        .ok()
        .as_deref()
        .map(str::trim)
        .is_some_and(|v| v.eq_ignore_ascii_case("production") || v.eq_ignore_ascii_case("prod"))
    // `ETL_BUILD_ENV` fallback **제거**.
}
```

```rust
// services/etl-base-layer/src/config.rs::build_r2_config_strict
// - namespace 4개 모두 set → Ok(Some(R2Config))
// - namespace 부분 set → Err(ConfigError::PartialR2Namespace)  ← fail-fast
// - namespace 4개 모두 unset → Ok(None)  ← local-only mode
// - legacy `R2_*` fallback path **삭제**
```

```python
# services/scraper-py/dtmk_vworld.py::load_r2_credentials
# 동일 분기 (3개). legacy `R2_*` fallback **삭제**.
```

## 컨텍스트 — Codex Round 6 audit

Codex Round 6 read-only audit 가 박제한 19 trick 중 *config resolution 단의 root
structural fallback* 4개:

1. `crates/sp9-base-layer-config/src/types.rs:355` — `ETL_BUILD_ENV` backward-compat
   fallback violates explicit `ETL_ENVIRONMENT` SSOT
2. `services/etl-base-layer/src/config.rs:121` — namespaced R2 config falls back to legacy `R2_*`
3. `services/etl-base-layer/src/config.rs:151` — partial namespaced R2 credentials return
   `None`, enabling legacy fallback instead of fail-fast
4. `services/etl-base-layer/src/config.rs:128` — invalid `GOLD_VERSION` panics instead of
   returning typed config error

사용자 박제 ("trick 1개라도 거부, 표면이 아닌 근본") 기준 — ADR 0029 의 1-sprint
backward-compat 자체가 *표면 합의*. 진짜 SSS = 즉시 제거 + 회귀 invariant.

## 검토한 옵션

### A — 1-sprint 유예 후 제거 (ADR 0029 의 원래 path)
- 장점: 운영자 migration 시간
- **거부**: backward-compat path 가 *credential mix* 위험을 매 호출에서 재생산.
  사용자 사고 ("local smoke 가 prod R2 modify") 재발 가능성 잔존.

### B — 즉시 완전 제거 + 회귀 test (본 결정)
- 장점:
  - root structural fallback 0
  - partial namespace = typed `PartialR2Namespace` fail-fast (credential mix 차단)
  - `GOLD_VERSION` panic 제거 — typed `InvalidGoldVersion` err
  - Rust ↔ Python 동일 정책
- 단점:
  - operator 가 `R2_<ENV>_*` 즉시 마이그레이션 필요 — 단 workflow yml 은 이미
    namespace 사용 중 (Round 5+ migration 완료), 영향 X
- **채택**: production cron 영향 0 + SSS-grade 도달

## 변경 매트릭스

| 위치 | 이전 (ADR 0029) | 현재 (ADR 0030) |
|---|---|---|
| `Environment::is_production_from_env` | `ETL_ENVIRONMENT` 우선 + `ETL_BUILD_ENV` fallback | `ETL_ENVIRONMENT` only |
| `Config::from_env` R2 path | `build_r2_config_namespaced().or_else(legacy)` | `build_r2_config_strict()?` |
| Partial namespace | `None` (legacy fallback 활성) | `Err(PartialR2Namespace)` |
| Invalid `GOLD_VERSION` | `panic!` | `Err(InvalidGoldVersion)` |
| Python `load_r2_credentials` | 3-tier fallback (namespace → local fail-fast → legacy) | 2 분기 (namespace atomic → None) |
| Python `make_r2(creds=None)` | `load_r2_credentials()` 호출 | 동일 + `None` 시 `SystemExit` |

## SSS 7기둥 매핑

| 기둥 | 이전 (ADR 0029 backward-compat) | 본 결정 (ADR 0030 strict) |
|---|---|---|
| 일관성 | △ — 두 path 양립 | ✅ — 단일 path |
| 자동강제 | △ — backward-compat warning 만 | ✅ — typed err fail-fast |
| 추적성 | △ — legacy warning 박제 | ✅ — `PartialR2Namespace` 의 `present` / `missing` 박제 |
| 안전성 | ❌ — credential mix 가능 | ✅ — atomic 4-of-4 강제 |
| 가시성 | △ | ✅ — config 단계에서 즉시 detect |
| SSOT | ❌ — `R2_*` + `R2_<ENV>_*` 양립 | ✅ — `R2_<ENV>_*` 만 |
| 명확성 | ❌ — `ETL_BUILD_ENV` / `ETL_ENVIRONMENT` 양립 | ✅ — `ETL_ENVIRONMENT` 만 |

## 회귀 test (사고 invariant 박제)

Rust `services/etl-base-layer/src/config.rs::tests`:
- `legacy_r2_no_longer_activates_anywhere` — staging + legacy 4개 set → `None`
- `partial_namespace_fails_fast` — partial namespace = `PartialR2Namespace` err
- `invalid_gold_version_returns_typed_error` — `GOLD_VERSION=V3` → `InvalidGoldVersion` (no panic)
- `local_env_with_no_r2_credentials_is_local_only_mode` — local + no creds = OK

Python `services/scraper-py/tests/test_dtmk_vworld.py`:
- `test_load_r2_credentials_legacy_completely_ignored` — staging + legacy 4개 → None
- `test_load_r2_credentials_local_namespace_zero_returns_none` — local + unset = None
- `test_load_r2_credentials_partial_namespace_fails_fast` — partial = SystemExit

## 영향

### 신규
- `docs/adr/0030-legacy-r2-removal-and-atomic-namespace.md` (본 파일)
- `ConfigError::InvalidGoldVersion` variant
- `ConfigError::PartialR2Namespace` variant
- `build_r2_config_strict()` (replaces `build_r2_config_namespaced` + `build_r2_config_legacy`)

### 수정
- `crates/sp9-base-layer-config/src/types.rs::Environment::is_production_from_env` —
  `ETL_BUILD_ENV` fallback 제거
- `services/etl-base-layer/src/config.rs` — strict loader + typed errors + 회귀 test
- `services/etl-base-layer/src/main.rs::load_config_or_exit` — 3 variant 명시 stderr
- `services/etl-base-layer/src/main.rs` BuildLineage / init_sentry — `ETL_BUILD_ENV` fallback 제거
- `services/scraper-py/dtmk_vworld.py` — `load_r2_credentials` 단순화, legacy path 삭제
- `.env.example` — DEPRECATED section 제거

### 폐기
- `build_r2_config_namespaced` (replaced by `_strict`)
- `build_r2_config_legacy` (완전 삭제)

## 재검토 트리거

- 새 env (e.g. canary / qa) 추가 — `Environment` enum 확장 + `_R2_NAMESPACE_PREFIX` 자동 sync
- 외부 vault (AWS Secrets Manager / Doppler) 통합 결정 시 본 ADR 의 env-driven path
  단계적 교체 (별도 ADR)

## 참고

- ADR 0029 (Environment 명시 분리)
- AGENTS.md § 1 (자동 강제) + § 10.1.5 (Security & Privacy)
- 12-Factor App III. Config: https://12factor.net/config
