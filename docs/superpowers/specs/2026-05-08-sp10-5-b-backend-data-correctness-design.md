# SP10.5-B: Backend Data Correctness — SSS-grade PIPA-compliant Hardening

| | |
|---|---|
| 작성일 | 2026-05-08 |
| 상태 | Draft |
| 결정 ADR | 0024 (Allowlist), 0025 (Two-tier vault), 0026 (TTL), 0027 (RBAC + audit), 0028 (AWS KMS Pulumi) |
| 목적 | 패널 backend data correctness 를 SSS-grade PIPA-compliant 로 hardening |
| 추정 | 5~7 영업일 |

---

## 1. 목표

1. **PII 기본 차단**: 외부 API 응답에 포함될 수 있는 개인정보(소유자명, 연락처 등)를 수집 시점에 Allowlist 기반으로 자동 폐기한다.
2. **이중 저장 분리**: 정제된 데이터는 기존 parcel_external_data (Tier 1), 원문 full-raw는 KMS 암호화된 parcel_external_data_pii_vault (Tier 2)에 분리 보관한다.
3. **PIPA 4원칙 강제**: 수집 목적 한정·최소 수집·보유 기간·파기를 시스템이 자동 강제한다.
4. **Vault 접근 RBAC + 감사**: 원문 조회는 ZITADEL admin role + purpose code + ticket_id 요건을 충족해야만 허용하며 모든 접근이 기록된다.
5. **Building reader 실 연결**: NoOpBuildingRegisterReader 교체로 data.go.kr 건축물대장 실 데이터 수신을 활성화한다.
6. **통합 테스트 실 router 전환**: 핸들러 로직 재구현 방식에서 진짜 Axum router 호출 방식으로 교체해 회귀 감지력을 높인다.
7. **Readiness degraded 표시**: /healthz/ready 응답이 building_reader, vault_kms 상태를 포함하도록 확장된다.

---

## 2. 비목표

- 필드 단위 토큰화(field-level tokenization) — Phase-2 FU 항목
- GDPR right-to-erasure 구현 — Phase-2 FU 항목
- AI 어시스턴트 경로(apps/ai-assistant/) 연동 — AGENTS.md §3 별도 모듈
- Pulumi 외 AWS 콘솔 직접 변경 — AGENTS.md §1 절대 규칙
- 공공 데이터 재배포 · 오픈소스 공개 — AGENTS.md §6 사용자 확인 필요 항목

---

## Design Parts

Detailed design sections are split by responsibility so this spec remains a navigable SSOT instead of a single oversized file.

- [Part 01 - Core Abstractions, Data Flow, And Redaction Policy](./2026-05-08-sp10-5-b-backend-data-correctness-design.part-01-core-redaction.md)
- [Part 02 - Vault, Cleanup, Drift, And Production Rules](./2026-05-08-sp10-5-b-backend-data-correctness-design.part-02-vault-operations.md)
- [Part 03 - Acceptance, Integration Changes, Tasks, And SSS Mapping](./2026-05-08-sp10-5-b-backend-data-correctness-design.part-03-acceptance-tasks.md)
