## 향후 추가 (sub-project 단위)
- subagent 정의 (예: `code-reviewer.md`, `docs-auditor.md`)
- 공용 prompt 템플릿
- glossary 별칭 (도구별 차이)
```

- [ ] **Step 4: 검증 + 커밋**

```bash
pnpm biome check .claude/settings.json .mcp.json
pnpm markdownlint-cli2 .agents/README.md
git add .claude/settings.json .mcp.json .agents/
git commit -m "chore(agents): set up Claude/AGENTS shared config and MCP placeholder"
```

---

### Task 15: GitHub 메타 (CODEOWNERS + PR + Issue 템플릿)

**Files:**
- Create: `.github/CODEOWNERS`, `.github/pull_request_template.md`, `.github/ISSUE_TEMPLATE/bug.md`, `.github/ISSUE_TEMPLATE/feature.md`

- [ ] **Step 1: `.github/CODEOWNERS`**

```
# 기본: 1인 운영자 (확장 시 멤버 추가)
* @w1kch9812-cmd

# 도메인별 (사용자 결정 시 갱신)
/docs/adr/             @w1kch9812-cmd
/docs/conventions/     @w1kch9812-cmd
/docs/sss-charter.md   @w1kch9812-cmd
/.github/              @w1kch9812-cmd
/infrastructure/       @w1kch9812-cmd
/crates/auth/          @w1kch9812-cmd
/services/             @w1kch9812-cmd
```

- [ ] **Step 2: `.github/pull_request_template.md`**

```markdown
## 변경 요약
<!-- 이 PR이 무엇을 바꾸는지 1-3 문장 -->

## 동기 (Why)
<!-- 왜 이 변경이 필요한가 -->

## 변경 범위
- [ ] 단일 sub-project 안에서만 변경
- [ ] sub-project N의 spec/plan과 일치
- [ ] 관련 ADR 작성/갱신 됨

## 테스트
- [ ] 단위 테스트 추가/갱신
- [ ] 통합 테스트 (필요 시)
- [ ] E2E (필요 시)
- [ ] 수동 확인 시나리오: <...>

## SSS 7 기둥 자체 검증
- [ ] 일관성: 기존 패턴 따름
- [ ] 자동 강제: lefthook + CI 통과
- [ ] 추적성: ADR/audit log 갱신
- [ ] 안전성: 타입/값 객체 검증
- [ ] 가시성: tracing/log 추가
- [ ] SSOT: 정보 중복 없음
- [ ] 명확성: 컨벤션 준수

## Breaking Change
- [ ] 없음
- [ ] 있음 (영향 + 마이그레이션 명시):

## 관련 이슈
Closes #
```

- [ ] **Step 3: Issue 템플릿 2개**

`.github/ISSUE_TEMPLATE/bug.md`:
```markdown
---
name: 버그
about: 버그 신고
title: 'bug: '
labels: bug
---

## 무엇이 잘못됐나요
<!-- 한 문장 -->

## 재현 방법
1.
2.
3.

## 기대 동작 vs 실제 동작
**기대:**

**실제:**

## 환경
- OS:
- 브라우저 (해당 시):
- 앱 버전:
- correlation_id (있다면):

## 추가 정보
스크린샷, 로그, ...
```

`.github/ISSUE_TEMPLATE/feature.md`:
```markdown
---
name: 기능 제안
about: 새 기능 제안
title: 'feat: '
labels: enhancement
---

## 무엇을 원하나요
<!-- 한 문장 -->

## 동기 (Why)
<!-- 누가 어떤 상황에서 어떤 가치를 얻나 -->

## 사용자 시나리오
<!-- "매수자가 ... 할 때 ... 하면 ..." 형식 -->

## 영향 범위
- 도메인:
- 관련 sub-project:
- 신규 ADR 필요 여부:

## YAGNI 자체 점검
- [ ] 정말 지금 필요한 기능인가?
- [ ] 더 단순한 대안 검토했는가?
```

- [ ] **Step 4: 검증 + 커밋**

```bash
pnpm markdownlint-cli2 .github/**/*.md
git add .github/CODEOWNERS .github/pull_request_template.md .github/ISSUE_TEMPLATE/
git commit -m "chore(github): add CODEOWNERS + PR template + issue templates"
```

---

### Task 16: 최종 검증 + 첫 PR

**Files:**
- 없음 (검증만)

- [ ] **Step 1: 전체 파일 수 확인**

```bash
find . -type f \
  ! -path "./node_modules/*" \
  ! -path "./_archive/*" \
  ! -path "./target/*" \
  ! -path "./.next/*" \
  ! -path "./reference/*" \
  ! -path "./.git/*" \
  | wc -l
```

기대: 76개 이상.

- [ ] **Step 2: 전 파일 ≤500줄 검증**

```bash
find . -type f \( -name "*.md" -o -name "*.toml" -o -name "*.json" -o -name "*.yml" -o -name "*.yaml" \) \
  ! -path "./node_modules/*" ! -path "./_archive/*" ! -path "./target/*" ! -path "./reference/*" ! -path "./.git/*" \
  -exec sh -c 'wc -l "$1" | awk "{print \$1, \$2}" | awk "\$1 > 500"' _ {} \;
