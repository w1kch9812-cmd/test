## 9. 검증 기준 (Sub-project 1 완료 판정)

다음 모두 YES일 때 sub-project 1 완료:

### 9.1 결과물 검증

- [ ] 60-80개 파일 모두 작성 + 커밋
- [ ] 모든 파일 ≤500줄 (자동 검증 통과)
- [ ] 모든 docs/{category}/ 에 README 존재
- [ ] 11개 ADR 모두 *컨텍스트 / 결정 / 대안 / 결과* 섹션 존재
- [ ] 9개 컨벤션 .md 모두 도구·룰·예시 포함

### 9.2 SSS 검증 (15 질문 중 가능 항목)

- [ ] (Q1) 새 외부 API 추가를 위한 표준 패턴 문서가 `docs/data-sources/README.md` + `docs/backend/circuit-breaker.md`에 존재
- [ ] (Q3) 에러 형식 SSOT (`docs/conventions/error-format.md`) 존재 + RFC 9457 명시
- [ ] (Q4) 의존성 방향 룰 정의 (`Cargo.toml` 공유 lints + dependency-cruiser 설정)
- [ ] (Q5) 시크릿 스캔 pre-commit + CI에 셋업 (gitleaks)
- [ ] (Q6) 모든 결정에 ADR 존재 (11개)
- [ ] (Q9) 코드 스타일 위반이 commit 단계에서 차단 (lefthook 검증)
- [ ] (Q11) 정보별 SSOT 매트릭스 존재 (`docs/ssot-matrix.md`)
- [ ] (Q13) 도메인 용어 사전 존재 + 위반 검증 룰 정의
- [ ] (Q14) 모든 파일 ≤500줄 자동 검증 셋업

### 9.3 작동 검증

- [ ] `pnpm install` 성공
- [ ] `cargo check` 성공 (Cargo workspace 유효)
- [ ] `pnpm turbo run lint --dry` 성공 (turbo 설정 유효)
- [ ] CI 워크플로우 PR에서 그린
- [ ] pre-commit 훅 실제 동작 (시크릿 커밋 시도 시 차단)
- [ ] AGENTS.md 라우팅 표가 모든 docs/* 위치를 정확히 가리킴 (markdown-link-check 통과)

### 9.4 사용자 검증

- [ ] 사용자가 spec 검토 후 승인
- [ ] 사용자가 결과물(60-80개 파일) 검토 후 승인

---

## 10. 의존성 + 전제 (Prerequisites)

### 10.1 로컬 환경

- Rust 1.83+ (rustup)
- Node.js 20.18+ + pnpm 9.12+
- Git 2.40+
- Docker Desktop (Phase 0 로컬 DB 셋업)

### 10.2 외부

- GitHub 레포 (이미 존재: `gongzzang3` — 또는 새로 만들지 결정 필요)
- AWS 계정 (Phase 1엔 IAM 셋업만, 인프라는 sub-project 8)
- 도메인 (TBD, sub-project 8에서 결정)

### 10.3 사용자 결정 (해결됨)

- ✅ GitHub 레포: `gongzzang3` (이름 변경 가능, 5분 작업)
- ✅ 라이선스: LICENSE 파일 없음 또는 한 줄 (`Copyright © 2026 공짱. All Rights Reserved.`). 외부 deps는 `deny.toml`로 자동 검증
- ✅ 코드 스타일: Biome v2.4 단독
- ✅ 인증 IdP: Zitadel
- [ ] CODEOWNERS의 초기 멤버 (sub-project 1 시작 시 1인부터)

---

## 11. 후속 Sub-projects (의존 그래프)

```
SP1 (헌법+모노레포)  ← 현재
 ↓
 ├─▶ SP2 (DB + Core 도메인)
 │   ↓
 │   ├─▶ SP3 (인증)
 │   ├─▶ SP4 (V-World 통합)
 │   ├─▶ SP9 (ETL 파이프라인)
 │   └─▶ SP5 (첫 API endpoint)
 │       ↓
 │       └─▶ SP6 (첫 프론트엔드)
 │           └─▶ SP11 (검색)
 │
 ├─▶ SP7 (관측성) — 병렬 가능
 ├─▶ SP8 (인프라 IaC)
 └─▶ SP12 (컴플라이언스) — 병렬 가능
```

각 sub-project는 별도 brainstorm → spec → plan → 구현 사이클.

---

## 12. 위험 + 완화

| 위험 | 영향 | 완화 |
|------|------|------|
| ADR 결정 미숙 (특히 Auth) | sub-project 3 시점 큰 재작업 | ADR-0005에 옵션·기준만, 실제 결정은 sub-project 3 brainstorm |
| 컨벤션이 처음부터 너무 엄격 | 개발 속도 저하 | Phase 1엔 *코드 0줄*이라 영향 낮음, Phase 2 시점에 조정 가능 |
| 60-80 파일 한 번에 작성 → 일관성 깨짐 | 1주차 재작업 | Implementation plan에서 작성 *순서* 정의, 의존 그래프 따라 배치 |
| 트리 구조 변경 시 모든 링크 깨짐 | 큰 재작업 | markdown-link-check CI로 즉시 발견 |
| Claude 자동 import (`@AGENTS.md`)가 실패 | 컨텍스트 누락 | Markdown 링크 병행, 모든 도구 읽기 가능하게 |

---

## 13. 자체 검토 체크리스트 (이 spec 자체)

### Placeholder 스캔
- [ ] "TBD" 또는 "TODO" 또는 미완성 섹션 없음
  - § 10.3 GitHub 레포 이름·라이선스·CODEOWNERS 멤버 = 사용자 결정 대기 (TBD 명시)
  - 이 외엔 결정 보류는 § 2.3 에 명시적으로 기록함

### 내부 일관성
- [ ] § 6 결과물 목록과 § 5 트리 구조가 일치 (스폿 체크 통과)
- [ ] § 7 AGENTS.md 룰이 § 5 의 docs 트리와 일치
- [ ] § 8 자동 강제 도구가 § 6.8 결과물과 일치

### 스코프
- [ ] Sub-project 1은 *문서 + 설정*만, 코드 0줄 — § 2.1·2.2 명확히 분리

### 모호성
- [ ] "충분히 명확한가?" — § 9 검증 기준이 객관적 측정 가능 (체크박스 형태)

---

## 14. 다음 단계

이 spec이 사용자 승인되면:

1. **writing-plans 스킬 호출** — 60-80 파일 작성 *순서·의존*을 implementation plan으로 분해
2. **executing-plans 또는 단계별 구현** — plan 따라 파일 생성
3. **검증 + 사용자 검토** — § 9 기준 통과 확인

---

## 15. 참고 자료

- 7 기둥 SSS 정의: 본 spec § 4 + (작성 예정) `docs/sss-charter.md`
- 트리 구조 영감: `daangn/seed-design`, `vercel-labs/claude-managed-agents-starter`
- AGENTS.md 오픈 표준: https://agents.md
- ADR 템플릿: MADR (https://adr.github.io/madr/)
- Conventional Commits: https://www.conventionalcommits.org
- RFC 9457 Problem Details: https://www.rfc-editor.org/rfc/rfc9457
