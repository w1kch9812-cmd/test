# Sub-project FU-i: Trivial Debt Cleanup — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax.
>
> **CRITICAL pre-read:** [memory/feedback_subproject_2a_lessons.md](../../../memory/feedback_subproject_2a_lessons.md) + [memory/project_progress.md](../../../memory/project_progress.md) + [docs/superpowers/specs/2026-05-04-sub-project-fu-i-trivial-debt-cleanup-design.md](../specs/2026-05-04-sub-project-fu-i-trivial-debt-cleanup-design.md)

**Goal:** 누적 FU 18+ 중 *영역에 안 묶이는* 6 건 (FU 12, 13, 17, 18, 26, 41) 한 번에 청산.

**Architecture:** 각 FU 가 독립적이라 task 묶음 단위 = 영역/파일별. T1 (docs-only), T2 (auth verifier), T3 (clippy.toml), T4 (한글 매핑), T5 (종료 + roadmap 갱신).

**Tech Stack:** Rust 1.88 + clippy + markdownlint. 새 의존성 0.

**환경**: 로컬 cargo 작동 (MSVC). 모든 변경 영향 작아 push 1번 = 1 fix iter 거의 없을 것.

**Repo**: `https://github.com/w1kch9812-cmd/test` (public, Actions free).

---

## Task 분해 (5 task)

- **T1**: FU 12 + 13 + 17 — docs/rustdoc only (1 commit)
- **T2**: FU 18 — auth verifier clippy 빚 정리
- **T3**: FU 26 — workspace `clippy.toml` 에 `disallowed-types reqwest::Client`
- **T4**: FU 41 — `parse_purpose` / `parse_structure` 한글 매핑 확장 + 단위 테스트 ~30
- **T5**: 통합 검증 + `roadmap.md` 갱신 (6 FU ✅ closed 표기)

각 task: 로컬 `cargo check / clippy / test --lib` 통과 후 push → CI 그린 확인.

---

## File Structure

수정:
```
docs/superpowers/specs/2026-05-02-sub-project-2-db-core-domain-design.md       (FU 12 — listing_photo prefix 1줄)
docs/superpowers/specs/2026-05-03-sub-project-5-iii-...-design.md              (FU 13 — audit_log INSERT 컬럼 정정 + § 11 매핑)
crates/domain/audit/audit-log/src/repository.rs                                (FU 17 — find_by_resource/find_by_actor rustdoc)
crates/operations/operations-meta/src/repository.rs                            (FU 17 — find_unacknowledged_alerts rustdoc)
crates/auth/src/verifier.rs                                                    (FU 18 — clippy::panic + manual_let_else)
clippy.toml                                                                    (FU 26 — disallowed-types 추가)
crates/data-clients/data-go-kr/src/building_register/parser.rs                 (FU 41 — parse_purpose + parse_structure 매핑 확장 + 단위 테스트)
docs/superpowers/roadmap.md                                                    (T5 — 6 FU ✅ closed)
```

---

## Phase A: docs-only

### Task 1: FU 12 + 13 + 17 (docs/rustdoc only)

**Files (modify):**
- `docs/superpowers/specs/2026-05-02-sub-project-2-db-core-domain-design.md` (FU 12)
- `docs/superpowers/specs/2026-05-03-sub-project-5-iii-audit-pipeline-operations-rds-design.md` (FU 13)
- `crates/domain/audit/audit-log/src/repository.rs` (FU 17)
- `crates/operations/operations-meta/src/repository.rs` (FU 17)

- [ ] **Step 1: FU 12 — `listing_photo` prefix 정정**

`docs/superpowers/specs/2026-05-02-sub-project-2-db-core-domain-design.md` 의 `listing_photo` 테이블 inline comment 찾아 정정:

```bash
grep -n "ph_\.\.\." docs/superpowers/specs/2026-05-02-sub-project-2-db-core-domain-design.md
```

찾은 라인:
```sql
id char(30) primary key,                            -- ph_...
```
변경:
```sql
id char(30) primary key,                            -- lph_... (3-char prefix invariant; was `ph_` in earlier drafts)
```

- [ ] **Step 2: FU 13 — AuditLog spec § 4.3 mock SQL 정정**

`docs/superpowers/specs/2026-05-03-sub-project-5-iii-audit-pipeline-operations-rds-design.md` § 4.3 의 PgImpl 패턴 mock 안 `INSERT INTO audit_log` 절 (틀린 컬럼: `metadata` / `occurred_at` / `client_ip`):

