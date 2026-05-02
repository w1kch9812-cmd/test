---
name: Sub-project 2a 작업 패턴 교훈
description: Plan 2a 31 task 진행 중 학습된 4가지 재현 가능한 실패/성공 패턴
type: feedback
---

Sub-project 2a (DB + 마이그레이션 + shared-kernel + 14 값 객체) 진행 중 *반복 발생*한 패턴들. Plan 2b 이후 dispatch 작성 시 자동 적용.

## 1. Plan/spec 본문 paraphrase 신뢰 금지 (5건 catch)

**Why:** Plan 2a 작성 시 내가 spec § X를 paraphrase한 부분이 5번 spec 실제 내용과 어긋남:
- parcel/building/IndustrialComplex/Manufacturer 분류 (R2 정적인데 plan은 RDS Core)
- bookmark_external `target_type` (실제 `target_kind`, 6값 vs 4값)
- search_history 컬럼 구조 (실제 `query text + filters jsonb` not `query_payload jsonb`)
- analysis_report 필드 (실제 `target_pnus[] + snapshot` not `payload + expires_at`)
- ID prefix length (spec inline `ph_` 2자리 vs plan은 3자리)

**How to apply:**
- Implementer dispatch에 *spec § X 라인 범위* 직접 인용 → "verbatim 복사" 강제
- Plan 본문에 SQL/코드 inline 금지 (SSOT — spec이 정답)
- "Plan은 hint, spec은 truth — 충돌 시 spec 따르고 flag in report" 명시
- Plan 본문에 "spec § X (lines NNN-NNN)" 형태 anchor만 두기

## 2. 값 객체 dispatch 표준 패턴 (Tasks 12-25)

**Why:** Tasks 12-13에서 정립된 패턴이 14-25 모두 동일하게 재현. 매 dispatch에 같은 boilerplate 반복.

**How to apply (Plan 2b/2c value object 추가 시):**
- `#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]` (f64 newtype은 Eq/Hash 제외)
- `#[serde(transparent)]` newtype
- `try_new(&str) -> Result<Self, XError>` + `as_str/as_<inner>` accessor + `Display` + `FromStr`
- `#[must_use]` on accessors, `#[derive]` 빠뜨리지 말것
- *모든 식별자* 백틱 (`PNU`, `ASCII`, `Pnu`, `MoneyKrw` etc.) — `clippy::doc_markdown` 강제
- `# Errors` rustdoc on fallible methods
- *Provably-infallible expect* → `#[allow(clippy::expect_used)]` + `# Panics` rustdoc 정당화
- Test module: `#![allow(clippy::expect_used, clippy::unwrap_used)]`
- TDD 흐름: stub (compile only, runtime fail) → 실패 확인 → 실제 구현 → 통과 → commit
- *각 task당 1-3 CI iteration* 정상 (fmt, missing_const_for_fn, missing_panics_doc 자동 catch)

## 3. 마이그레이션 파일명 규약 (Task 26 발견)

**Why:** `V<major>_<minor>__<snake>.sql` 형식이 sqlx-cli 0.8.2에서 *깨짐*. sqlx는 첫 `_` 앞을 `i64::parse`로 시도하기 때문.

**How to apply:**
- 파일명 = `<5자리 정수 버전>_snake_case.sql` (예: `10001_core_tables.sql`, `30002_queue_optimistic_locking.sql`)
- 정수 버전 = `MM × 10000 + mmm` (M = major, m = minor, 0-padded)
- Major 1 = V001 초기 18 테이블, Major 2 = V002 role+trigger, Major 3 = V003 fixup
- 새 마이그레이션은 *마지막 정수 버전 + 1* 다음 번호
- Forward-only — 머지된 파일 절대 수정 X. 정정은 새 마이그레이션으로
- `migrations/00001_enable_postgis.sql` = 항상 첫 번째 (PostGIS 확장 의존 보장)

## 4. tarpaulin은 clippy --all-features보다 엄격 (Task 26 발견)

**Why:** Lint job 통과했지만 tarpaulin job에서 *컴파일 실패* 발생 (chrono `LocalResult` 케이스). `cargo clippy --all-features`는 일부 feature가 API를 다르게 노출시켜 통과시킬 수 있음. tarpaulin은 `cargo test --no-run` 수준의 *strict* 컴파일.

**How to apply:**
- 새 코드 작성 시 *반드시* test 실행이 통과해야 SSS — 단순 컴파일 통과로 충분 X
- 학습 데이터에 의존한 chrono/regex/serde API 사용 시 *문서 직접 검증* 우선
- `LocalResult<T>::single()` → `Option<T>` (chrono 0.4.x), `.expect()` 직접 호출 불가
- `Utc::now()` vs `Utc.with_ymd_and_hms()` API 차이 인지
- CI tarpaulin job이 *최후 진실* — 로컬 cargo check / lint 통과해도 안심 금지

## 5. Plan 2b 시작 전 deferred items 인지

다음 3건은 *코드 결함 아님* (외부 정책 의존)이라 Plan 2b 진행에 차단 없음. 단 production 배포 전 처리 필요:
- BusinessNumber NTS 체크섬 알고리즘 *실제 사업자번호 표본* 검증
- BusinessNumber `D₃D₄` 사업자 유형 코드 검증 (국세청 코드표 의존)
- KsicCode 대분류 letter `A`-`U` 강제 (KSIC 11차/12차 개정 추적)
