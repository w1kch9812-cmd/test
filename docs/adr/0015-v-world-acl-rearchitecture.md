# ADR-0015: V-World ACL 재설계 — fixture-driven, layer-decomposed, envelope-aware

| | |
|---|---|
| 작성일 | 2026-05-06 |
| 상태 | Accepted |
| 결정자 | 사용자 |
| 컨텍스트 | V-World 실 API 첫 호출에서 `INVALID_RANGE` — 옛 ACL 이 한 번도 작동한 적 없음이 발견됨 |

## 컨텍스트

SP4-ii 에서 V-World ACL([`crates/data-clients/vworld/`](../../crates/data-clients/vworld/))
이 작성됐고, 단위·통합 테스트 모두 통과한 상태로 main 에 머지됨. 그러나 실
API 키로 처음 smoke test 를 돌렸을 때 모든 호출이 `Parse("missing
/response/result/featureCollection/features array")` 로 실패.

조사 결과 **세 층의 불일치** 발견:

1. **레이어 선택이 틀림** — `LAYER_USE_ZONE = "LT_C_UQ111"` (도시지역 용도지역)
   레이어로 PNU 조회 (`attrFilter=pnu:=:`) 시도. 그런데 `LT_C_UQ111` 은 PNU
   attribute 미보유 → V-World 가 `status: "ERROR" + code: INVALID_RANGE` 반환
2. **응답 envelope 무시** — 파서가 `status` 필드 분기 없이 곧장
   `result.featureCollection.features` 접근. `NOT_FOUND`/`ERROR` 응답에는
   `result` 자체가 없어 항상 `Malformed` 로 잘못 분류
3. **geometry 가정 오류** — 파서가 `Polygon` 만 허용. 실 V-World 응답은 항상
   `MultiPolygon`

## 근본 원인 (root causes)

테스트가 통과한 이유는 fixture 가 *상상*에서 만들어졌기 때문. 같은 사람이 같은
가정 아래 파서와 fixture 를 작성 → fixture 는 자기 검증을 못 함.

| ID | 원인 | 자동 강제 결여 |
|---|---|---|
| **R1** | Hand-crafted fixture (`sample_response()`) — 실 API 호출 0회 | 실 응답 fixture 명시 의무 없음 |
| **R2** | 단일 `parse_parcel()` 함수가 envelope + properties + geometry + admin 4 책임 혼합 | 책임 분리 강제 없음 |
| **R3** | `fetch_by_pnu` 가 단일 레이어 → "한 호출=완전한 Parcel" 가정 | V-World 데이터 모델이 multi-layer 라는 사실 반영 X |
| **R4** | 도메인 (`Parcel`) 이 `area`/`zoning` 비-Optional → `LP_PA_CBND_BUBUN` 미제공 필드를 채우라고 강제 | 도메인 invariants 가 외부 schema 와 mismatch |
| **R5** | `PolygonSrid` 만 존재 → MultiPolygon 표현 불가 | geometry 타입 다양성 미반영 |
| **R6** | smoke test 가 `#[ignore]` + nightly cron 에만 — 인간이 결과 안 봄 | drift 감지 닫힌 루프 (alert 없음) |
| **R7** | docs 가 잘못된 레이어/필드를 명시 (`docs/data-sources/v-world.md`) | docs 가 코드 진실로부터 검증되지 않음 |
| **R8** | 응답 ERROR envelope 손실 — 코드/메시지가 generic Malformed 에 묻힘 | 외부 에러 정보가 도메인까지 못 옴 |
| **R9** | sub-project plan 의 "Approved" 표식이 실 API 검증 없이 부여됨 | DoD 에 실 API smoke 요구 없음 |

## 결정

### 1. Fixture-driven (R1 차단)

- 모든 V-World 단위·통합 테스트는 `tests/fixtures/real_*.json` 만 사용
- `real_*` prefix 파일은 **실 API 호출 캡처에서만** 생성. hand-crafted 금지
- 통합 테스트 fixture 의 hand-crafted 잔재 제거 (이 ADR 시점)
- 향후: CI 에서 `real_*` 파일 변경 시 PR 설명에 캡처 명령(URL+날짜) 명시 의무

### 2. Layer decomposition (R2 차단)

```
crates/data-clients/vworld/src/
├── client.rs              # HTTP + Circuit Breaker (변경 없음)
├── envelope.rs            # status/error/features 분기 (NEW)
├── geometry.rs            # GeoJSON Polygon/MultiPolygon → MultiPolygonSrid (NEW)
├── layers/
│   ├── mod.rs
│   └── parcel_boundary.rs # LP_PA_CBND_BUBUN 전용 (NEW)
└── reader.rs              # layer parser 합성
```

각 모듈이 단일 책임. V-World 새 레이어 추가 = `layers/` 에 한 모듈 추가, envelope 무관.