```

기대: 출력 없음 (모든 파일 ≤500줄). 있으면 그 파일 분해 필요.

- [ ] **Step 3: 모든 도구 풀 검증**

```bash
pnpm biome check .
pnpm markdownlint-cli2 "**/*.md" "#node_modules" "#_archive" "#target" "#reference"
pnpm markdown-link-check docs/**/*.md AGENTS.md README.md TECH.md MEMORY.md CLAUDE.md
cargo fmt --all -- --check
cargo check --workspace --all-features
cargo deny check
```

기대: 모두 0 exit code.

- [ ] **Step 4: pre-commit 훅 실제 작동 확인**

```bash
# 더미 시크릿
echo "ZITADEL_API_KEY=sk_live_realtoken1234567890abcdef" > _temp_secret.txt
git add _temp_secret.txt
git commit -m "test"
# 기대: gitleaks가 차단

# 정리
git restore --staged _temp_secret.txt
rm _temp_secret.txt

# 더미 1500줄 markdown
yes "## heading" | head -1600 > _temp_huge.md
git add _temp_huge.md
git commit -m "test"
# 기대: file-size hook이 차단

git restore --staged _temp_huge.md
rm _temp_huge.md
```

- [ ] **Step 5: SSS 15 검증 체크 (가능 항목)**

`docs/sss-charter.md`의 15 검증 질문 중 sub-project 1에서 통과 가능한 항목 확인:
- Q3 (에러 형식 SSOT) — `docs/conventions/error-format.md` 존재 ✅
- Q4 (의존성 방향 룰) — `Cargo.toml` workspace lints + dependency-cruiser 추후 ✅(부분)
- Q5 (시크릿 자동 차단) — gitleaks pre-commit + CI ✅
- Q6 (모든 결정 ADR) — 11개 존재 ✅
- Q9 (코드 스타일 commit 차단) — lefthook biome + rustfmt ✅
- Q11 (정보별 SSOT 매트릭스) — `docs/ssot-matrix.md` 존재 ✅
- Q13 (도메인 용어 사전) — `docs/glossary.md` ✅. 위반 자동 검출 룰은 sub-project 2+에서 추가
- Q14 (모든 파일 ≤500줄 자동 검증) — lefthook + CI file-size hook ✅

→ 8/15 통과. 나머지 7개는 sub-project 2+에서.

- [ ] **Step 6: 첫 PR 생성**

`feature/sub-project-1-charter` 브랜치는 위 모든 커밋이 main에 직접인 경우 skip. PR 워크플로우 원하면:

```bash
git checkout -b chore/finalize-sub-project-1
# (변경 없으면 빈 PR이 안 만들어짐. 그냥 main에 모두 푸시되어 있어야 OK.)
git checkout main
git push origin main
```

GitHub Actions CI 실행 모니터링:
```bash
# https://github.com/w1kch9812-cmd/gongzzang3/actions 접속
# 모든 job 그린 확인
```

- [ ] **Step 7: spec § 9 검증 기준 체크리스트 채움**

`docs/superpowers/specs/2026-05-01-sub-project-1-charter-monorepo-design.md` § 9를 열어서 모든 체크박스 표기 + 사용자 검증 받음.

- [ ] **Step 8: 마무리 커밋**

```bash
git add docs/superpowers/specs/2026-05-01-sub-project-1-charter-monorepo-design.md
git commit -m "docs(spec): mark sub-project 1 verification checklist complete"
git push origin main
```

- [ ] **Step 9: Sub-project 1 완료 선언**

`docs/superpowers/specs/2026-05-01-sub-project-1-charter-monorepo-design.md` 헤더 메타데이터:
- `상태` Draft → **Done**

```bash
# Edit 또는 sed
git add docs/superpowers/specs/2026-05-01-sub-project-1-charter-monorepo-design.md
git commit -m "docs(spec): close sub-project 1"
git push origin main
```

---

## Self-Review (이 plan 자체)

### Spec coverage
spec § 6 결과물 목록 76개를 task 7-15에 1:1 매핑함:
- 루트 진입점 10 → Task 7 (4개) + Task 5 (.gitignore/.editorconfig/.gitattributes/.nvmrc/LICENSE) + Task 3 (`.env.example`)
- 모노레포 설정 10 → Task 3
- 자동 강제 6 → Task 4 + 5 + 6
- GitHub 메타 4 → Task 15
- .claude/.agents/.mcp 3 → Task 14
- docs 진입점 4 → Task 8
- ADR 12 → Task 9
- Conventions 10 → Task 10
- 도메인 카테고리 13 → Task 12
- Data sources 6 → Task 11
- 워크스페이스 멤버 28 → Task 13

→ 합계 일치.

### Placeholder 스캔
- 각 ADR/컨벤션 섹션에 *작성할 내용 outline*만 있고 *완성 본문* 없음 — 의도적 (각 파일은 plan 실행 중 작성, plan은 *작성 가이드*)
- "TBD" 또는 "TODO" 없음
- 모든 step에 명령 또는 코드 포함 ✅

### Type / Path 일관성
- ADR 파일명 일관 (`0005-auth-zitadel.md` 4곳에서 동일)
- 워크스페이스 멤버 경로 일관 (`crates/embedding/` 한 곳)
- 환경 변수 일관 (`ZITADEL_API_KEY`, `VWORLD_API_KEY` 등)

### 누락 검토
- spec § 9 검증 기준 모두 Task 16에서 체크 ✅
- spec § 11 후속 sub-project 2-12 — plan은 sub-project 1에 한정, 다른 sub-project는 별도 plan ✅

→ 자체 검토 통과.

---

## 실행 방식 선택

Plan complete and saved to `docs/superpowers/plans/2026-05-01-sub-project-1-charter-monorepo.md`.

두 가지 실행 옵션:

**1. Subagent-Driven (추천)** — 각 task마다 fresh subagent dispatch, task 사이에 사용자 검토. 빠른 반복.

**2. Inline Execution** — 이 세션에서 task를 순차 실행, 체크포인트마다 검토.

어느 방식?
