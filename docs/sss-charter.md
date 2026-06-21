# SSS 헌장 (SSS Charter)

> *하이엔드 엔터프라이즈 SSS급* 품질의 정의. **"표면적으로 잘 정리된 폴더 ≠ SSS. 근본적으로 깔끔한 시스템 = SSS."**
> 적용: gongzzang · platform-core · dawneer 3개 repo 공통 ([ADR 0045](./adr/0045-adr-placement-cross-repo-governance.md)).
> v2 (2026-06-22): 제품 우선 종속 + 2계층(안쪽 7 / 바깥쪽 5)으로 재정렬. v1은 제품 우선 전환 전 작성되어 ceremony를 포함했음.

---

## 0. 최상위 규칙 — 제품 우선 (SSS보다 우선)

이 프로젝트는 **아직 런칭 전(유저 0)**이다. SSS는 **"그렇게 *될 수 있게* 설계하라"는 방향**이지,
"유저도 없는데 검사·거버넌스를 미리 다 지으라"는 뜻이 **아니다.** 충돌하면 [AGENTS.md ✱ 제품 우선
원칙](../AGENTS.md)이 이긴다. ([ADR 0044](./adr/0044-bazel-transition-reconciliation.md))

- **이 헌장은 *방향 문서*다. *메타 머신*이 아니다.** 레지스트리/투영/래칫/증거번들/준비게이트/자기감사
  자동화를 만들지 않는다.
- **SSS는 *벌어들이는* 것이지 *미리 바르는* 것이 아니다.** 기둥은 새 기능을 만들 때 그 기능에 적용해서
  채운다. 빈 코드에 기둥 문서·검사를 늘리는 것 자체가 ceremony다.
- **진척은 유저 가시 기능으로 측정한다 — 문서·검사·기둥 통과 수가 아니라.**
- 각 기둥의 **"런칭 전 최소선"**만 지금 지킨다. 그 이상(대시보드·SLO·외부감사 등)은 수요가 당길 때
  (실제 트래픽·실제 사고) 올린다.

---

## 1. 두 계층

| 계층 | 질문 | 기둥 |
|---|---|---|
| **A. 안쪽 — 어떻게 짓는가** | "우리가 잘 짓고 있나?" | 일관성 · 자동강제 · 추적성 · 안전성 · 가시성 · SSOT · 명확성 |
| **B. 바깥쪽 — 유저가 무엇을 받는가** | "유저에게 진짜 좋은가?" | 데이터 정확성·신뢰 · 신뢰성 · 보안·프라이버시 · 성능 · 접근성·UX |

안쪽 7기둥을 100% 지켜도 **느리고·데이터 틀리고·털리는** 제품이 나올 수 있다. SSS = 안쪽 규율 위에
바깥쪽 결과를 얹은 것. **둘 다 있어야 SSS.**

---

## 계층 A — 안쪽 (7기둥)

각 기둥 = 표준(구체) + "어떻게 아나(신호)". (구체 표는 진짜 표준이라 보존. 도구 일부는 *방향*이며,
실제 도입은 트래픽이 생긴 뒤.)

### A-1. 일관성 — 같은 일은 같은 방식으로, 예외 0

| 영역 | 표준 |
|------|------|
| 외부 API 호출 | Circuit Breaker + Retry + Timeout + Audit log |
| 에러 응답 | RFC 9457 Problem Details + `correlationId` |
| ID | ULID + 도메인 prefix (`usr_`/`lst_`/`prc_`) |
| 시간 | UTC `TIMESTAMPTZ` |
| 좌표 | EPSG:4326 명시 |
| API 응답 | camelCase |
| 한국어 UI | 해요체 |

*신호:* 두 곳에서 같은 문제를 다르게 풀고 있지 않다.

### A-2. 자동 강제 — 사람이 아니라 시스템이 차단

현행(작동 중): rustfmt·Biome·clippy `-D warnings`·gitleaks·cargo-deny·파일크기 hook·생성물 드리프트 가드.
*방향(수요 시):* OpenAPI diff, 커버리지 게이트, 이미지 서명/SBOM.
**규칙:** 새 검사는 *"실패 시 어떤 진짜 버그/사고를 막나?"*에 한 문장으로 답할 때만 추가. 못 하면 만들지 않는다.

