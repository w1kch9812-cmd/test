# ADR 0028 — Supply-chain SHA pin policy + manifest backup cleanup cron

| | |
|---|---|
| 작성일 | 2026-05-11 |
| 상태 | Accepted |
| 선행 | [ADR 0021](./0021-static-vector-tile-decomposition.md), [ADR 0024](./0024-etl-cancel-protocol-immediate-abort.md), [ADR 0027](./0027-admin-complex-layer-source-deferred.md) |

> **Handover note**: the supply-chain SHA pin policy remains active. The manifest
> backup cleanup cron portion is superseded by [ADR 0036](./0036-static-vector-tile-runtime-contract.md)
> and platform-core ADR 0004. Gongzzang no longer promotes, rolls back, or cleans
> up `gold/manifest.json`; platform-core Catalog owns that lifecycle.

## 결정

본 ADR 은 Codex Round 5 audit 가 발견한 *enterprise-grade 운영 통제* 미박제 영역 중
**supply-chain SHA pin** 과 **manifest backup cleanup cron** 두 가지를 박제.

### 1. Supply-chain SHA pin policy

모든 GitHub Actions 사용은 *commit SHA pin* 으로 박제. `@v4` 같은 tag pin 은
force-push 가능 → supply-chain attack vector (NIST SSDF 의 PO.5 / OWASP supply-chain
best practice 의 *Pin to immutable identifier*).

```yaml
# 권장 (SHA pin + release tag 주석):
- uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683  # v4.2.2

# 금지 (tag pin):
- uses: actions/checkout@v4
```

**예외 — `dtolnay/rust-toolchain`**: 본 action 은 *release tag* 가 없음
(`@stable` / `@nightly` / `@1.88` 가 *channel* 형식 — 의도된 floating ref).
SHA pin + `# @<channel>` 주석 형식 허용 (e.g. `@21dc36...  # @stable`):

```yaml
# 허용 (channel-based action 의 SHA pin + channel 주석):
- uses: dtolnay/rust-toolchain@21dc36fb71dd22e3317045c0c31a3f4249868b17  # @stable
```

본 예외는 *action 의 release model 자체* 가 channel-based 라 적용. 다른 모든 action
은 `# vX.Y.Z` 패턴 강제.

**자동 갱신**: `.github/dependabot.yml` 의 `package-ecosystem: github-actions` 가
주간으로 새 SHA PR 자동 생성 (tag pin 회귀 0 보장). 운영자는 weekly review 에서
검토 후 merge.

### 2. Manifest backup cleanup cron

Historical decision. This cleanup path is now disabled in Gongzzang after ADR 0036.
`gold/manifest.<previous_version>.json` retention and cleanup belongs to platform-core
Catalog. The Gongzzang workflow remains present only as a disabled audit stub, and
`etl-base-layer cleanup-manifest-backups` exits with the platform-core handover notice.

## 컨텍스트 — Codex Round 5 audit

| Finding | 위치 |
|---|---|
| Action SHA pin 부재 | `.github/workflows/sp9-base-layer-etl.yml:71,108,236,240` 등 |
| Backup cleanup 미구현 | `docs/sp9/sslo-runbook.md:242` "TODO ADR 0028" |

## 검토한 옵션

### A — 즉시 모든 actions SHA pin (수동)

- 장점: 즉시 SSS-grade supply-chain
- 단점:
  - 잘못된 SHA pin = workflow 실행 즉시 실패 (offline 환경에서 SHA 검증 못 함)
  - 매 갱신마다 manual lookup 필요 — 운영 부담
- **거부**: dependabot 가 자동화 — manual 변환은 일회성 작업이 SSS 가 아니라
  지속적 운영이 SSS.

### B — Dependabot 활성 + manual SHA pin 변환 (별도 PR)
- 장점: 자동 갱신 + 안전 (dependabot 가 검증된 SHA 만 제안)
- 단점: 본 commit 에서 즉시 SHA pin 안 됨 — 별도 PR 사이클
- **채택**: dependabot 가 *지속적* SSS 확보. 1회성 SHA pin 변환은 dependabot 가
  제안하는 PR 로 점진 적용.

### C — Renovate Bot (Dependabot 대체)
- 장점: 더 강력한 grouping / scheduling
- 단점: 외부 GitHub App 추가 install — 권한 model 변경
- **거부**: GitHub Actions 의 native dependabot 가 SP9 규모에 충분.

## 영향

### 신규
- `.github/dependabot.yml` — 4 ecosystems (github-actions / cargo / pip / npm) 주간 갱신
- `docs/adr/0028-supply-chain-sha-pin-and-cleanup-cron.md` (본 파일)

### 후속 sprint (본 ADR 박제만, 구현은 별도 PR)
- 모든 workflow yml 의 actions 를 dependabot 의 첫 PR 로 SHA pin 전환
- manifest cleanup 후속 구현은 platform-core Catalog 로 이관

### 변경 없음
- 기존 workflow yml — dependabot PR 이 자동 갱신할 예정
- cargo-deny / pip-audit / gitleaks gate — 이미 박제됨 (Round 3 P0-5)
- runbook § 6 (backup retention) — Gongzzang 절차는 ADR 0036 handover 로 superseded

## SSS 7기둥 매핑

| 기둥 | Tag pin (이전) | SHA pin + dependabot (본 결정) |
|---|---|---|
| 일관성 | ❌ — action 마다 다른 pin 형식 | ✅ — 모든 action SHA pin |
| 자동강제 | ❌ — manual lookup | ✅ — dependabot 주간 PR |
| 추적성 | △ — tag → 어떤 SHA 인지 모름 | ✅ — SHA 자체 박제 |
| 안전성 | ❌ — force-push attack 가능 | ✅ — immutable SHA |
| 가시성 | △ | ✅ — dependabot PR 이 운영자 view |
| SSOT | △ | ✅ — SHA 가 단일 식별자 |
| 명확성 | △ | ✅ — `@SHA  # tag` 주석 패턴 |

## 재검토 트리거

- dependabot 의 GitHub Actions ecosystem 갱신이 *SHA 단위* 가 아닌 *tag 단위* 만
  지원하는 것으로 확인되면 (현재는 SHA 갱신 지원 — `dependabot.yml` 의
  `enable-beta-ecosystems` 또는 일부 옵션) → Renovate 로 전환 검토
- workflow yml 변경 빈도가 분 단위로 단축 — manual review bandwidth 초과 시 자동 merge
  policy 박제
- supply-chain incident (SP9 또는 organization 단위) 발생 시 본 정책 재평가

## 참고

- NIST SSDF (Secure Software Development Framework): https://csrc.nist.gov/Projects/ssdf
- OWASP supply-chain: https://owasp.org/www-project-software-component-verification-standard/
- Dependabot config docs: https://docs.github.com/en/code-security/dependabot/dependabot-version-updates/configuration-options-for-the-dependabot.yml-file
- ADR 0027 (admin/complex SSS-DEBT 박제 패턴 — 본 ADR 도 동일 *trick 인정 + 절차 박제* path)
