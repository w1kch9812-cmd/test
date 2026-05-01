# ADR-0011: 임베딩 — Gemini Embedding 2 + pgvector (Phase 3+)

| | |
|---|---|
| 작성일 | 2026-05-01 |
| 상태 | Accepted (Phase 3+ 도입, Phase 1엔 자리만) |
| 결정자 | 운영자 |

## 컨텍스트

옵션 A 데이터 플랫폼이지만 *시맨틱 검색* (자연어 → 매물 매칭), *유사 매물 추천*, *법령 의미 검색*은 키워드 매칭의 한계 극복에 필요. 임베딩 = 텍스트→벡터 변환 (생성 X) → ADR-0010 옵션 A 위반 아님.

## 결정

- **임베딩 모델**: Google Gemini Embedding 2 (gemini-embedding-2)
- **벡터 DB**: pgvector (PostgreSQL 확장, ADR-0004 동일 인스턴스)
- **차원**: 768 (권장 균형) — 추후 1536/3072 평가
- **배치 처리**: 50% 할인 활용
- **도입 시점**: Phase 3 (출시 후 사용자 ~10K 도달 시)
- **Phase 1**: `crates/embedding/` 자리 + README만. 코드 없음
- **사용 사례**: 시맨틱 검색, 유사 매물 추천, 유사 제조업체, 법령 의미 검색, 검색 의도 분류

## 대안

- **OpenAI text-embedding-3-large**: 표준, $0.13/1M tokens (Gemini 5x 비쌈)
- **Cohere embed-multilingual-v3**: 다국어 강함, $0.10/1M (중간)
- **Jina Embeddings v3**: 가장 저렴 ($0.018/1M), 신생
- **BGE-M3 (오픈소스)**: 셀프호스트 GPU 비용, 한국어 강함 — Phase 4+ 옵션
- **KoSBERT (한국어 특화)**: 일반 의미 약함

## 결과

- 긍정: Gemini = 가성비 1순위 ($0.025/1M tokens, 배치 $0.0125), 8K 토큰 컨텍스트, 100+ 언어, pgvector = 별도 인프라 0 (PostgreSQL 그대로)
- 부정: Google API 의존 (대안: BGE-M3 셀프호스트), 도메인 미세조정 안 됨 (Phase 4+ 옵션), pgvector 1억 벡터 한계 (그 후 Qdrant 분리)
- 영향 영역: `crates/embedding/` (Phase 3+), `db/migration/V***__pgvector.sql`, `services/search-indexer/`

## 재검토 트리거

- 한국어 도메인 특화 임베딩이 Gemini 일반 임베딩 대비 검색 정확도 20%+ 개선 시 → BGE-M3 셀프호스트
- pgvector 1억 벡터 한계 도달 시 → Qdrant 분리
- Google API 비용이 검색 비용 50%+ 차지 시 → 셀프호스트 또는 다른 모델
- 사용자가 "AI 챗" 요구 시 → 옵션 C (`apps/ai-assistant/`) 활성화 (이 ADR과 별개)

## 참조

- → @docs/data/search.md (작성 예정)
- → @docs/adr/0010-scope-information-platform-option-a.md (옵션 A 범위)
- Gemini Embedding: https://ai.google.dev/gemini-api/docs/embeddings
- pgvector: https://github.com/pgvector/pgvector