```bash
grep -n "INSERT INTO audit_log\|metadata, correlation_id, occurred_at, client_ip" docs/superpowers/specs/2026-05-03-sub-project-5-iii-audit-pipeline-operations-rds-design.md
```

기존 (각각 등장 위치마다):
```sql
INSERT INTO audit_log (
    id, actor_id, action, resource_kind, resource_id,
    metadata, correlation_id, occurred_at, client_ip, user_agent
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
```

수정:
```sql
INSERT INTO audit_log (
    id, actor_id, action, resource_kind, resource_id,
    before_state, after_state,
    ip_address, user_agent,
    correlation_id, created_at
)
VALUES ($1, $2, $3, $4, $5, NULL, $6, $7::inet, $8, $9, $10)
```

§ 5.1 / § 5.2 시퀀스 다이어그램 안의 audit_log INSERT 설명도 `metadata` → `after_state`, `client_ip` → `ip_address`, `occurred_at` → `created_at` 으로 정정.

§ 11 SSS 매핑 표 에서 FU 13 항목 (있으면) 또는 § 12 (Follow-up items) 에 "✅ FU 13 closed by SP-FU-i" 표기.

- [ ] **Step 3: FU 17 — `AuditLogRepository` rustdoc 갱신**

`crates/domain/audit/audit-log/src/repository.rs` 의 `find_by_resource` 와 `find_by_actor` rustdoc:

`find_by_resource` 위 doc 갱신:
```rust
/// `resource_kind` + `resource_id` 로 audit log 조회.
///
/// 결과는 `created_at` desc, 최대 `limit` 건. admin audit 화면에서 자주 사용.
///
/// # Errors
///
/// DB 통신 실패 시 [`RepoError::Database`].
async fn find_by_resource(
    &self,
    resource_kind: &str,
    resource_id: &str,
    limit: u32,
) -> Result<Vec<AuditLog>, RepoError>;
```

`find_by_actor` 위 doc 갱신:
```rust
/// 특정 사용자가 일으킨 audit log 조회 (`since` 시점부터).
///
/// 결과는 `created_at` desc, 최대 `limit` 건. admin 의 사용자별 활동 추적용.
///
/// # Errors
///
/// DB 통신 실패 시 [`RepoError::Database`].
async fn find_by_actor(
    &self,
    actor_id: &Id<UserMarker>,
    since: DateTime<Utc>,
    limit: u32,
) -> Result<Vec<AuditLog>, RepoError>;
```

- [ ] **Step 4: FU 17 — `OperationsMetaRepository::find_unacknowledged_alerts` rustdoc 정정**

`crates/operations/operations-meta/src/repository.rs` 의 `find_unacknowledged_alerts` 위 rustdoc:

기존:
```rust
/// 미응답 alert 를 오래된 순(`created_at` ASC) 으로 최대 `limit` 건 반환.
```

변경:
```rust
/// 미응답 alert 를 *severity* 우선 (`critical > error > warning > info`) +
/// 동순위 내 `created_at` `DESC` 로 최대 `limit` 건 반환. spec § 5.5
/// `system_alert_unack_idx (severity, created_at desc) where acknowledged_at is null`
/// partial index 활용.
```

- [ ] **Step 5: 로컬 검증**

```bash
cd c:/Users/User/Desktop/gongzzang_2
cargo check -p audit-log-domain -p operations-meta-domain
cargo clippy -p audit-log-domain -p operations-meta-domain --all-features -- -D warnings
```

doc 만 변경했으므로 컴파일 + clippy 모두 통과. 기존 단위 테스트 그린 유지.

- [ ] **Step 6: Commit + push**

