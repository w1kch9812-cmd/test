# crates/domain/regulation

Regulation Bounded Context — 법령·규제 도메인.

## Aggregates
- **Law** — 법제처 법령·시행령·시행규칙 (캐시)
- **Regulation** — 산업단지 규제, 환경 기준
- **Permit** (Phase 2+) — 건축 허가, 환경 허가

## 의존
- `crates/domain/shared-kernel`
- 외부 의존 0

## 정책
- 법령 텍스트는 *원문 그대로* (LLM 가공 금지 — 옵션 A)
- 인용 형식: 정식 명칭 + 조·항·호 ("국토의 계획 및 이용에 관한 법률 제76조 제5항")
- raw_response 영구 보존
- 시맨틱 검색 (Phase 3+) = 임베딩만, 텍스트 생성은 X
