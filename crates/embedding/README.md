# crates/embedding

Phase 3+ 자리. 시맨틱 검색·유사 매물 추천을 위한 임베딩.

## 도입 시점
**Phase 3 (사용자 ~10K 도달 후)**. Phase 1엔 이 폴더만 자리 잡음.

## 책임 (Phase 3+)
- Gemini Embedding 2 API 호출 (배치 50% 할인 활용)
- pgvector 저장 (`embedding vector(768)` 컬럼)
- HNSW 인덱스 관리
- 유사도 검색 헬퍼 (cosine distance)
- 도메인별 임베딩 (매물·제조업체·법령 텍스트)
- 재인덱싱 (업데이트된 텍스트)

## 의존 (Phase 3+)
- `crates/data-clients/gemini-embedding`
- `crates/db`
- `crates/observability`
- `pgvector` Rust crate

## 정책 (Phase 3+ 도입 시)
- 임베딩 = 검색·추천에만 (LLM 생성 X — 옵션 A)
- raw 텍스트 + 임베딩 둘 다 저장 (재계산 가능)
- 비용 모니터링 (월 임베딩 토큰 수)
- Fallback: Postgres FTS (Gemini API 실패 시)

→ ADR-0011, → ADR-0010 (옵션 A 범위)