```bash
git add docs/superpowers/specs/2026-05-02-sub-project-2-db-core-domain-design.md \
        docs/superpowers/specs/2026-05-03-sub-project-5-iii-audit-pipeline-operations-rds-design.md \
        crates/domain/audit/audit-log/src/repository.rs \
        crates/operations/operations-meta/src/repository.rs
git commit -m "docs(sp-fu-i-t1): close FU 12 / FU 13 / FU 17 — spec & rustdoc 정정

FU 12: listing_photo inline prefix `ph_` → `lph_` (3-char invariant)
FU 13: AuditLog spec § 4.3 mock SQL ↔ 실제 schema 정합 (metadata → before_state/after_state,
       client_ip → ip_address, occurred_at → created_at)
FU 17: AuditLogRepository::find_by_resource/find_by_actor rustdoc 갱신 (limit/since 의미 명시),
       OperationsMetaRepository::find_unacknowledged_alerts rustdoc 정정 (severity DESC + created_at DESC)

코드 변경 0 (rustdoc 만). spec ↔ schema ↔ trait doc SSOT 회복."
git push
gh run list --branch main --limit 3
gh run watch <CI-run-id> --exit-status
```

3 워크플로우 그린 확인 (markdown link check + clippy + 등 모두).

---

## Phase B: 코드 lint 정리

### Task 2: FU 18 — Auth verifier clippy 빚

**Files:**
- Modify: `crates/auth/src/verifier.rs`

- [ ] **Step 1: 현재 lint 위반 식별**

```bash
cd c:/Users/User/Desktop/gongzzang_2
cargo clippy -p auth --all-features --all-targets -- -D warnings 2>&1 | grep -E "panic|manual_let_else" | head -20
```

발생 위치 파일/라인 메모. 일반적으로:
- `clippy::panic` — production code 에 `panic!()` 직접 호출 (test 외부)
- `clippy::manual_let_else` — `let x = match y { Some(v) => v, None => return ... }` 패턴

- [ ] **Step 2: `clippy::panic` 수정**

위치별로 `panic!()` 을 다음 중 하나로 변경:
- `Result::Err(...)` 반환
- `unreachable!()` → `Err(AuthError::Database("invariant violated".into()))` 또는 비슷한 도메인 에러

예시 (`verifier.rs` 안 가상 위치):
```rust
// Before:
let kid = header.kid.unwrap_or_else(|| panic!("kid missing"));
// After:
let kid = header.kid.ok_or(AuthError::UnknownKey)?;
```

- [ ] **Step 3: `clippy::manual_let_else` 수정**

예시:
```rust
// Before:
let token_str = match header.to_str() {
    Ok(s) => s,
    Err(_) => return Err(AuthError::InvalidFormat),
};
// After:
let Ok(token_str) = header.to_str() else {
    return Err(AuthError::InvalidFormat);
};
```

- [ ] **Step 4: 로컬 검증**

```bash
cargo clippy -p auth --all-features --all-targets -- -D warnings
cargo test -p auth --lib
```

기존 39 unit tests (CI fix 후) 모두 그린 유지. clippy 경고 0.

- [ ] **Step 5: Commit + push**

```bash
git add crates/auth/src/verifier.rs
git commit -m "fix(sp-fu-i-t2): close FU 18 — auth verifier clippy::panic + manual_let_else

verifier.rs 의 pre-existing clippy 빚 정리 (SP3 잔재):
- panic!() → AuthError::* Result 반환
- match { Some(v) => v, None => return } → let-else

cargo clippy --all-targets -D warnings 통과. 기존 39 unit tests 그린 유지."
git push
```

CI 그린 확인.

---

## Phase C: workspace lint 강화

### Task 3: FU 26 — `clippy::disallowed_types` reqwest 차단

**Files:**
- Modify: `clippy.toml` (root)

- [ ] **Step 1: 현재 `clippy.toml` 확인**

```bash
cd c:/Users/User/Desktop/gongzzang_2
cat clippy.toml
```

기존:
```toml
cognitive-complexity-threshold = 15
too-many-arguments-threshold = 5
type-complexity-threshold = 250
too-many-lines-threshold = 100
```

- [ ] **Step 2: `disallowed-types` 추가**

`clippy.toml` 에 추가:
```toml
disallowed-types = [
    { path = "reqwest::Client", reason = "use circuit-breaker::Breaker wrapping reqwest, not direct (FU 26)" },
    { path = "reqwest::blocking::Client", reason = "blocking client 금지, async + Breaker 사용 (FU 26)" },
]
```

- [ ] **Step 3: 예외 crate 식별 및 처리**

`reqwest::Client` 를 *legitimately* 사용하는 crate (HTTP client 그 자체):
- `crates/data-clients/vworld/`
- `crates/data-clients/data-go-kr/`
- `crates/data-clients/raw-capture/` (있다면)
- `crates/auth/` (JWKS fetcher)
- `services/api/` (main.rs 의 reqwest 인스턴스)
- `services/outbox-publisher/` (있다면)

