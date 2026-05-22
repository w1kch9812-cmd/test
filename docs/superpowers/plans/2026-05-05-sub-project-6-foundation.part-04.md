## 참고

- Spec: `docs/superpowers/specs/2026-05-05-sub-project-6-foundation-design.md`
- Plan: `docs/superpowers/plans/2026-05-05-sub-project-6-foundation.md`
- AGENTS.md: 프로젝트 헌법 (한국어 컨벤션 / SSS 7기둥 / SSOT 매트릭스)
````

#### Step 4.11: roadmap 갱신

- [ ] **Step**: Update `docs/superpowers/roadmap.md`

다음 변경 적용:

**Header:**
```markdown
> **갱신일**: 2026-05-05 (SP6-foundation 종료 직후)
> **현재 main**: `<T4 commit hash>` (SP6-foundation — frontend 인프라)
```

**완료 표 (SP6-foundation 행 추가):**
```markdown
| **6-foundation** | Frontend 인프라 (Next.js 16 + shadcn + tokens + i18n + UX 패턴) | apps/web (Next.js 16 + React 19 + Tailwind 4) + packages/ui (shadcn primitives + Pretendard tokens, swap-able) + packages/api-types (utoipa → TS) + 한국어 helper + error/not-found/loading + ky API client + TanStack Query + proxy skeleton + instrumentation.ts (Sentry 자리) + Vitest + Playwright + @axe-core/playwright (WCAG 2.1 AA) + size-limit (bundle < 200KB) + .github/workflows/frontend.yml. SSS 7기둥 모두 ◎ | ✅ |
```

**누적 통계:**
- 33 crate (Rust 그대로) + JS workspace 추가
- ~1278 tests + ~7 unit + 3 e2e (Playwright) — frontend
- 5 CI workflow (frontend 추가)

**SP6 시리즈 갱신:**
```markdown
### SP6 시리즈 (Frontend)
- ✅ SP6-foundation: 인프라 (2026-05-05) — Next.js 16 + shadcn + tokens + UX
- 미착수 SP6-i: auth flow + 화면 (login/signup/profile + OIDC + RBAC, 2-3일)
- 미착수 SP6-ii: 매물 검색 + Naver Maps (2-3일)
- 미착수 SP6-iii: 매물 상세 + 북마크 (1-2일)
- 미착수 SP6-iv: 매물 등록 broker 전용 (2일)
- 미착수 SP6-v: 알림 (1일)
```

#### Step 4.12: Workspace 전체 검증

- [ ] **Step**: 모든 검증

```bash
pnpm install
pnpm typecheck
pnpm lint
pnpm test
pnpm build
pnpm --filter=@gongzzang/web test:bundle
pnpm --filter=@gongzzang/web exec playwright install chromium --with-deps
pnpm --filter=@gongzzang/web test:e2e
```

Expected: 모두 pass.

#### Step 4.13: T4 commit + push

- [ ] **Step**: T4 commit

```bash
git add apps/web docs/frontend docs/superpowers/roadmap.md \
        .github/workflows/frontend.yml

git commit -m "$(cat <<'EOF'
feat(sp6-foundation-t4): smoke + frontend CI + a11y + bundle + docs + roadmap

T4 of SP6-foundation (마지막):
- apps/web/app/page.tsx — /api/proxy/healthz smoke 호출 + Card UI (한국어 해요체)
- apps/web/playwright.config.ts — chromium project + webServer (pnpm dev)
- apps/web/tests/e2e/healthz.spec.ts — smoke (200 또는 502 — frontend pipeline 정상 의미)
- apps/web/tests/e2e/a11y.spec.ts — @axe-core/playwright (WCAG 2.1 AA, critical/serious 0)
- apps/web/.size-limit.json — production bundle < 200KB JS gzipped + < 60KB framework
- .github/workflows/frontend.yml — pnpm + Node 20 + lint/typecheck/test/build/bundle/e2e+a11y
  - paths filter (apps/web + packages/ui + packages/api-types 변경 시만)
  - Playwright report artifact upload (on failure)
- docs/frontend/README.md — 운영 SSOT (시작법 / 디렉토리 / 한국어 컨벤션 / swap path / 진화)
- docs/superpowers/roadmap.md — SP6-foundation ✅ + SP6-i ~ v 자리 명시 + 누적 통계

SP6 시리즈 첫 sub-project 완료. SP6-i ~ v 가 이 foundation 위에서 빠른 빌드.

Closing: (없음 — 첫 frontend sub-project)
미흡수 (SP6-i ~ v 또는 SP7-i): auth flow / 매물 검색 / Sentry 통합
EOF
)"

git push origin main
```

