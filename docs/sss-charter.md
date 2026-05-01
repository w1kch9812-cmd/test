# SSS 헌법 — 7 기둥

> 이 프로젝트(공짱)가 만족시켜야 할 *하이엔드 엔터프라이즈 SSS급* 품질의 정의.
> **"표면적으로 잘 정리된 폴더 ≠ SSS"**. **"근본적으로 깔끔한 시스템 = SSS"**.

---

## 0. SSS의 진짜 정의

| 잘못된 SSS (표면적) | 진짜 SSS (근본적) |
|---------|---------|
| 폴더 잘 나눔 | 임시방편 0개 |
| README 잘 씀 | 산업 표준 패턴 + 일관성 |
| 거대 단일 SSOT 파일 (1,400줄) | 폴더 단위 SSOT (각 ≤500줄) |
| "팀이 알아서 잘 지킴" | 시스템이 자동 차단 |
| 결정 사후 기록 | 결정 *전에* ADR 작성 |
| `TEMP`, `HACK`, `XXX` | 임시방편 코드 0개 |

진짜 SSS = 다음 7 기둥을 *측정 가능하게* 만족.

---

## 1. 일관성 (Consistency)

**모든 같은 종류의 일은 같은 방식으로**. 예외 0.

| 영역 | 일관성 기준 |
|------|---------|
| 외부 API 호출 | 100% Circuit Breaker + Retry + Timeout + Audit log |
| 에러 응답 | 100% RFC 9457 Problem Details |
| ID 생성 | 100% ULID + 도메인 prefix (`usr_`, `lst_`, `prc_`) |
| 시간 저장 | 100% UTC TIMESTAMPTZ |
| 좌표 저장 | 100% EPSG:4326 |
| 데이터 변경 | 100% Audit log + version bump |
| API 응답 | 100% camelCase + correlationId 포함 |
| 한국어 UI | 100% 해요체 |

→ 한 군데라도 "예외 — 이건 그냥 …" 하면 SSS 깨짐.

## 2. 자동 강제 (Enforcement)

**규칙은 사람이 지키는 게 아니라 시스템이 차단**.

| 규칙 | 강제 도구 | 단계 |
|------|---------|------|
| 코드 스타일 | rustfmt + Biome | pre-commit (lefthook) |
| Lint | clippy pedantic + Biome | pre-commit + CI |
| 의존성 방향 | dependency-cruiser + cargo-arch | CI 차단 |
| 파일 크기 ≤500/1500 | 자체 hook + CI | pre-commit + CI |
| 시크릿 스캔 | gitleaks | pre-commit + CI |
| 공급망 보안 | cargo-audit + cargo-deny + Snyk | CI |
| API 변경 | OpenAPI diff (Spectral) | CI |
| 테스트 커버리지 | cargo-tarpaulin + Vitest | CI |
| 커밋 메시지 | commitlint | commit-msg hook |
| 이미지 서명 | Cosign + Sigstore | CD |
| SBOM | syft (생성) + Grype (스캔) | CD |

→ "팀이 깜빡할 수 있는 모든 규칙 = 자동화".

## 3. 추적성 (Traceability)

**임의 시점, 임의 사용자, 임의 데이터 변경을 *완전히* 재구성 가능**.

| 요소 | 추적 방식 |
|------|---------|
| HTTP 요청 | `X-Correlation-Id` (전 호출 체인 통과) |
| 외부 API 호출 | audit log + raw_response JSONB 보존 |
| DB 변경 | `created_by`, `updated_by`, `version` 컬럼 + 별도 audit table |
| 배포 | Git SHA + Cosign 서명 + SBOM |
| 의존성 | SBOM (syft) |
| 결정 | ADR (`docs/adr/NNNN-*.md`) |
| 비즈니스 이벤트 | 도메인 이벤트 + Outbox 패턴 |
| 사용자 행동 | 분석 이벤트 (PIPA 준수 마스킹) |
| 인프라 변경 | Pulumi state + Git history |

→ "왜 이렇게 됐어?" 모든 질문에 답 가능해야 함.

## 4. 안전성 (Safety)

**런타임에 깨질 일이 컴파일/스키마 단계에서 차단**.

| 메커니즘 | 도구 |
|---------|------|
| 메모리 안전 | Rust ownership + `unsafe_code = forbid` |
| 타입 안전 | Rust + TS strict + OpenAPI 자동 동기화 |
| 값 안전 | Newtype 값 객체 (`Pnu`, `Money`, `BusinessNumber`) |
| 상태 안전 | enum + exhaustive match |
| 동시성 안전 | Tokio + Optimistic Locking |
| 멱등성 | Idempotency-Key 헤더 모든 쓰기 |
| 회복성 | Retry + Circuit Breaker + Fallback |
| 트랜잭션 | Outbox 패턴 (DB ↔ 메시징 일관성) |
| 입력 검증 | garde/validator + zod |
| 비밀번호 | Zitadel 위임 (자체 관리 X) |
| 암호화 | AES-256-GCM at-rest, TLS 1.3 in-transit |
| Field-level 암호화 | 사업자번호, 주민번호 등 민감 필드 |