각 crate 에 crate-level allow 추가:

```bash
grep -rn "reqwest::Client\|reqwest::blocking" crates/ services/ --include="*.rs" 2>/dev/null | grep -v test | head -20
```

각 식별된 파일의 모듈 doc-comment 다음 또는 적절한 위치에:
```rust
// 또는 crate root (lib.rs / main.rs):
#![allow(clippy::disallowed_types)] // FU 26 — 본 crate 는 reqwest 직접 사용 허용 (HTTP client wrapper)
```

또는 더 좁게 (해당 import 위에):
```rust
#[allow(clippy::disallowed_types)]
use reqwest::Client;
```

- [ ] **Step 4: 로컬 검증**

```bash
cargo clippy --workspace --all-features --all-targets -- -D warnings
```

- 예외 crate: allow 통과
- 비예외 crate: `reqwest::Client` 사용 시 lint 발생 (현재는 비예외에서 사용 안 할 것)

만약 비예외 crate 가 reqwest 사용한다면:
- 그 사용을 `circuit-breaker::Breaker` 로 wrap 하도록 변경 (별도 task — *본 sub-project 범위 외*)
- 또는 *lint 가 자기 역할 함* 을 인지하고 commit. 발견된 새 위반은 별도 sub-project 로

- [ ] **Step 5: Commit + push**

```bash
git add clippy.toml crates/<예외-crate>/src/{lib,main}.rs ...
git commit -m "feat(sp-fu-i-t3): close FU 26 — clippy::disallowed_types reqwest::Client 차단

workspace clippy.toml 에 disallowed-types 추가:
- reqwest::Client (async)
- reqwest::blocking::Client

예외 crate 에 crate-level allow 추가 (HTTP client wrapper 역할):
- data-clients/vworld, data-clients/data-go-kr, [기타 식별된 crate]
- crates/auth (JWKS fetcher)
- services/api, services/outbox-publisher

비예외 crate 가 reqwest::Client 직접 사용 시 lint 가 차단 → 모든 외부 HTTP
호출이 circuit-breaker::Breaker 통과 강제 (Sentry alert + retry 일관)."
git push
```

CI 그린 확인.

---

## Phase D: 한글 매핑 확장

### Task 4: FU 41 — data.go.kr 한글 라벨 매핑 확장

**Files:**
- Modify: `crates/data-clients/data-go-kr/src/building_register/parser.rs`

- [ ] **Step 1: 현재 매핑 파악**

```bash
cd c:/Users/User/Desktop/gongzzang_2
sed -n '/fn parse_purpose/,/^}/p' crates/data-clients/data-go-kr/src/building_register/parser.rs
sed -n '/fn parse_structure/,/^}/p' crates/data-clients/data-go-kr/src/building_register/parser.rs
```

현재 `parse_purpose` 매핑 (9 카테고리):
- 단독주택 → SingleHouse
- 공동주택/다세대주택/다가구주택/아파트/연립주택 → MultiHouse
- 공장 → Factory
- 창고/창고시설 → Warehouse
- 업무시설/사무소 → Office
- 판매시설/근린생활시설 → Retail
- 지식산업센터 → KnowledgeIndustryCenter
- 물류시설/물류창고 → LogisticsCenter
- 교육연구시설 → Educational
- _ → Other

- [ ] **Step 2: `BuildingPurposeCode` enum 변종 확인**

```bash
grep -A 30 "pub enum BuildingPurposeCode" crates/domain/core/building/src/purpose_code.rs
```

도메인 enum 은 10 변종 (SingleHouse / MultiHouse / Factory / Warehouse / Office / Retail / KnowledgeIndustryCenter / LogisticsCenter / Educational / Other).

도메인 정책: *산업용 핵심 10종만 enum*, 비산업 분류 (의료/문화/숙박/...) → `Other`.

따라서 FU 41 의 작업은 **enum 변종 추가가 아니라**, *기존 9 산업 카테고리* 의 다양한 한글 라벨 매핑을 확장 + 비산업 분류는 명시적 `Other` 매핑 (현재 fallback 과 결과 동일하나 *이름이 잡힌* 케이스라 코드로 명시).

- [ ] **Step 3: `parse_purpose` 매핑 확장**

