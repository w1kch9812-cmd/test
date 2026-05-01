# Git + PR 컨벤션

## 1. 브랜치

| 종류 | 형식 | 예시 |
|------|------|------|
| 메인 | `main` | (보호 브랜치, 직접 push 금지) |
| 기능 | `feat/<topic>` | `feat/listing-search-api` |
| 버그 | `fix/<topic>` | `fix/auth-token-leak` |
| 잡일 | `chore/<topic>` | `chore/update-deps` |
| 문서 | `docs/<topic>` | `docs/sub-project-2-spec` |
| 리팩터 | `refactor/<topic>` | `refactor/listing-status-machine` |
| 성능 | `perf/<topic>` | `perf/parcel-search-index` |
| CI | `ci/<topic>` | `ci/add-mutation-test` |

`<topic>` = kebab-case 짧게 (3-5단어).

## 2. 커밋 메시지 (Conventional Commits)

```
<type>(<scope>?): <subject>

<body 선택>

<footer 선택>
```

### type (필수)

- `feat` — 새 기능
- `fix` — 버그
- `chore` — 잡일 (deps, 설정)
- `docs` — 문서만
- `test` — 테스트만
- `refactor` — 리팩터 (동작 변경 X)
- `perf` — 성능 개선
- `ci` — CI 설정
- `build` — 빌드 시스템
- `style` — 포맷 (rustfmt, biome)
- `revert` — 되돌리기

### scope (선택, 권장)

- 도메인: `listing`, `parcel`, `user`, `auth`, ...
- 인프라: `db`, `cache`, `api`, `ui`, ...
- 도구: `monorepo`, `quality`, `ci`, ...

### 예시

```
feat(listing): 매물 검색 API 추가
fix(auth): JWT 만료 시 401 대신 200 반환 버그 수정
chore(deps): Biome 2.4 → 2.5 업그레이드
docs(adr): ADR-0012 검색 엔진 결정 추가
refactor(parcel): PNU 값 객체로 캡슐화
perf(db): listing_geom_gist_idx 추가로 공간 쿼리 100ms → 10ms
```

자동 강제: `lefthook commit-msg` (이미 셋업됨).

## 3. PR 룰

### 크기

- **권장 ≤500줄** (생성/수정 합산)
- 1,000줄 초과 = 분할 검토
- 1,500줄 초과 = 자동 차단 (file-size CI)

### 1 PR = 1 관심사

- ❌ "feat + fix + refactor" 섞기
- ✅ 각각 분리 PR

### PR 제목 형식

커밋 메시지 첫 줄과 동일 형식:
```
feat(listing): 매물 검색 API 추가
```

### PR 본문 (`pull_request_template.md`로 자동)

```markdown
## 변경 요약
1-3 문장

## 동기 (Why)
왜 필요한가

## 변경 범위
- [ ] 단일 sub-project 안에서만
- [ ] sub-project N의 spec/plan과 일치
- [ ] 관련 ADR 작성/갱신

## 테스트
- [ ] 단위 추가/갱신
- [ ] 통합 (필요 시)
- [ ] E2E (필요 시)
- [ ] 수동 시나리오: ...

## SSS 7 기둥 자체 검증
- [ ] 일관성 / [ ] 자동 강제 / [ ] 추적성 / [ ] 안전성
- [ ] 가시성 / [ ] SSOT / [ ] 명확성

## Breaking Change
- [ ] 없음
- [ ] 있음 (마이그레이션 명시):

## 관련 이슈
Closes #
```

## 4. 머지 전략

- **Squash merge** (한 PR = 한 커밋)
- main에 직접 push 금지 (GitHub branch protection)
- 자기 PR 자기 승인 금지 (Phase 2+ 팀 합류 시)
- CI 그린 + 1+ 리뷰어 승인 필수

## 5. 보호 룰 (GitHub branch protection)

- Require pull request before merging
- Require approvals (Phase 2+)
- Require status checks to pass:
  - `lint-format`, `typecheck`, `cargo-check`, `secret-scan`, `link-check`, `file-size`
- Require linear history (squash 강제)
- Require conversation resolution
- Restrict force pushes
- Restrict deletions

## 6. 태그 + 릴리즈

- 태그: `v<major>.<minor>.<patch>` (`v0.1.0`)
- Changesets로 자동 (sub-project 16+)
- 릴리즈 노트 자동 생성 (Conventional Commits 기반)

## 7. 커밋 자체 규칙

- ✅ Atomic — 한 커밋 = 한 논리적 변경
- ✅ Bisect 가능 — 각 커밋이 컴파일·테스트 통과
- ✅ rebase 친화 — fixup/squash 자유
- ❌ "WIP", "tmp", "fix typo" 같은 의미 없는 메시지

## 8. 자동 강제 흐름

| 단계 | 도구 | 차단 |
|------|------|------|
| commit-msg | lefthook + 자체 정규식 | Conventional Commits 형식 |
| pre-commit | lefthook + Biome + rustfmt + gitleaks + file-size | 포맷·시크릿·크기 |
| pre-push | lefthook + cargo check/clippy + typecheck + link check | 깊은 검증 |
| PR | GitHub Actions | 풀스택 (lint/type/test/SAST/SCA) |
| Merge | GitHub branch protection | 위 모두 + 리뷰 |

## 9. 금지

- ❌ `git push --force` to main
- ❌ `git rebase -i main` 후 force push (PR 외)
- ❌ 시크릿 git history에 남기기 (gitleaks가 차단)
- ❌ 큰 바이너리 git 직접 (Git LFS 또는 S3)
- ❌ `--no-verify` 사용 (훅 우회)
