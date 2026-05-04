# Sub-project 4-ii: V-World 외부 API + ParcelReader — 구현 계획

| | |
|---|---|
| 작성일 | 2026-05-04 |
| 상태 | Approved |
| 선행 spec | [`2026-05-04-sub-project-4-ii-vworld-parcel-reader-design.md`](../specs/2026-05-04-sub-project-4-ii-vworld-parcel-reader-design.md) |
| 추정 | 7 task, 1-2일 |

---

## T1 — spec + plan 커밋

이미 작성된 spec/plan 함께 commit.

**commit**: `docs(sp4-ii): spec + plan — V-World ParcelReader + circuit-breaker`

---

## T2 — `crates/circuit-breaker` 신규 라이브러리

**대상**: `crates/circuit-breaker/` (현재 README 만 — 빈 crate)

```
crates/circuit-breaker/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── policy.rs       (Policy struct + vworld_default)
    ├── breaker.rs      (Breaker + CircuitState + Mutex<Inner>)
    ├── execute.rs      (execute fn + BreakerError enum)
    └── tests.rs        (12+ 단위 테스트)
```

**Cargo.toml**:
```toml
[package]
name = "circuit-breaker"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true

[dependencies]
thiserror = { workspace = true }
tokio = { workspace = true, features = ["time", "sync", "macros"] }
tracing = { workspace = true }

[dev-dependencies]
tokio = { workspace = true, features = ["macros", "rt", "test-util"] }
```

**workspace.members** 에 `crates/circuit-breaker` 추가.

**테스트 12+** — spec § 8 참조. `tokio::time::pause()` 로 결정적 시간 제어.

**검증**: 로컬 `cargo +1.88.0-x86_64-pc-windows-gnu clippy -p circuit-breaker --all-features --all-targets -- -D warnings`

**commit**: `feat(sp4-ii-t2): circuit-breaker library — Policy + Breaker + execute`

---

## T3 — `crates/data-clients/vworld` 신규 라이브러리

**대상**: `crates/data-clients/vworld/` 신규 (현재는 `crates/data-clients/README.md` 만)

```
crates/data-clients/vworld/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── client.rs       (VWorldConfig + VWorldClient + fetch_feature_by_pnu)
    ├── reader.rs       (VWorldParcelReader impl ParcelReader)
    ├── parser.rs       (V-World JSON → Parcel ACL)
    ├── raw_capture.rs  (RawCapture trait + NoOpRawCapture)
    └── error.rs        (ParseError + ConfigError + RawCaptureError)
```

**Cargo.toml**:
```toml
[package]
name = "vworld-client"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true

[dependencies]
parcel-domain = { path = "../../domain/core/parcel", version = "0.1.0" }
shared-kernel = { path = "../../domain/core/shared-kernel", version = "0.1.0" }
circuit-breaker = { path = "../../circuit-breaker", version = "0.1.0" }
async-trait = { workspace = true }
chrono = { workspace = true }
geo-types = { workspace = true }
reqwest = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }

[dev-dependencies]
tokio = { workspace = true, features = ["macros", "rt-multi-thread"] }
wiremock = { workspace = true }
```

**Note**: `parcel-domain` 의 actual package name 확인 필요. `crates/domain/core/parcel/Cargo.toml` 의 `name`. 현재까지는 다른 도메인이 `<name>-domain` 형태였음.

**workspace.members** 에 `crates/data-clients/vworld` 추가.

**workspace.deps** 에 `wiremock = "0.6"` 추가.

**테스트**:
- `parser.rs` 안 `#[cfg(test)]` — fixture JSON 5-7 케이스
- `tests/integration.rs` — wiremock 6 케이스

**검증**: `cargo +1.88.0-x86_64-pc-windows-gnu clippy -p vworld-client --all-features -- -D warnings` (단, `wiremock` 등 통합 deps 가 link 필요 시 일부만 가능)

**commit**: `feat(sp4-ii-t3): vworld-client — VWorldParcelReader + ACL parser + raw_capture`

---

## T4 — workspace.members + workspace.deps 갱신

**대상**: 루트 `Cargo.toml`

```toml
members = [
    # 기존 ...
    "crates/circuit-breaker",
    "crates/data-clients/vworld",
]

[workspace.dependencies]
# 기존 ...
wiremock = "0.6"
```