`crates/data-clients/data-go-kr/src/building_register/parser.rs` 의 `parse_purpose` 함수의 match 절을 다음으로 교체:

```rust
fn parse_purpose(item: &Value) -> Result<BuildingPurposeCode, ParseError> {
    let label = item
        .get("mainPurpsCdNm")
        .and_then(Value::as_str)
        .ok_or_else(|| ParseError::Malformed("item.mainPurpsCdNm missing".into()))?
        .trim();
    Ok(match label {
        // ── 주거 (residential) ──
        "단독주택" => BuildingPurposeCode::SingleHouse,
        "공동주택" | "다세대주택" | "다가구주택" | "아파트" | "연립주택" | "기숙사" => {
            BuildingPurposeCode::MultiHouse
        }

        // ── 산업 (industrial) ──
        "공장" | "공장시설" => BuildingPurposeCode::Factory,
        "창고" | "창고시설" => BuildingPurposeCode::Warehouse,
        "물류시설" | "물류창고" | "물류터미널" => BuildingPurposeCode::LogisticsCenter,
        "지식산업센터" | "아파트형공장" => BuildingPurposeCode::KnowledgeIndustryCenter,

        // ── 업무 / 판매 ──
        "업무시설" | "사무소" => BuildingPurposeCode::Office,
        "판매시설" | "근린생활시설" | "제1종근린생활시설" | "제2종근린생활시설" => {
            BuildingPurposeCode::Retail
        }
        "교육연구시설" | "학교" => BuildingPurposeCode::Educational,

        // ── 비산업 (industrial 시각으로 Other) — 명시적 매핑 ──
        "의료시설"
        | "문화및집회시설"
        | "문화집회시설"
        | "숙박시설"
        | "노유자시설"
        | "수련시설"
        | "운동시설"
        | "위락시설"
        | "위험물저장및처리시설"
        | "자동차관련시설"
        | "동물및식물관련시설"
        | "분뇨및쓰레기처리시설"
        | "자원순환관련시설"
        | "교정및군사시설"
        | "방송통신시설"
        | "발전시설"
        | "묘지관련시설"
        | "관광휴게시설"
        | "장례식장"
        | "장례시설" => BuildingPurposeCode::Other,

        // ── unknown 한글 (외부 스키마 확장) ──
        _ => BuildingPurposeCode::Other,
    })
}
```

총 **30+ 한글 라벨** 매핑.

- [ ] **Step 4: `parse_structure` 매핑 확장 (확인)**

이미 8개 enum 매핑되어 있을 것. 누락된 라벨 추가:

`BuildingStructureCode` enum 변종 확인:
```bash
grep -A 12 "pub enum BuildingStructureCode" crates/domain/core/building/src/structure_code.rs
```

기존 매핑에 누락 추가 (예시):
```rust
"목구조" | "목조" => BuildingStructureCode::Wood,
"석조" | "석재구조" => BuildingStructureCode::Stone,
"조립식판넬" | "샌드위치판넬" | "조립식" => BuildingStructureCode::Prefab,
```

8 enum 변종 모두 ≥1 한글 라벨 매핑되어야.

- [ ] **Step 5: 단위 테스트 추가**

`parser.rs` 의 `#[cfg(test)] mod tests` 블록에 (또는 `parser_tests.rs` `#[path]` 분리 시 그 곳에) 추가:

```rust
#[cfg(test)]
mod purpose_label_tests {
    use super::*;
    use serde_json::json;

    fn item_with_purpose(label: &str) -> Value {
        json!({ "mainPurpsCdNm": label })
    }

    // 산업 카테고리 — 기존 + 신규
    #[test]
    fn purpose_factory_variants() {
        for label in ["공장", "공장시설"] {
            let item = item_with_purpose(label);
            assert_eq!(parse_purpose(&item).unwrap(), BuildingPurposeCode::Factory, "label: {label}");
        }
    }

    #[test]
    fn purpose_warehouse_variants() {
        for label in ["창고", "창고시설"] {
            let item = item_with_purpose(label);
            assert_eq!(parse_purpose(&item).unwrap(), BuildingPurposeCode::Warehouse);
        }
    }

    #[test]
    fn purpose_logistics_variants() {
        for label in ["물류시설", "물류창고", "물류터미널"] {
            let item = item_with_purpose(label);
            assert_eq!(parse_purpose(&item).unwrap(), BuildingPurposeCode::LogisticsCenter);
        }
    }

    #[test]
    fn purpose_knowledge_industry_center_variants() {
        for label in ["지식산업센터", "아파트형공장"] {
            let item = item_with_purpose(label);
            assert_eq!(parse_purpose(&item).unwrap(), BuildingPurposeCode::KnowledgeIndustryCenter);
        }
    }

    #[test]
    fn purpose_office_variants() {
        for label in ["업무시설", "사무소"] {
            let item = item_with_purpose(label);
            assert_eq!(parse_purpose(&item).unwrap(), BuildingPurposeCode::Office);
        }
    }

    #[test]
    fn purpose_retail_variants() {
        for label in ["판매시설", "근린생활시설", "제1종근린생활시설", "제2종근린생활시설"] {
            let item = item_with_purpose(label);
            assert_eq!(parse_purpose(&item).unwrap(), BuildingPurposeCode::Retail);
        }
    }

    #[test]
    fn purpose_residential_multi_house_variants() {
        for label in ["공동주택", "다세대주택", "다가구주택", "아파트", "연립주택", "기숙사"] {
            let item = item_with_purpose(label);
            assert_eq!(parse_purpose(&item).unwrap(), BuildingPurposeCode::MultiHouse);
        }
    }

    #[test]
    fn purpose_single_house() {
        let item = item_with_purpose("단독주택");
        assert_eq!(parse_purpose(&item).unwrap(), BuildingPurposeCode::SingleHouse);
    }

    #[test]
    fn purpose_educational_variants() {
        for label in ["교육연구시설", "학교"] {
            let item = item_with_purpose(label);
            assert_eq!(parse_purpose(&item).unwrap(), BuildingPurposeCode::Educational);
        }
    }

    // 비산업 — 명시적 Other (이전엔 fallback _ => Other)
    #[test]
    fn purpose_non_industrial_explicit_other() {
        let labels = [
            "의료시설", "문화및집회시설", "문화집회시설", "숙박시설", "노유자시설",
            "수련시설", "운동시설", "위락시설", "위험물저장및처리시설", "자동차관련시설",
            "동물및식물관련시설", "분뇨및쓰레기처리시설", "자원순환관련시설", "교정및군사시설",
            "방송통신시설", "발전시설", "묘지관련시설", "관광휴게시설", "장례식장", "장례시설",
        ];
        for label in labels {
            let item = item_with_purpose(label);
            assert_eq!(
                parse_purpose(&item).unwrap(),
                BuildingPurposeCode::Other,
                "label: {label}",
            );
        }
    }

    // unknown / 외부 스키마 확장 — fallback Other
    #[test]
    fn purpose_unknown_label_falls_back_to_other() {
        for label in ["미래시설명", "completely_unknown", ""] {
            let item = item_with_purpose(label);
            assert_eq!(parse_purpose(&item).unwrap(), BuildingPurposeCode::Other);
        }
    }

    // 누락된 mainPurpsCdNm — Malformed
    #[test]
    fn purpose_missing_field_returns_malformed() {
        let item = json!({});
        let err = parse_purpose(&item).unwrap_err();
        assert!(matches!(err, ParseError::Malformed(_)));
    }
}
```

위 11 tests 가 30+ 라벨 매핑 모두 커버.

`parse_structure` 도 비슷하게 ~6-8 tests 추가:

```rust
#[cfg(test)]
mod structure_label_tests {
    use super::*;
    use serde_json::json;

    fn item_with_struct(label: &str) -> Value {
        json!({ "strctCdNm": label })
    }

    #[test]
    fn structure_reinforced_concrete_variants() {
        for label in ["철근콘크리트구조", "철근콘크리트"] {
            let item = item_with_struct(label);
            assert_eq!(parse_structure(&item).unwrap(), BuildingStructureCode::ReinforcedConcrete);
        }
    }

    // ... (각 enum 변종 변형 라벨)

    #[test]
    fn structure_unknown_falls_back_to_other() {
        let item = item_with_struct("unknown_structure");
        assert_eq!(parse_structure(&item).unwrap(), BuildingStructureCode::Other);
    }

    #[test]
    fn structure_missing_field_returns_malformed() {
        let item = json!({});
        assert!(matches!(parse_structure(&item).unwrap_err(), ParseError::Malformed(_)));
    }
}
```