**사용자 체크포인트**: T4 commit + push 후 5 CI workflow 그린 확인 + 다음 sub-project 결정.

---

## 위험 요소

- **Next.js 16 stable 여부**: 작업 시점 기준 16 가 안정 버전 아니면 15.x 로 fallback (package.json 만 수정)
- **Tailwind 4 PostCSS plugin**: 현재 alpha — 추후 `tailwindcss/postcss` 명시 변경 가능
- **next-intl App Router**: plugin 위치 (`./i18n.ts`) Next.js 버전마다 다름
- **shadcn CLI 미사용**: 본 plan 은 코드 직접 작성 — shadcn 의 자동 cn 패턴 / 의존성 미스 가능
- **Backend 미동행 e2e**: smoke 테스트가 502 도 OK 처리 — 미래 SP6-i 후 진짜 backend 호출 검증 필요
- **utoipa 미통합**: api-types/generated/schema.ts 가 placeholder — 실 utoipa 통합은 별도 sub-project (또는 SP6-i)

## 추정

- T1: 1 commit, 2-3시간 (monorepo + Next.js setup + 의존성 설치)
- T2: 1 commit, 4-5시간 (shadcn 6 컴포넌트 + tokens + i18n + 한국어 helper + UX patterns)
- T3: 1 commit, 3-4시간 (API client + TanStack Query + proxy + instrumentation + Zustand)
- T4: 1 commit, 3-4시간 (smoke + Playwright + axe + size-limit + workflow + docs)

총: 3-4일 (각 task 끝 사용자 체크포인트 포함)

## 완료 후 다음

- SP6-i: auth flow + 화면 brainstorming → spec → plan → impl
- 또는 SP4-iii-b 데이터 풍부화
- 또는 SP7-i Sentry (frontend instrumentation 활용)

---

## 자가 평가 — Spec coverage

Spec 의 모든 § 가 plan task 로 covered:

- § 1 배경 — context only
- § 2 목표 11개 → T1 (monorepo+Next.js), T2 (shadcn+tokens+i18n+UX), T3 (API client+proxy+Sentry자리), T4 (CI+a11y+bundle+smoke)
- § 3 SSS 7기둥 — T1-T4 누적
- § 4 Scope 포함 — T1-T4 모두 cover. 미포함 (auth/Naver Maps/PWA) 명시
- § 5 아키텍처 (큰 그림 + 호출 흐름 + swap path) → T1-T4 + docs/frontend/README
- § 6 Stack 18개 → 의존성 (T1-T3) + 도구 (T4)
- § 7 디렉토리 구조 → T1-T4 파일 그대로
- § 8 작업 단위 T1-T4 → 본 plan 의 Phase A-D
- § 9 검증 / 테스트 전략 → T2 unit + T4 e2e + a11y + bundle + workflow
- § 10 Migration / Swap path → docs/frontend/README + tokens 분리
- § 11 Follow-up → roadmap 갱신
- § 12 추정 → 본 plan 추정
- § 13 SSS 자가 평가 → T1-T4 누적
- § 14 핵심 결정 16개 → 모두 plan 에 반영

**모든 § 가 task 로 covered.** ✅
