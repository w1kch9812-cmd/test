# Sub-project FU-i: Trivial Debt Cleanup (Spec)

| | |
|---|---|
| 작성일 | 2026-05-04 |
| 상태 | Approved |
| 선행 | SP1-SP3, SP4-i/ii/iii-a/iii-d, SP5-i/ii/iii/iv (모두 완료) |
| 후속 | Small/Medium FU 들은 *영역별 sub-project* 진행 시 자연 묶임 (SP4-iii-b/c/e, SP-FU-OCC 등) |
| 본 sub-project 가 닫는 FU | 12, 13, 17, 18, 26, 41 (6건) |

---

## 1. 개요

본 sub-project 는 **어느 영역에도 안 묶이는 *trivial* FU 6 건** 을 한 번에 닫는 게 목표예요.

각 FU 는 변경 영향이 작아요 (doc-only / 1-2 file fix / clippy.toml 추가). 한 spec / 한 plan / 한 execution 으로 처리하는 게 SSS 분해 패턴.

영역별 SSOT 가 있는 FU (40/42/43/44 등) 는 본 sub-project 에 *포함 안* 함 — 그 FU 들의 SSOT 는 *해당 영역 sub-project* (SP4-iii-e PMTiles 등). 본 sub-project 에서 미리 처리하면 영역 sub-project 진행 시 *같은 코드 다시 수정* → 빚 추가.

---

## 2. 범위 (Scope)

### 포함 — 6 FU
1. **FU 12**: `listing_photo.id` prefix 표기 일관화 — spec inline `ph_` ↔ 실제 marker `lph_` (spec doc only)
2. **FU 13**: AuditLog spec § 4.3 mock SQL ↔ 실제 schema 정렬 — spec mock 의 `metadata` 컬럼이 실제로 없음, `before_state`/`after_state`/`ip_address`/`created_at` 으로 정정 (spec doc only)
3. **FU 17**: Trait doc stale 정리 — 코드가 SSOT, spec/trait doc 갱신
   - `AuditLogRepository::find_by_resource(limit)` `find_by_actor(since)` — spec 보다 실제 trait 가 풍부
   - `OperationsMetaRepository::find_unacknowledged_alerts` — doc 은 "created_at ASC" 이나 spec/impl/index 는 "severity DESC + created_at DESC"
4. **FU 18**: AuthCrate clippy 빚 — `crates/auth/src/verifier.rs` 의 pre-existing `clippy::panic` + `clippy::manual_let_else` 정리
5. **FU 26**: `clippy::disallowed_types` 로 `reqwest::Client` 직접 호출 차단 — workspace `clippy.toml` 에 추가, `data-clients/*` crate 들이 자체 wrapper (Circuit Breaker 통과) 만 사용하도록 강제
6. **FU 41**: 한글 라벨 매핑표 확장 — `crates/data-clients/data-go-kr/src/building_register/parser.rs` 의 `BuildingPurposeCode` / `BuildingStructureCode` enum 매핑이 28+ 케이스 누락 (현재 다수 `Other` fallback) → 표준 분류표 기준 풀 매핑

### 미포함 (별도 영역 sub-project 에서)
- FU 4 / 6 / 8 (BusinessNumber NTS, D₃D₄, KsicCode A-U) — 외부 데이터 + 인프라 (별도 SP-FU-IdValidation)
- FU 14 (BVQ/LRQ `updated_at`) — SP5 시리즈 잔재, 단독 SP-FU-OCC 와 묶음
- FU 15 (Repository.save `expected_version`) — 13 도메인 trait 변경 = 단독 SP-FU-OCC
- FU 16 (LRQ UNIQUE INDEX) — SP-FU-OCC 와 묶음 (BVQ/LRQ 영역)
- FU 28 / 29 / 30 — SP4 잔여 또는 SP7 (관측성)
- FU 40 — SP4-iii-e (PMTiles) 와 묶음
- FU 42 / 43 / 44 — SP4-iii-a 후속 또는 SP4-iii-b 와 묶음

---

## 3. 아키텍처

각 FU 가 독립적이라 별도 모듈 분해 없음. 작업은 *영향 받는 파일* 수준.