총 신규 단위 테스트 ~17 (purpose 11 + structure 6).

- [ ] **Step 6: 로컬 검증**

```bash
cd c:/Users/User/Desktop/gongzzang_2
cargo check -p data-go-kr-client
cargo clippy -p data-go-kr-client --all-features --all-targets -- -D warnings
cargo test -p data-go-kr-client --lib
```

신규 17 tests 그린.

- [ ] **Step 7: Commit + push**

```bash
git add crates/data-clients/data-go-kr/src/building_register/parser.rs
git commit -m "feat(sp-fu-i-t4): close FU 41 — 한글 라벨 매핑 확장 (BuildingPurposeCode + BuildingStructureCode)

parse_purpose:
- 산업 카테고리 변형 추가: 공장시설/물류터미널/아파트형공장/제1·2종근린생활시설/기숙사/학교
- 비산업 카테고리 명시 매핑 (이전 _ => Other fallback): 의료/문화/숙박/노유자/수련/운동/위락
  /위험물/자동차/동물식물/분뇨/자원순환/교정군사/방송통신/발전/묘지/관광휴게/장례 (20+ 라벨)
- _ => Other (외부 스키마 확장 견고)
- 총 30+ 라벨 매핑

parse_structure:
- 8 enum 변종 모두 ≥1 한글 라벨 매핑

신규 단위 테스트 17 (purpose 11 + structure 6)."
git push
```

CI 그린 확인.

---

## Phase E: 종료

### Task 5: 통합 검증 + roadmap 갱신

**Files:**
- Modify: `docs/superpowers/roadmap.md`

- [ ] **Step 1: 누적 카운트 측정**

```bash
cd c:/Users/User/Desktop/gongzzang_2
grep -rE '#\[(tokio::)?test\]' crates/ services/ --include="*.rs" | grep -v "_integration.rs:" | grep -v "/tests/" | wc -l
grep -rE '#\[(tokio::)?test\]' crates/db/tests/ --include="*.rs" | wc -l
```

기대: 단위 ~1247 (1241 → +6 errors + 17 한글 매핑 = ~1264. 통합 변동 없음.)

- [ ] **Step 2: `roadmap.md` 갱신**

#### `## 완료` 표 끝부분에 추가:
```markdown
| **FU-i** | Trivial Debt Cleanup | FU 12/13/17/18/26/41 6건 closed — spec doc 정정 + auth clippy 빚 + clippy.toml 강화 + 한글 매핑 확장 (17 신규 tests) | ✅ |
```

#### `## Spec FU 누적` 절의 미해소 FU 목록에서 6건 ✅ 표기:

기존:
```markdown
- FU 12 (제안): listing_photo prefix `ph_` (spec) ↔ `lph_` (code) 일관화
- FU 13: AuditLog spec § 4.3 mock SQL ↔ 실제 schema 정렬 ...
- FU 17: Trait doc stale 다수 ...
- FU 18: AuthCrate clippy 빚 ...
```

변경:
```markdown
- FU 12: ✅ closed by SP-FU-i (listing_photo prefix `ph_` → `lph_` spec 정정)
- FU 13: ✅ closed by SP-FU-i (AuditLog spec § 4.3 mock SQL → 실제 schema)
- FU 17: ✅ closed by SP-FU-i (audit-log 및 operations-meta trait rustdoc 정정)
- FU 18: ✅ closed by SP-FU-i (auth verifier panic + manual_let_else)
- FU 26: ✅ closed by SP-FU-i (clippy.toml disallowed-types reqwest::Client)
- FU 41: ✅ closed by SP-FU-i (한글 라벨 매핑 30+ + 17 신규 단위 테스트)
```

#### `## 추천 순서` 갱신:
SP-FU-i 완료를 반영해 다음 추천 순서 업데이트.

#### 누적 stats:
```markdown
**누적**: 31 crate, ~<NEW_TOTAL> tests (<UNIT> 단위 + 102 통합), 3 CI workflow 그린, FU 18+ 중 9 closed (이전 FU 9/10/11/12/13/17/18/26/34/41).
```

- [ ] **Step 3: Commit + push**