### A-3. 추적성 — 변경·결정·데이터를 재구성 가능

요청=`X-Correlation-Id`, 외부호출=audit log+raw lineage, DB=`created_by/updated_by/version`+audit,
결정=ADR, 이벤트=Outbox, 배포=Git SHA(+서명/SBOM은 방향). 수용한 위험엔 사유+추적 링크.

### A-4. 안전성 — 런타임 에러를 컴파일/스키마에서 차단

Rust ownership + `unsafe` 금지, TS strict, 값 객체 newtype(`Pnu`·`Money`·`BusinessNumber`),
enum+exhaustive match, sqlx 컴파일타임 쿼리, 멱등성 키(쓰기), Outbox 트랜잭션 일관성, 입력 검증,
민감필드 암호화. 생성코드는 타입드 SSOT에서만. *(공격자 방어는 안전성이 아니라 B-3.)*

### A-5. 가시성 — 서비스 상태 인지

*최소선:* 구조화 로그 + health/ready + 핵심 메트릭. *방향:* RED/USE 메트릭, 분산추적, 에러추적,
SLO/Error-Budget (트래픽 생긴 뒤 실측해서 정의 — 미리 대시보드 안 지음).

### A-6. SSOT — 한 정보 = 한 곳

사본은 사본임을 명시. 인프라=Pulumi만, TS타입=OpenAPI 자동만, DB스키마=마이그레이션만,
도메인 용어=[glossary.md](./glossary.md)만. 생성물엔 "수정금지+재생성 명령" 마커 + 드리프트 가드.
상세: [ssot-matrix.md](./ssot-matrix.md).

### A-7. 명확성 — 추측 없이 의도 이해

컨벤션 SSOT: [conventions/](./conventions/README.md) (rust·typescript·sql·naming-and-ids·error-format·
ui-writing-korean·testing·git-and-pr·comments). *신호:* 처음 보는 사람이 이름만 보고 역할을 안다.

---

## 계층 B — 바깥쪽 (신규 5기둥)

유저가 실제로 받는 것. 각 기둥 = 정의 / 좋다는 건 / 어떻게 아나 / 런칭 전 최소선.

### B-1. 데이터 정확성·신뢰 (Data Integrity & Trust) ★ 이 제품의 심장

부동산 데이터 플랫폼은 **데이터가 맞는 게 곧 제품**이다. 틀린 공시지가·잘못된 필지 경계 하나가 신뢰를 깬다.
완전함보다 **정확함**이 우선. 모르면 "모름"이라 말하고 추측을 사실처럼 보이지 않게.

- **좋다는 건:** 모든 값에 출처·수집시각·SRID·라이선스 추적, 원본(raw lineage) 보존, 신선도 인지.
- **어떻게 아나:** 임의 매물/필지의 "이 값 어디서 언제 왔나"를 끝까지 되짚을 수 있다. 오래된 데이터가 최신인 척 안 함.
- **런칭 전 최소선:** Catalog 원본=Platform Core lineage store, 자체 외부호출=승인된 archive/lineage 계약
  ([AGENTS.md §3·§8](../AGENTS.md)). SRID 항상 명시. "맞는 것만 보여주고, 불확실하면 숨기거나 표시."

### B-2. 신뢰성 (Reliability / SLO)

"상태를 안다"(가시성)를 넘어 **"안 죽는다를 보장한다"**.

- **좋다는 건:** 핵심 플로우에 SLO+에러버짓, 외부 의존성 장애 시 graceful degradation(캐시·부분응답·명확한 에러).
- **어떻게 아나:** 외부 API 하나 죽여도 사이트가 통째로 안 죽고 유저는 "원인+대응" 메시지를 본다.
- **런칭 전 최소선:** 모든 외부 호출에 Circuit Breaker/Retry/Timeout, health/ready. SLO 수치는 트래픽 후 측정.

### B-3. 보안·프라이버시 (Security & Privacy)

컴파일 안전성과 **별개** — 공격자 관점 방어 + 개인정보(PIPA).

- **좋다는 건:** 위협 모델, 권한(authz) 정합성, CSP/XSS/CSRF/rate-limit, PII 로그/스팬/이벤트 금지, 시크릿 코드 밖,
  의존성 취약점은 *추적되거나 고쳐짐*(조용히 무시 ❌).
