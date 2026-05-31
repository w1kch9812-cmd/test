# T2 Allowlists Migration - Part 03: V-World Reader, Docs, Verification, And Acceptance

Parent index: [T2 Allowlists Migration](./T2-allowlists-migration.md).


## Step 2.6: V-World reader source const (BLOCKER — 마이그과 동일 PR)

Spec §11 — `crates/data-clients/vworld/src/reader.rs:71` literal `"vworld"` → const `RAW_CAPTURE_SOURCE`.

- [ ] **Step 2.6.1: Modify `crates/data-clients/vworld/src/reader.rs` — add const at top**

기존 imports 아래 (struct 선언 위) 에 추가 (data-go-kr building reader 의 line 37 패턴 따름):

```rust
/// `parcel_external_data.source` 컬럼 값 (CHECK enum 일치).
///
/// 30012 마이그레이션 이후 'vworld' (legacy alias) 대신 'vworld_parcel' 사용.
/// 마이그레이션과 *동일 PR* 에 묶여야 함 — 마이그만 적용되고 코드가 'vworld'
/// 그대로 INSERT 시 backfill 직후 다시 'vworld' row 가 생긴다.
pub const RAW_CAPTURE_SOURCE: &str = "vworld_parcel";
```

- [ ] **Step 2.6.2: Modify line 71 — replace literal with const**

기존:
```rust
.capture(pnu.as_str(), "vworld", &raw, now)
```

→
```rust
.capture(pnu.as_str(), RAW_CAPTURE_SOURCE, &raw, now)
```

- [ ] **Step 2.6.3: Build + run vworld tests**

```bash
cargo check -p vworld
# Expected: Finished
cargo test -p vworld --lib
# Expected: all tests pass (literal change 만이라 logic 동일)
```

- [ ] **Step 2.6.4: Commit**

```bash
git add crates/data-clients/vworld/src/reader.rs
git commit -m "feat(sp10-5-b-T2): vworld reader source literal → RAW_CAPTURE_SOURCE const"
```

---

## Step 2.7: raw-capture lib.rs doc example update

- [ ] **Step 2.7.1: Modify `crates/data-clients/raw-capture/src/lib.rs:7-18` doc example**

기존:
```rust
//! capture.capture("1111010100100010000", "vworld", &raw_json, Utc::now()).await?;
```

→
```rust
//! capture.capture("1111010100100010000", "vworld_parcel", &raw_json, Utc::now()).await?;
//! // legacy 'vworld' source 는 migration 30012 backfill 후 'vworld_parcel' 로 통일됨
```

- [ ] **Step 2.7.2: Verify doc test still parses**

```bash
cargo doc -p raw-capture-client --no-deps
# Expected: Finished (doc example 은 `ignore` 라 실행 안 됨)
```

- [ ] **Step 2.7.3: Commit**

```bash
git add crates/data-clients/raw-capture/src/lib.rs
git commit -m "docs(sp10-5-b-T2): lib.rs example source vworld → vworld_parcel"
```

---

## Step 2.8: End-to-end verification (sanitize 실제 V-World fixture)

T2 의 모든 산출물 (allowlist + factory + reader const) 이 실 V-World fixture 와 호환되는지 통합 검증.

- [ ] **Step 2.8.1: Append failing integration test to `crates/data-clients/raw-capture/src/sources/vworld_parcel.rs`**

```rust
    #[test]
    fn sanitize_real_vworld_fixture_retains_jiga() {
        use crate::{AllowlistSanitizer, RawSanitizer};

        // 실 V-World 응답 fixture (gangnam yeoksam 737, jiga=67300000)
        let raw = serde_json::json!({
            "response": {
                "service": {"name": "data", "version": "2.0"},
                "status": "OK",
                "record": {"total": "1", "current": "1"},
                "page": {"total": "1", "current": "1", "size": "10"},
                "result": {
                    "featureCollection": {
                        "type": "FeatureCollection",
                        "features": [{
                            "type": "Feature",
                            "geometry": {"type": "MultiPolygon", "coordinates": []},
                            "properties": {
                                "pnu": "1168010100107370000",
                                "jibun": "737 대",
                                "addr": "서울특별시 강남구 역삼동 737",
                                "jiga": "67300000",
                                "gosi_year": "2025",
                                "gosi_month": "01"
                            }
                        }]
                    }
                }
            }
        });

        let san = AllowlistSanitizer::for_source("vworld_parcel").unwrap();
        let r = san.sanitize(&raw);

        // 공시지가 보존
        let jiga = r.value
            ["response"]["result"]["featureCollection"]["features"][0]["properties"]["jiga"]
            .clone();
        assert_eq!(jiga, "67300000");
        // status envelope 보존
        assert_eq!(r.value["response"]["status"], "OK");
    }
```

- [ ] **Step 2.8.2: Run — verify PASS (no drift, all allowlist paths match)**

```bash
cargo test -p raw-capture-client --lib sources::vworld_parcel::tests::sanitize_real_vworld_fixture
# Expected: ok. 1 passed
```

- [ ] **Step 2.8.3: Run full raw-capture test suite + clippy**

```bash
cargo test -p raw-capture-client --lib
# Expected: all tests pass (sanitizer + capture + sources = 20+ tests)
cargo clippy -p raw-capture-client -- -D warnings
# Expected: no warnings
cargo fmt --check
# Expected: no diff
```

- [ ] **Step 2.8.4: Commit**

```bash
git add crates/data-clients/raw-capture/src/sources/vworld_parcel.rs
git commit -m "test(sp10-5-b-T2): vworld_parcel sanitize real fixture (jiga retained)"
```

---

## Acceptance — T2 완료 기준

- [ ] `cargo test -p raw-capture-client --lib` 20+ test 모두 통과
- [ ] `cargo test -p vworld --lib` 회귀 0 (literal → const 변경만이라 logic 동일)
- [ ] `cargo clippy --workspace -- -D warnings` 통과
- [ ] migration 30012 forward + rollback 검증 완료
- [ ] `parcel_external_data` 의 모든 'vworld' row 가 'vworld_parcel' 로 backfill됨
- [ ] `RAW_CAPTURE_SOURCE` const 가 `crates/data-clients/vworld/src/reader.rs` 에 정의됨
- [ ] T3 에서 사용할 인터페이스 export: `sources::data_go_kr_building::{SOURCE_ID, BUILDING_ALLOWLIST}`, `sources::vworld_parcel::{SOURCE_ID, VWORLD_PARCEL_ALLOWLIST}`, `AllowlistSanitizer::for_source`, `SanitizerError`

**다음 task:** [T3-vault-kms-lineage.md](T3-vault-kms-lineage.md) — Two-tier vault migration (30013, 30014) + PgPiiVaultCapture + AWS KMS envelope encryption + DualTierCapture composer.