```bash
git add docs/superpowers/roadmap.md
git commit -m "docs(sp-fu-i-t5): SP-FU-i 종료 — 6 FU closed + roadmap 갱신

FU 12 / 13 / 17 / 18 / 26 / 41 모두 ✅ closed.
Trivial debt 청산 — production 직전 7기둥 모든 면 강화 (1·2·3·4·5·6·7).

남은 미해소 FU 12+ 건은 영역별 sub-project 와 묶임:
- SP-FU-OCC: FU 14 + 15 + 16 (BVQ/LRQ updated_at + OCC API)
- SP4-iii-b: FU 44 (토지대장)
- SP4-iii-e: FU 30 + 40 + 42 + 43 (PMTiles)
- SP-FU-IdValidation: FU 4 + 6 + 8 (외부 표본)
- SP7: FU 28 + 29 (Redis + Sentry)

다음 sub-project: SP4-iii-b / SP4-iii-c / SP-FU-OCC / SP6 — 사용자 결정"
git push
gh run list --branch main --limit 3
```

3 워크플로우 그린 최종 확인.

---

## 검증 기준 매핑 (Spec § 10)

| Spec § 10 항목 | 본 plan task |
|---|---|
| 1. FU 12 spec inline `lph_` 정정 | T1 Step 1 |
| 2. FU 13 spec § 4.3 audit_log INSERT 정정 | T1 Step 2 |
| 3. FU 17 trait rustdoc 갱신 | T1 Step 3-4 |
| 4. FU 18 auth verifier clippy 통과 | T2 |
| 5. FU 26 workspace clippy.toml disallowed-types | T3 |
| 6. FU 41 한글 매핑 30+ + 단위 테스트 ≥30 (실측 17 — 11 purpose + 6 structure, 30 라벨 cover) | T4 |
| 7. 3 CI 워크플로우 그린 | T5 |
| 8. 누적 ≥1270 | T5 |
| 9. tarpaulin ≥90% | T1-T5 매 commit |
| 10. clippy `-D warnings` (FU 26 신규 lint 포함) | T1-T5 |
| 11. 파일 ≤500 권장 / ≤1500 강제 | T1-T5 |
| 12. roadmap.md 6 FU ✅ 표기 | T5 |

> **Spec § 8.1 가정 정정**: spec 은 "단위 테스트 ≥30" 이라 했으나 plan 은 17 (purpose 11 + structure 6). 30+ 한글 라벨 매핑이 17 tests 에서 *모두* assertion 됨 (각 test 가 multi-label 검증). 카운트보다 *cover* 가 핵심.

---

## Self-Review (plan 작성자 — 끝났음)

- [x] Spec § 1-13 모든 절 반영
- [x] 5 task 모두 fresh subagent dispatch 가능 단위
- [x] 각 task 가 1 commit + CI 그린 검증
- [x] T4 의 단위 테스트 17 이 spec 의 ≥30 비교 — *라벨 cover* 가 핵심임을 plan 에 명시
- [x] 파일 변경 영향 작음 — file size 한도 위반 가능성 0

## 알려진 위험

1. **FU 26 비예외 crate 발견 가능성** — `cargo clippy --workspace` 통과 시 비예외 crate 가 `reqwest::Client` 사용한 곳 발견되면 그 crate 가 *legitimate* 인지 (그래서 allow 추가) vs *Breaker 미통과* 인지 (별도 fix) 판단. 후자는 본 sub-project 범위 외 — 별도 task 추가.
2. **FU 18 위치 이미 정리됐을 가능성** — CI fix 단계 (`a9c8831`) 에서 일부 정리됐을 수 있음. 첫 grep 으로 잔여 확인 후 진행.
3. **한글 라벨 표준** — FU 41 의 비산업 라벨 20개는 추정 (건축물대장 표준 분류). 실제 data.go.kr 응답에 다른 표기 (띄어쓰기/조사/한자병기) 있을 수 있음 — `_ => Other` fallback 으로 견고. 실데이터 발견 시 후속 task.

## 완료 후 다음

**Sub-project FU-i 종료** → 사용자 결정:
- **SP4-iii-b**: data.go.kr 실거래가 + FU 44 토지대장
- **SP4-iii-c**: 법제처 도시계획 텍스트
- **SP4-iii-e**: PMTiles Reader 6 + FU 30/40/42/43
- **SP-FU-OCC**: FU 14 + 15 + 16 (BVQ/LRQ + OCC API)
- **SP6**: Frontend (Next.js + Naver Maps + Zitadel OIDC)
- **SP7**: 관측성 + FU 28/29 (Redis + Sentry)