### 3. 도메인 정렬 (R3, R4, R5 차단)

`Parcel` aggregate 변경:

- `geom: PolygonSrid` → `geom: MultiPolygonSrid` (`shared_kernel::geometry::MultiPolygonSrid` 신규)
- `area: AreaM2` → `area: Option<AreaM2>` (LP_PA_CBND_BUBUN 미제공)
- `zoning: Zoning` → `zoning: Option<Zoning>` (별도 spatial intersect 필요)
- 신규 `gosi_year_month: Option<GosiYearMonth>` (공시지가 lineage)
- `land_use_type` 은 `jibun` 마지막 토큰 ("737 대" → "대") 에서 도출

### 4. Envelope-aware error (R8 차단)

`ParseError::VWorldApi { code, text }` 신규 — V-World `status: "ERROR"` 응답의 코드/메시지를 그대로 보존.

`Outcome::NotFound` enum variant — `status: "NOT_FOUND"` 시 `result` 부재가 invariant. 호출자는 `Ok(None)`.

### 5. SSOT 동기화 (R7 차단)

- [docs/data-sources/v-world.md](../data-sources/v-world.md) 전면 재작성 (실 응답 기준)
- 모든 응답 예시가 `tests/fixtures/real_*.json` 에 1:1 대응
- 향후 fixture 변경 시 docs 동시 갱신 (PR 리뷰 책임)

### 6. 운영 가시성 (R6, R9 차단 — 부분)

본 ADR 에서는 *원칙* 만 결정 — 실행은 후속:

- smoke test 결과를 nightly cron 에서 Sentry/Slack 알림으로 push (별도 작업)
- 모든 외부 데이터 소스 sub-project 의 DoD 에 "실 API smoke 1회 통과" 추가
- ADR 0015 의 핵심 — *실 API 호출 없이 Approved 안 됨*

## 영향

### 변경된 crate

| Crate | 변경 |
|---|---|
| `shared-kernel` | `MultiPolygonSrid` + `EmptyMultiPolygon` error variant 추가 |
| `parcel-domain` | `Parcel` 필드 시그니처 변경 (`area`/`zoning` Option, `geom` MultiPolygon, `gosi_year_month` 신규) |
| `vworld-client` | `parser.rs` 삭제, `envelope`/`geometry`/`layers/parcel_boundary` 신규 |
| `data-go-kr-client` | `building_register/reader.rs` — V-World 호출이 `LP_PA_CBND_BUBUN` 사용 + `MultiPolygonSrid::first_polygon` 으로 `PolygonSrid` 추출 |

### 마이그레이션 부담

- `Parcel` 직접 구성하는 곳: **없음** (vworld 파서만 — 새 layer parser 가 대체)
- `parcel.geom`/`parcel.area` 접근: **2곳** — 본 ADR 시점에 모두 갱신됨

### 테스트 결과 (이 ADR 시점)

- `shared-kernel`: 34 passed (geometry 모듈)
- `parcel-domain`: 10 passed
- `vworld-client`: 30 unit + 7 wiremock 통합 — all passed
- `data-go-kr-client`: 47 unit + 6 통합 + 6 fixture passed

## 대안

### 대안 A: 표면 패치만 (`LAYER_USE_ZONE` 상수만 교체)

거부. R2-R9 가 모두 잠복. 다음 데이터 소스(법제처/data.go.kr 추가 endpoint)에서
같은 종류의 사고 재발 보장.

### 대안 B: 도메인 수정 보류 (`area`/`zoning` 그대로)

거부. 외부 schema 와 도메인 invariant mismatch 를 거짓말(0.0 sentinel) 로 메우면
SSS § 3 (추적성) + § 4 (안전성) 위반.

### 대안 C: 한 번에 zoning spatial intersect 도 통합

거부. 본 ADR 의 scope 는 *기존 코드의 부정확성 제거* + *재발 방지 메커니즘*.
새 기능(zoning intersect) 은 별도 sub-project (FU TBD).

## 참고

- 옛 spec: [docs/superpowers/specs/2026-05-04-sub-project-4-ii-vworld-parcel-reader-design.md](../superpowers/specs/2026-05-04-sub-project-4-ii-vworld-parcel-reader-design.md) — 본 ADR 로 일부 supersede
- raw 응답 캡처 fixture: `crates/data-clients/vworld/tests/fixtures/` (historical; Platform Core-owned after ADR 0034)
- 검증 패턴 baseline: `crates/data-clients/data-go-kr/tests/real_response_fixtures.rs` (historical; Platform Core-owned after ADR 0034)
> Current status (2026-05-28): Historical. V-World Catalog ingestion is now
> owned by Platform Core under ADR 0034. Do not recreate the local Gongzzang
> V-World client paths described below; Gongzzang consumes Platform Core
> published contracts only.