## 5. 가시성 (Observability)

**서비스 상태 실시간 인지**.

| 신호 | 도구 |
|------|------|
| 메트릭 (RED + USE) | Prometheus → Grafana |
| 구조화 로그 | tracing + Loki + Vector |
| 분산 추적 | OpenTelemetry → Tempo |
| 에러 | Sentry (셀프호스트 → SaaS) |
| 사용자 경험 (RUM) | Sentry RUM |
| 합성 모니터링 | k6 + GitHub Actions cron |
| SLO 정의 | Grafana SLO + 알림 |
| Error Budget | Grafana 자동 계산 |
| On-call | Grafana OnCall (OSS) → PagerDuty (Phase 3+) |
| DORA 메트릭 | 자체 (배포 빈도/MTTR/변경 실패율/리드 타임) |
| 비용 관측 | AWS Cost Explorer + 알림 |

## 6. 단일 출처 (SSOT)

**한 정보 = 한 곳에만**. 사본은 명시적으로 그것이 사본임을 표시.

상세 매트릭스: → [ssot-matrix.md](./ssot-matrix.md)

핵심 규칙:
- AWS 콘솔 직접 변경 = 금지 (Pulumi만)
- TS 타입 수동 작성 = 금지 (OpenAPI 자동만)
- DB 스키마 수동 변경 = 금지 (마이그레이션만)
- 도메인 용어는 [glossary.md](./glossary.md) 만 SSOT

## 7. 명확성 (Clarity / Convention)

**처음 보는 사람이 *추측 없이* 의도 이해**.

| 영역 | 컨벤션 SSOT |
|------|---------|
| Rust | [conventions/rust.md](./conventions/rust.md) |
| TypeScript | [conventions/typescript.md](./conventions/typescript.md) |
| SQL | [conventions/sql.md](./conventions/sql.md) |
| 네이밍 + ID | [conventions/naming-and-ids.md](./conventions/naming-and-ids.md) |
| 에러 형식 | [conventions/error-format.md](./conventions/error-format.md) |
| UI 한국어 | [conventions/ui-writing-korean.md](./conventions/ui-writing-korean.md) |
| 테스트 | [conventions/testing.md](./conventions/testing.md) |
| Git/PR | [conventions/git-and-pr.md](./conventions/git-and-pr.md) |
| 주석 | [conventions/comments.md](./conventions/comments.md) |

---

## 15 검증 질문 (자체 평가 체크리스트)

이 15개 모두 **YES**여야 진짜 SSS:

### 일관성·자동 강제
1. □ 새 외부 API 5분 안에 표준 패턴(CB+Retry+Audit+OTel)으로 추가 가능?
2. □ 새 endpoint를 만들면 OpenAPI + TS 타입이 자동 갱신?
3. □ API 응답 형식이 100% RFC 9457 일관?

### 추적성
4. □ 임의 사용자 ID로 모든 활동을 시간순 재구성 가능?
5. □ 임의 배포가 어떤 코드/의존성으로 만들어졌는지 SBOM으로 증명?
6. □ 모든 기술/도메인 결정에 ADR 존재?

### 안전성·자동 차단
7. □ 의존성 방향 위반 시 CI에서 빌드 실패?
8. □ 비밀(.env, API 키)이 git에 들어가면 자동 차단?
9. □ 코드 스타일 위반이 commit 단계에서 차단?

### 가시성
10. □ 새벽 3시 장애가 자동 알림 + 런북 + 대응자에게 page?

### SSOT
11. □ 같은 정보가 두 곳에 있으면 즉시 어느 게 SSOT인지 답 가능?
12. □ AWS 콘솔에 직접 만든 리소스가 0개?

### 명확성
13. □ 도메인 용어 사전 위반이 CI 차단?
14. □ 모든 파일 ≤500줄?

### 데이터 거버넌스
15. □ 외부 공공 API 응답을 1년 후에도 raw 그대로 재현 가능?

→ **15/15 = 진짜 SSS**. 14/15 이하면 미달.

## SSS 단계 (시간 흐름)

```
[0단계: SSS 청사진]      ← 현재 (sub-project 1 진행 중)
   ↓ sub-project 1 완료
[1단계: SSS 기반]        ← 70 파일 + 자동 강제 작동, 코드 0줄
   ↓ sub-project 2-9 완료
[2단계: SSS 핵심]        ← 첫 화면 작동, 7 기둥 *부분* 검증
   ↓ Phase 3 출시 + 운영 6개월
[3단계: SSS 운영]        ← 15 검증 12/15 이상 통과
   ↓ ISMS-P + 외부 펜테스트
[4단계: SSS 인증]        ← 15/15 + 외부 검증 + 컴플라이언스 인증
```

## 자체 평가 주기

- 매 PR: 7 기둥 자체 점검 (PR 템플릿)
- 매 sub-project 완료: 15 검증 질문 결과 spec에 기록
- 분기별: 자체 SSS 감사 (`docs/governance/`에 결과)
- 연 1회: 외부 감사 (Phase 3+)
