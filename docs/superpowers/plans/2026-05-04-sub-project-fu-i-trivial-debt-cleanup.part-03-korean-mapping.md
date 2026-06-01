# Sub-project FU-i Trivial Debt Cleanup - Part 03: Korean Mapping Expansion

Parent index: [Sub-project FU-i Trivial Debt Cleanup](./2026-05-04-sub-project-fu-i-trivial-debt-cleanup.md).

## Phase D: 한글 매핑 확장

> **2026-05-04 갱신 — 실 API 검증 결과 적용 (spec § 4.6 footnote 참조)**:
> T4 매핑 전략을 "30+ 한글 라벨 매핑" → **"Cd primary (5자리 표준코드) + CdNm fallback"** 하이브리드로 변경.
> 검증 fixture 5건 (`crates/data-clients/data-go-kr/tests/fixtures/real_*.json`).
> 추가로 `BR_TITLE_PATH` deprecated bug 발견 — `BldRgstService_v2` → `BldRgstHubService` 같이 fix (FU 41 prerequisite, 같은 PR).
> 아래 step 들의 한글 라벨 30+ 표기는 deprecated. 실제 구현은 commit 메시지 참조.

### Task 4: FU 41 — data.go.kr 한글 라벨 매핑 확장 → Cd primary + endpoint fix

**Files:**
- Modify: `crates/data-clients/data-go-kr/src/building_register/parser.rs`
- Modify: `crates/data-clients/data-go-kr/src/building_register/client.rs` (endpoint URL fix prerequisite)
- Add: `crates/data-clients/data-go-kr/tests/fixtures/real_*.json` (5건 실 API 응답)
- Add: `crates/data-clients/data-go-kr/tests/real_response_integration.rs` (fixture 기반 테스트)

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