**검증**: `cargo metadata --no-deps` clean.

**commit**: `chore(sp4-ii-t4): workspace — add circuit-breaker + vworld-client + wiremock dev-dep`

---

## T5 — wiremock 통합 테스트

**대상**: `crates/data-clients/vworld/tests/vworld_integration.rs`

6 시나리오 (spec § 8). `wiremock::Mock` 을 사용해 fake V-World server. `VWorldClient::with_policy` 에 `base_url` override 로 mock 서버 가리키게.

**commit**: `feat(sp4-ii-t5): vworld wiremock integration tests`

---

## T6 — 종합 검증 + push + CI 모니터링

**로컬**:
- `cargo fmt --all --check` ✓
- `cargo metadata --no-deps` ✓
- `cargo clippy -p circuit-breaker -p vworld-client --all-features --all-targets -- -D warnings` (proc-macro 만 deps 라 link 가능 가능성 — 시도)
- `cargo test -p circuit-breaker` (link 필요 — MinGW 부재로 실행 불가 가능 — push 후 CI)

**push**:
```bash
git push origin main
```

**CI 모니터링**: 3 workflow 그린 확인.

**실패 시 fix**: SP4-i / SP5-ii 학습 — clippy false positive 미리 차단:
- `circuit-breaker` 의 `Breaker` / `BreakerError` — `#![allow(clippy::module_name_repetitions)]` 미리
- `vworld-client` 의 `VWorldClient` / `VWorldConfig` — 동일

---

## T7 — SSOT 갱신

**대상**:
- `docs/superpowers/roadmap.md`:
  - 완료 표 SP4-ii 행
  - 다음 SP 후보: SP4-iii 권장 (data.go.kr + 법제처 + R2 Reader 6 + raw_response DB 저장)
- `memory/project_progress.md`:
  - 새 섹션 `### Sub-project 4-ii: V-World ParcelReader`
  - 누적 카운트 ~1190+ tests, 29 crate
- `MEMORY.md` 한 줄

**commit**: `docs(sp4-ii-t7): SP4-ii 종료 — V-World ParcelReader + circuit-breaker`

---

## 변경 파일 요약 (예상)

| 분류 | 파일 | 변경 |
|---|---|---|
| 신규 lib | `crates/circuit-breaker/{Cargo.toml, src/{lib,policy,breaker,execute,tests}.rs}` | 신규 |
| 신규 lib | `crates/data-clients/vworld/{Cargo.toml, src/{lib,client,reader,parser,raw_capture,error}.rs, tests/vworld_integration.rs}` | 신규 |
| workspace | `Cargo.toml` | members + deps |
| docs | spec + plan + roadmap | 신규 / 갱신 |
| memory | project_progress + MEMORY.md | 갱신 |

총 ~16 신규 파일.

---

## 위험 요소

- **`reqwest::Error` Display vs Debug**: BreakerError<E> 의 E: Display bound. reqwest::Error 은 Display 구현 ✓. 그대로 작동.
- **`tokio::time::pause()`** 가 단위 테스트에서 작동하려면 `[tokio::test(flavor = "current_thread", start_paused = true)]` 필요. 또는 `tokio::test` + 명시적 `pause()`.
- **`PolygonSrid::try_new_wgs84`** 는 `geo_types::Polygon` 받음. V-World JSON 의 coordinates 는 `Vec<Vec<Vec<f64>>>` (GeoJSON Polygon) — 변환 필요.
- **PNU 19자리 분해 → AdminDivision**: `AdminDivision::try_new(SidoCode, SigunguCode, EupmyeondongCode)`. PNU `1111010100100010000`:
  - `11` = sido (서울)
  - `11010` = sigungu (종로구)
  - `100` = eupmyeondong
  - `1` = 산/대 구분
  - `0001` = 본번
  - `0000` = 부번
  Confirm with shared-kernel/admin_division.rs.
- **wiremock 사용 시 `tokio::test`**: wiremock 은 자체 runtime 사용 — 충돌 가능. wiremock docs 확인.
- **circuit breaker 의 `Mutex<Inner>`**: tokio Mutex 보단 std::sync::Mutex 가 가볍고 충분 (lock 시간이 매우 짧음). Mutex 선택 시 std 선호.
