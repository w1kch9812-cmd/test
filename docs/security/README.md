# security/

보안·취약성·공급망 SSOT.

## 책임 영역
- OWASP ASVS Level 3 준수
- PIPA (한국 개인정보보호법)
- ISMS-P 인증 준비 (Phase 3+ 후반)
- 데이터 분류 (Public/Internal/Confidential/Restricted)
- PII 마스킹 (자동 미들웨어)
- 암호화 at-rest (AES-256-GCM) + in-transit (TLS 1.3)
- Field-level 암호화 (사업자번호, 주민번호, DI/CI)
- Secrets 관리 (AWS Secrets Manager + Vault)
- SAST (Semgrep + CodeQL)
- DAST (OWASP ZAP)
- SCA (cargo-audit + cargo-deny + Snyk + socket.dev)
- 시크릿 스캔 (gitleaks)
- 컨테이너 스캔 (Trivy)
- Threat Modeling (STRIDE)
- Penetration Testing (분기, Phase 3+)
- Bug Bounty (HackerOne, Phase 4+)

## 작성 예정 문서 (전반, sub-project별 점진)
- `owasp-asvs.md` — Level 3 체크리스트
- `pipa.md` — 한국 개인정보보호법
- `isms-p.md` — 인증 추진 (Phase 3+)
- `data-classification.md` — 분류 매트릭스
- `pii-masking.md` — 자동 미들웨어 패턴
- `encryption.md` — AES-GCM + KMS
- `secrets.md` — Vault + Secrets Manager
- `sast-dast.md` — 도구 + CI 통합
- `supply-chain-slsa.md` — SLSA Level 3 + SBOM
- `threat-modeling.md` — STRIDE
- `pen-test.md` — 외부 펜테스트 운영

## 관련 ADR
- (도입 시 별도 ADR 작성 — 0012+)

## 관련 컨벤션
- → @docs/conventions/error-format.md (PII 마스킹 메시지)