| FU | 파일 | 변경 종류 |
|---|---|---|
| 12 | `docs/superpowers/specs/2026-05-02-sub-project-2-db-core-domain-design.md` | spec doc 1 line |
| 13 | `docs/superpowers/specs/2026-05-03-sub-project-5-iii-...-design.md` § 4.3 | spec doc 5-10 line |
| 17 | `crates/domain/audit/audit-log/src/repository.rs` + `crates/operations/operations-meta/src/repository.rs` | rustdoc 갱신 |
| 18 | `crates/auth/src/verifier.rs` | code fix 2 곳 |
| 26 | `clippy.toml` (root) + `Cargo.toml` `[workspace.lints.clippy]` 또는 crate-level | clippy 설정 추가 |
| 41 | `crates/data-clients/data-go-kr/src/building_register/parser.rs` (또는 `building_codes.rs`) | enum 확장 + 매핑 |

---

## 4. 컴포넌트 정의

### 4.1 FU 12 — `listing_photo` prefix 일관화

`docs/superpowers/specs/2026-05-02-sub-project-2-db-core-domain-design.md` § 5.1 의 listing_photo 스키마 inline comment:
```sql
id char(30) primary key,                            -- ph_...
```
→
```sql
id char(30) primary key,                            -- lph_... (3-char prefix invariant; was `ph_` in earlier drafts)
```

이미 SP3 에서 동일 패턴 (`fc_` → `fea_`) 으로 처리한 적 있음. 같은 형식 따름.

### 4.2 FU 13 — AuditLog spec mock SQL 정정

`docs/superpowers/specs/2026-05-03-sub-project-5-iii-audit-pipeline-operations-rds-design.md` § 4.3 의 PgImpl 패턴 mock 의 `INSERT INTO audit_log` 절:

기존 (틀림):
```sql
INSERT INTO audit_log (
    id, actor_id, action, resource_kind, resource_id,
    metadata, correlation_id, occurred_at, client_ip, user_agent
)
```

수정:
```sql
INSERT INTO audit_log (
    id, actor_id, action, resource_kind, resource_id,
    before_state, after_state,
    ip_address, user_agent,
    correlation_id, created_at
)
```

§ 5.1 의 시퀀스 다이어그램 의 audit_log INSERT 설명도 동일하게.

§ 11 (FU 매핑) 에서 "FU 13 = AuditLog spec mock 정정" 항목을 "✅ closed by SP-FU-i" 로 표기.

### 4.3 FU 17 — Trait doc stale 정리

#### `crates/domain/audit/audit-log/src/repository.rs`
실제 trait method 시그니처:
```rust
async fn find_by_resource(
    &self,
    resource_kind: &str,
    resource_id: &str,
    limit: u32,
) -> Result<Vec<AuditLog>, RepoError>;

async fn find_by_actor(
    &self,
    actor_id: &Id<UserMarker>,
    since: DateTime<Utc>,
    limit: u32,
) -> Result<Vec<AuditLog>, RepoError>;
```

trait rustdoc 가 `limit` / `since` 파라미터의 의미 / 정렬 순서 / 활용 시점을 명시 안 함. 추가:
- `find_by_resource`: "최근 `limit` 건, `created_at` desc"
- `find_by_actor`: "`since` 시점부터, 최근 `limit` 건, `created_at` desc — admin audit 화면에서 사용"

#### `crates/operations/operations-meta/src/repository.rs`
`find_unacknowledged_alerts` rustdoc:
```rust
/// 미응답 alert 를 오래된 순(`created_at` ASC) 으로 최대 `limit` 건 반환.
```
→
```rust
/// 미응답 alert 를 *severity* 우선 (`critical > error > warning > info`) +
/// 동순위 내 `created_at` DESC 로 최대 `limit` 건 반환. spec § 5.5
/// `system_alert_unack_idx (severity, created_at desc) where acknowledged_at is null`
/// partial index 활용.
```

### 4.4 FU 18 — AuthCrate clippy 빚

`crates/auth/src/verifier.rs` 의 두 lint:
1. `clippy::panic` — test 코드 외부에 `panic!()` 호출이 있을 가능성 (실제 위치 grep 으로 식별 후 fix). 해결: `Result` 반환 또는 `expect_err` test helper 활용
2. `clippy::manual_let_else` — `if let ... else { return ... }` 패턴을 `let ... else { return ... };` 로 변경

`cargo clippy -p auth --all-targets -- -D warnings` 통과해야.

### 4.5 FU 26 — `clippy::disallowed_types` 로 `reqwest::Client` 차단

목적: `data-clients/*` 외 crate 가 `reqwest::Client` 를 직접 사용 못하게 강제. 모든 외부 HTTP 호출은 `crates/circuit-breaker` 의 `Breaker` 통과해야 (Sentry alert + retry 일관 처리).

