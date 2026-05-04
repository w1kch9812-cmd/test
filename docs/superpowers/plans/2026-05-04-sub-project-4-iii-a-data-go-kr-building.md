# Sub-project 4-iii-a: data.go.kr 건축물대장 + BuildingReader — 계획

| | |
|---|---|
| 작성일 | 2026-05-04 |
| 상태 | Approved |
| 선행 spec | [`2026-05-04-sub-project-4-iii-a-data-go-kr-building-design.md`](../specs/2026-05-04-sub-project-4-iii-a-data-go-kr-building-design.md) |
| 추정 | 7 task, 1-2일 |

---

## T1 — spec + plan 커밋

이 commit. `docs(sp4-iii-a): spec + plan — data.go.kr 건축물대장 + BuildingReader`

---

## T2 — `Policy::data_go_kr_default()` 추가

**대상**: `crates/circuit-breaker/src/policy.rs`

```rust
impl Policy {
    pub const fn data_go_kr_default() -> Self {
        Self {
            timeout_ms: 15_000,
            max_retries: 2,
            retry_base_ms: 1_000,
            open_threshold: 5,
            open_window_ms: 5_000,
            open_cooldown_ms: 30_000,
        }
    }
}
```

단위 테스트 1개 추가.

**commit**: `feat(sp4-iii-a-t2): Policy::data_go_kr_default — 15s/retry 2회/30s cooldown`

---

## T3 — `crates/data-clients/data-go-kr` 신규 lib

```
crates/data-clients/data-go-kr/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── client.rs           (DataGoKrConfig + DataGoKrClient)
    ├── pnu_split.rs        (Pnu → PnuParts split)
    ├── error.rs            (ConfigError + ParseError)
    ├── building_register/
    │   ├── mod.rs
    │   ├── client.rs       (BuildingRegisterClient — getBrTitleInfo)
    │   ├── parser.rs       (data.go.kr JSON → Vec<Building>)
    │   └── reader.rs       (DataGoKrBuildingReader impl BuildingReader)
    └── (future: land_register, real_transaction)
```

**Cargo.toml deps**:
- building-domain, parcel-domain, shared-kernel
- circuit-breaker, raw-capture-client
- vworld-client (geom 합성용 — § 3.3 spec)
- async-trait, chrono, geo-types, reqwest, serde, serde_json, thiserror, tokio, tracing
- dev: wiremock, tokio[full]

**commit**: `feat(sp4-iii-a-t3): data-go-kr-client lib — Client + pnu_split + ACL parser`

---

## T4 — `DataGoKrBuildingReader` impl `BuildingReader`

`crates/data-clients/data-go-kr/src/building_register/reader.rs`:
- `fetch_by_pnu(pnu)`:
  1. `pnu_split(pnu)` → 5 분해 파라미터
  2. `BuildingRegisterClient.fetch_title_info(parts)` → raw JSON
  3. `raw_capture.capture(pnu, "data_go_kr_building", &raw, now)` (best-effort)
  4. `vworld_client.fetch_feature_by_pnu(LT_C_UQ111, pnu)` → polygon (geom 합성)
  5. `parse_building_title(raw, polygon, now)` → `Vec<Building>`
  6. `Ok(buildings)`
- `fetch_by_id`: 미구현 — `Err(Fetch("fetch_by_id deferred to FU 42"))`

**commit**: `feat(sp4-iii-a-t4): DataGoKrBuildingReader — BuildingReader impl with V-World geom 합성`

---

## T5 — 통합 테스트

`crates/data-clients/data-go-kr/tests/building_register_integration.rs`:
6 시나리오:
1. `fetch_by_pnu_happy_path` — 200 + 단일 건물 → Vec[1]
2. `fetch_by_pnu_multi_buildings` — items.item 배열에 3건물 → Vec[3]
3. `fetch_by_pnu_empty_returns_empty_vec` — items 빈 → Vec[]
4. `fetch_by_pnu_5xx_retries_then_fails` — 503 ×3 → Fetch
5. `fetch_by_pnu_malformed_returns_parse_error`
6. `fetch_by_pnu_circuit_opens_after_threshold`

mock 2개 (data.go.kr building register + V-World) — `wiremock::MockServer` 분리 또는 path-based dispatch.

**commit**: `feat(sp4-iii-a-t5): wiremock integration tests — building_register 6 scenarios`

---

## T6 — workspace + 검증 + push

- `Cargo.toml.members` 에 `crates/data-clients/data-go-kr` 추가
- 로컬 `cargo clippy --workspace --all-features --all-targets -- -D warnings` 그린
- push
- 3 CI workflow 그린 확인

**commit (필요 시)**: `fix(sp4-iii-a): <issue>`

---

## T7 — SSOT 갱신

- roadmap.md / project_progress.md / MEMORY.md
- 누적 31 crate, ~1230+ tests

**commit**: `docs(sp4-iii-a-t7): SP4-iii-a 종료`

---

## 위험 요소

- **`Building.geom` 합성 trade-off**: V-World 필지 폴리곤 = 건물 폴리곤 approximation. 정확한 footprint 는 SP4-iii-e (FU 40)
- **한글 코드 매핑**: `mainPurpsCdNm` `strctCdNm` 한글 → enum. 매핑표는 spec § FU 41 — 누락된 케이스는 `Other` fallback
- **API 키 필수**: `ODP_SERVICE_KEY` 미설정 시 `from_env` 실패 → 통합 테스트는 mock server 라 API 키 불필요
- **응답 단일/배열 다형**: data.go.kr 응답에서 `items.item` 이 1개일 때 객체, 다수일 때 배열인 경우 있음 — `serde_json::Value` 로 받아 양쪽 처리
- **`mgmBldrgstPk` BigInt**: 문자열로 받아 도메인에 미사용 (FU 42 용 보존)
