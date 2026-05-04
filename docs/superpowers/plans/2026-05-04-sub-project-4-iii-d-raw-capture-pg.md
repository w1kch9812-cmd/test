# Sub-project 4-iii-d: RawCapture trait 분리 + PgRawCapture — 계획

| | |
|---|---|
| 작성일 | 2026-05-04 |
| 상태 | Approved |
| 선행 spec | [`2026-05-04-sub-project-4-iii-d-raw-capture-pg-design.md`](../specs/2026-05-04-sub-project-4-iii-d-raw-capture-pg-design.md) |
| 추정 | 7 task, 1일 |

---

## T1 — spec + plan 커밋

이 commit. `docs(sp4-iii-d): spec + plan — RawCapture trait + PgRawCapture`

---

## T2 — `crates/data-clients/raw-capture` 신규 lib

```
crates/data-clients/raw-capture/
├── Cargo.toml
└── src/
    └── lib.rs
```

`lib.rs` 내용은 기존 `vworld-client/src/raw_capture.rs` 그대로 이동 + 모듈 docstring 으로 export 명시.

워크스페이스 `members` 에 추가.

**commit**: `feat(sp4-iii-d-t2): raw-capture-client lib — RawCapture trait + NoOpRawCapture`

---

## T3 — vworld-client 갱신

- `crates/data-clients/vworld/Cargo.toml` deps 에 `raw-capture-client` 추가
- `crates/data-clients/vworld/src/raw_capture.rs` 삭제
- `crates/data-clients/vworld/src/lib.rs` re-export 갱신:
  ```rust
  pub use raw_capture_client::{NoOpRawCapture, RawCapture};
  ```
- `crates/data-clients/vworld/src/error.rs` 의 `RawCaptureError` 도 raw-capture-client 로 이동 → vworld error.rs 가 re-export 또는 삭제

**commit**: `refactor(sp4-iii-d-t3): vworld-client uses raw-capture-client crate`

---

## T4 — 마이그레이션 V003_05 + db Cargo dep

- `migrations/30005_parcel_external_data.sql` 신규
- `crates/db/Cargo.toml` deps 에 `raw-capture-client` 추가

**commit**: `feat(sp4-iii-d-t4): migration V003_05 — parcel_external_data table`

---

## T5 — `PgRawCapture` 구현체

- `crates/db/src/raw_capture.rs` 신규
- `crates/db/src/lib.rs` `pub mod raw_capture;` 추가
- 구현: spec § 3.4 그대로

**commit**: `feat(sp4-iii-d-t5): PgRawCapture — UPSERT into parcel_external_data`

---

## T6 — 통합 테스트

`crates/db/tests/raw_capture_integration.rs`:
- 3 시나리오 (spec § 2)
- `truncate_all` 에 `parcel_external_data` 추가

**commit**: `feat(sp4-iii-d-t6): raw_capture integration tests`

---

## T7 — 검증 + push + SSOT

- 로컬 `cargo clippy --workspace --all-features --all-targets -- -D warnings` 그린
- push
- CI 3 workflow 그린 확인
- SSOT 갱신 (FU 27 종료 표기)

**commit**: `docs(sp4-iii-d-t7): SP4-iii-d 종료 — FU 27 closed (raw_response DB 저장)`

---

## 변경 파일 요약

| 분류 | 파일 | 변경 |
|---|---|---|
| 신규 lib | `crates/data-clients/raw-capture/{Cargo.toml, src/lib.rs}` | 신규 |
| 신규 마이그 | `migrations/30005_parcel_external_data.sql` | 신규 |
| 신규 PgImpl | `crates/db/src/raw_capture.rs` | 신규 |
| 신규 test | `crates/db/tests/raw_capture_integration.rs` | 신규 |
| workspace | `Cargo.toml` | members 추가 |
| db | `Cargo.toml` + `lib.rs` | dep + pub mod |
| vworld | `Cargo.toml` + `lib.rs` + `error.rs`, `raw_capture.rs` 삭제 | refactor |
| docs | spec + plan | 신규 |
| memory + roadmap | SSOT | 갱신 |

총 ~12 파일.

---

## 위험 요소

- **`db-migrations.yml` workflow** 가 마이그레이션 적용 후 schema 검증 — V003_05 추가 시 자동 실행
- **`truncate_all`** 에 `parcel_external_data` 추가 안 하면 통합 테스트 격리 깨짐
- **vworld-client backward compat**: 기존 호출자 (현재 0) 가 `vworld_client::RawCapture` 임포트하면 — re-export 유지
- **vworld-client `crates/data-clients/vworld/src/error.rs`** 의 `RawCaptureError` 이미 정의 — raw-capture-client 로 이동 후 vworld error.rs 에서 re-export
- **기존 `NoOpRawCapture` 의 `target = "vworld.raw"`** → 일반화하면서 `target = "raw.capture"` 로 변경 (논의: 호출자가 `tracing::filter::Target` 으로 필터링 — vworld 만 필터링 안 됨. `target` 인자 받기 옵션도 있지만 단순화 우선 — `raw.capture` 통일)