방법:
- workspace root `clippy.toml` 신규 작성:
```toml
disallowed-types = [
    { path = "reqwest::Client", reason = "use circuit-breaker::Breaker wrapping reqwest, not direct (FU 26)" },
]
```

- 예외 crate (`data-clients/vworld`, `data-clients/data-go-kr`, `data-clients/raw-capture`, `crates/auth` JWKS fetcher 등) 는 crate-level `#![allow(clippy::disallowed_types)]` 또는 `clippy.toml` 에서 `allowed-paths` 활용 (clippy 0.x 옵션 확인 필요).

만약 `clippy.toml allowed-paths` 가 없으면, 차단 대신 *권장* 으로 약화 (`#![warn(clippy::disallowed_types)]`) 또는 별도 lint 설계.

### 4.6 FU 41 — 한글 라벨 매핑표 확장

`crates/data-clients/data-go-kr/src/building_register/` 의 한글→enum 매핑:

#### 현재 상태 (SP4-iii-a 산출물)
- 응답의 `mainPurpsCdNm` (주용도) / `strctCdNm` (구조) 가 한글 라벨
- 일부만 `BuildingPurposeCode` 엔num 매핑 (나머지 `Other` fallback)

#### 표준 분류표 (확장 대상)
- 건축물대장 주용도 코드: 약 28개 (단독주택/공동주택/근린생활시설1/2/문화및집회/판매/의료/교육/노유자/수련/운동/업무/숙박/위락/공장/창고/위험물/자동차관련/동물/식물/자원순환/교정/국방/방송/발전/묘지/관광휴게/장례)
- 구조 코드: 약 8-10개 (목구조/석조/벽돌/철근콘크리트/철골/철골철근콘크리트/조립식 등 — 이미 SP2b-ii 의 `BuildingStructureCode` 8개 정의되어 있음)

#### 작업
- `BuildingPurposeCode` 가 현재 어느 위치에 정의되어 있는지 확인 (SP2b-ii 또는 data-go-kr crate)
- 누락된 한글 라벨 매핑 추가 (대략 20+ 새 매핑)
- 신규 enum variant 가 필요하면 도메인에 추가 (Spec 변경)
- 단위 테스트로 라벨→enum 변환 검증

---

## 5. 데이터 흐름

본 sub-project 는 도메인/데이터 흐름 변경 없음. 모든 작업은 *기존 코드 보강* 또는 *문서 정정*. 새 데이터 흐름 추가 0.

---

## 6. 에러 매핑 정책

본 sub-project 는 새 에러 타입 도입 0. 기존 `RepoError` / `ClientError` / `AuthError` 변경 없음.

---

## 7. 가시성

신규 코드 ≤30 줄. 모든 신규 함수가 `tracing::instrument` 패턴 따르지만 본 sub-project 의 변경은 그 함수들 내부 *수정* 이라 instrument 추가 0.

---

## 8. 테스트 전략

### 8.1 단위 테스트 (FU 41)
- `BuildingPurposeCode::from_kor_label` (또는 `parse_purpose`) 28+ 케이스
- `BuildingStructureCode::from_kor_label` 8-10 케이스
- 누락된 라벨 → `Other` (fallback 유지)
- 빈 문자열 / `None` → `Other`

신규 단위 테스트 ~30+

### 8.2 단위 테스트 (FU 18)
- `verifier.rs` 의 변경된 코드가 기존 5+ tests 모두 그린 유지

### 8.3 단위 테스트 (FU 26)
- clippy.toml 가 적용되는지 *deliberate* test 로 검증 어려움 (lint 자체)
- 검증: workspace 의 비예외 crate 에서 `reqwest::Client` import 시 clippy 가 잡는지 — 본 sub-project 의 *최종 commit* 에서 cargo clippy --workspace --all-targets 통과 여부로 간접 검증

### 8.4 통합 테스트
없음. 본 sub-project 는 통합 테스트 신규 추가 0.

### 8.5 doc 정정 (FU 12, 13, 17)
- `markdownlint-cli2` 통과
- 코드 변경이 따르는 문서 → 코드 컴파일 영향 0

---

## 9. CI 통합

기존 3 workflow (CI / db-migrations / walking-skeleton) 그대로. 새 step 추가 없음. 신규 lint (FU 26) 는 기존 `cargo clippy` step 이 자동 catch.

---

## 10. 검증 기준 (DoD)

본 sub-project 종료 조건:

1. FU 12 — spec § 5.1 listing_photo inline `lph_` 표기로 정정됨
2. FU 13 — spec § 4.3 audit_log INSERT 컬럼이 실제 schema (`before_state`/`after_state`/`ip_address`/`created_at`) 와 일치
3. FU 17 — `AuditLogRepository::find_by_resource` `find_by_actor` rustdoc 갱신, `OperationsMetaRepository::find_unacknowledged_alerts` rustdoc 갱신
4. FU 18 — `cargo clippy -p auth --all-features --all-targets -- -D warnings` 통과 (panic + manual_let_else 해소)
5. FU 26 — workspace `clippy.toml` 에 `disallowed-types reqwest::Client` 추가, 비예외 crate 들이 통과
6. FU 41 — `BuildingPurposeCode` enum 28+ 변종 (또는 from_kor_label 매핑) + 단위 테스트 ≥30
7. 3 CI 워크플로우 (CI / db-migrations / walking-skeleton) 모두 그린
8. 누적 단위 테스트 ≥1241 + ~30 = ≥1270
9. tarpaulin ≥90% 유지
10. clippy `-D warnings` 통과 (단, FU 26 으로 신규 lint 추가)
11. 모든 파일 ≤500 권장 / ≤1500 강제
12. `roadmap.md` 의 "Spec FU 누적" 절에서 FU 12/13/17/18/26/41 ✅ closed 표기

---

## 11. SSS 7 기둥 매핑

| 기둥 | 본 sub-project 닫는 결함 |
|---|---|
| 1 일관성 | FU 12 (prefix), FU 13 (spec ↔ schema), FU 26 (HTTP 호출 강제 통일) |
| 2 자동 강제 | FU 26 (clippy 가 컴파일 시점 차단) |
| 3 추적성 | FU 26 (모든 외부 호출 Circuit Breaker 통과 = Sentry 추적) |
| 4 안전성 | FU 18 (auth crate panic 제거), FU 41 (한글 매핑 fallback 정확도) |
| 5 가시성 | FU 17 (trait doc 명확화), FU 26 (Breaker → Sentry alert 경로) |
| 6 SSOT | FU 13 (spec ↔ schema), FU 17 (trait doc ↔ impl) — 모두 SSOT 회복 |
| 7 명확성 | FU 12, 13, 17 (문서 정확) — 모두 명확성 강화 |

**6 FU 가 7 기둥 모두에 효과**. 개별 FU 작아도 누적 효과 큼 = SSS 결함 청산.

---

## 12. Follow-up items (본 sub-project 후 누적 FU 잔여)

본 sub-project 후 *영역 별 sub-project 와 묶일* FU 들:

### SP-FU-OCC (별도, 단독 가치) — Medium
- FU 14: BVQ/LRQ entity `updated_at` ↔ DB
- FU 15: Repository.save OCC API `expected_version` 명시
- FU 16: LRQ `find_by_listing` UNIQUE INDEX

### SP4-iii-b (data.go.kr 실거래가) — 자연 묶임
- FU 44: 토지대장 endpoint

### SP4-iii-e (PMTiles Reader 6) — 자연 묶임
- FU 30: `fetch_markers_in_bbox` PMTiles
- FU 40: `Building.geom` 정확한 footprint
- FU 42: `BuildingReader::fetch_by_id`
- FU 43: 캐시 정책 (`expires_at`)

### SP-FU-IdValidation (외부 데이터 표본 필요) — Large
- FU 4: BusinessNumber NTS 체크섬 외부 검증
- FU 6: BusinessNumber D₃D₄ 사업자 유형 코드
- FU 8: KsicCode 대분류 letter A-U

### SP7 (관측성) 또는 SP4 잔여 — 자연 묶임
- FU 28: Redis 캐시 (TTL 24h)
- FU 29: Sentry alert on Breaker open

`docs/superpowers/roadmap.md` 의 누적 FU 절을 본 sub-project 종료 시 갱신.

---

## 13. 후속 sub-project 시드

- **SP-FU-OCC**: FU 14 + 15 + 16 묶음 (Medium, 1-2일, 13 도메인 trait 변경 큰 리팩토링)
- **SP4-iii-b**: data.go.kr 실거래가 + FU 44
- **SP4-iii-e**: PMTiles Reader 6 + FU 30/40/42/43
- **SP-FU-IdValidation**: FU 4 + 6 + 8 (외부 사업자번호 표본 확보 후)
- **SP7**: 관측성 + FU 28/29 (Redis + Sentry 인프라)