- **어떻게 아나:** "이 라우트 누가 호출 가능?"에 정책(traffic-auth-policy SSOT)으로 답하고, 취약점이 사유·기한 없이
  방치돼 있지 않다.
- **런칭 전 최소선:** gitleaks·cargo-deny 그린(수용 취약점은 deny.toml에 사유+TODO). 권한정책=registry→생성코드 단일출처.
  PII 로그 0. **현재 미해결:** rustls-webpki 취약점(aws-sdk legacy TLS) → 프로덕션 전 처리.

### B-4. 성능 (Performance — 유저 체감)

빠름은 기능. 내부 효율이 아니라 **유저가 느끼는 속도**.

- **좋다는 건:** 핵심 화면 지연 예산(LCP/INP/CLS, §10.2 Core Web Vitals), 번들 예산, N+1·불필요 왕복 없음.
- **어떻게 아나:** 실제 기기/네트워크에서 핵심 플로우가 예산 안.
- **런칭 전 최소선:** 명백한 성능 자살(무한루프·페이지당 수백 요청·거대 번들)만 차단. 정밀 SLO는 실측 후.

### B-5. 접근성·UX 품질 (Accessibility & UX Quality)

유저가 실제로 받는 경험.

- **좋다는 건:** WCAG 2.2 AA, 키보드-only 핵심 플로우, 해요체·능동·긍정 UX 라이팅, 다크패턴 없음, 유저 노출 문자열=typed i18n.
- **어떻게 아나:** 키보드만으로 핵심 작업 가능, axe 위반 0(핵심 화면), 거절 선택지 항상 존재.
- **런칭 전 최소선:** 아이콘버튼 aria-label, div→button, label 연결, 진입 즉시 바텀시트/이탈방지 인터럽트 금지.

---

## 2. SSS는 어떻게 벌어들이나

> 기둥을 *문서로* 더 쓰는 게 SSS가 아니다. **기능 하나를 만들 때 12기둥을 그 기능에 적용**하면 그 기능이 SSS가 되고,
> 기능마다 쌓이면 제품이 SSS가 된다.

새 기능의 "완료" 정의 (런칭 전):

1. 유저가 실제로 뭔가 할 수 있다 (안쪽 7기둥 + 동작).
2. 보여주는 데이터가 출처·신선도 추적된다 (B-1).
3. 외부 의존성이 죽어도 기능이 통째로 안 죽는다 (B-2).
4. 권한이 맞고 PII가 안 샌다 (B-3).
5. 핵심 기기에서 느리지 않다 (B-4).
6. 키보드로 되고 다크패턴이 없다 (B-5).

6개에 한 문장씩 답하면 그 기능은 SSS. 못 답하면 아직 아니다.

---

## 3. 이 헌장이 *아닌* 것 (anti-ceremony 가드)

- 점수 매트릭스 ❌ · 기둥별 준수 레지스트리 ❌ · 자기검증 게이트 ❌ · 분기별 자체 SSS 감사 ❌ · PR 7기둥 점검
  템플릿 ❌ (전부 메타 머신 / 자기 진척 측정, 금지).
- "기둥 N개 통과"를 진척 지표로 삼지 않는다. 진척 = 유저가 할 수 있게 된 일.
- 새 검사/문서는 "실패 시 어떤 진짜 사고를 막나?"에 답할 때만. 못 하면 만들지 않고, 이미 있으면 삭제.
- 외부 인증(ISMS-P)·외부 펜테스트·SLO 대시보드는 *프로덕션 트래픽이 생긴 뒤*의 일이지 런칭 전 작업이 아니다.

---

## 참조

- [AGENTS.md](../AGENTS.md) — ✱ 제품 우선 원칙(최상위) · §0 7기둥 · §10 패널 SSS 축(패널 한정 상세, B기둥의 구현체)
- [ADR 0044](./adr/0044-bazel-transition-reconciliation.md) — PowerShell 제거 / 제품 우선 전환
- [ssot-matrix.md](./ssot-matrix.md) · [conventions/](./conventions/README.md) · [glossary.md](./glossary.md)
- 표준: WCAG 2.2 AA · OWASP ASVS · NIST SSDF SP 800-218 · Core Web Vitals · 개인정보보호법(PIPA)
