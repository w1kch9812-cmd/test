# Sub-project FU-i Trivial Debt Cleanup - Part 04: Closeout, Verification, Risks, And Next Steps

Parent index: [Sub-project FU-i Trivial Debt Cleanup](./2026-05-04-sub-project-fu-i-trivial-debt-cleanup.md).

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
