# compliance/

법적·규제·인증 컴플라이언스 SSOT.

## 책임 영역
- PIPA (개인정보보호법, 한국 필수)
- ISMS-P 인증 (Phase 3+ 후반, 매출 후)
- SOC 2 Type II (B2B 진출 시, Phase 4+)
- ISO 27001 (Phase 4+)
- 공공데이터 라이선스 (각 데이터셋별)
- Audit Log immutable (Cloudflare R2 bucket lock/retention)
- 데이터 retention 정책
- GDPR 호환 (글로벌 진출 시 활성화)
- 우 right to be forgotten (가입 탈퇴 시 데이터 삭제 또는 가명화)
- 법적 보유 (Legal Hold)
- 이용약관 / 개인정보처리방침 / 위치정보 동의

## 작성 예정 문서 (Phase 3+ 점진)
- `pipa.md` — 한국 개인정보보호법 매핑
- `isms-p.md` — 인증 준비 + 추진 일정
- `soc2.md` — SOC 2 Type II (Vanta/Drata 검토)
- `iso-27001.md` — ISO 27001
- `audit-log-immutable.md` — R2 bucket lock/retention 구성
- `data-retention.md` — 영역별 보존 기간
- `gdpr-rtbf.md` — 삭제 흐름 + 가명화
- `public-data-licensing.md` — 데이터셋별 매트릭스
- `legal-pages.md` — 이용약관/개인정보처리방침/위치정보 동의
- `data-classification.md` — Public/Internal/Confidential/Restricted

## 관련 ADR
- → @docs/adr/0010-scope-information-platform-option-a.md (옵션 A 범위 — 컴플라이언스 부담 낮춤)
- (Phase 3+ 인증 진입 시 추가 ADR)

## 관련 컨벤션
- → @docs/conventions/ui-writing-korean.md (사용자 동의 UI)
