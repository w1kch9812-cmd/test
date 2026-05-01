# 법제처 국가법령정보 API

## 개요

- 운영 기관: 법제처
- 공식 사이트: https://open.law.go.kr
- 우리 사용: 법령 본문, 조례, 시행령, 별표/서식

## 인증

- 회원가입 → API 사용 신청 → 인증키 발급
- 환경변수: `KOREAN_LAW_API_KEY`
- 단순 등록 (즉시 발급)

## Rate Limit

- 일별 한도 (정확한 수치 사이트 확인)
- 본문 조회는 무거움 — 캐시 필수

## 핵심 endpoint (`crates/data-clients/korean-law/`)

| endpoint | 용도 |
|----------|------|
| `/DRF/lawSearch.do` | 법령 검색 |
| `/DRF/lawService.do` | 법령 본문 조회 |
| `/DRF/lawJoSearch.do` | 조 검색 |
| `/DRF/precSearch.do` | 판례 검색 |
| `/DRF/lawAppendix.do` | 별표/서식 추출 |
| `/DRF/orderSearch.do` | 행정규칙 |

## 요청 예시

```
GET https://www.law.go.kr/DRF/lawSearch.do?
  OC={KOREAN_LAW_API_KEY}
  &target=law
  &type=XML
  &query=국토의 계획 및 이용에 관한 법률
```

응답: XML 또는 JSON (target에 따라).

## Circuit Breaker 정책

- timeout: 15초 (큰 본문)
- retry: 1회
- fallback: cached response (TTL 7일 — 법령은 자주 안 바뀜)

## 캐시 정책

| 종류 | TTL |
|------|-----|
| 법령 검색 결과 | 1시간 |
| 법령 본문 | 7일 (개정 시 invalidate) |
| 별표/서식 | 30일 |

## 라이선스

- 모든 데이터 = 공공저작물
- 재배포 자유 (출처 표기 필수)
- 우리 정책: 매물 분석 화면에 "출처: 법제처" 표기

## 사용자 노출 정책 (옵션 A 준수)

- ✅ 법령 본문 *그대로* 표시 (DB에 저장된 원문)
- ✅ 정식 명칭 + 조·항·호 인용 ("국토의 계획 및 이용에 관한 법률 제76조 제5항")
- ❌ LLM이 *생성*한 법령 인용 (옵션 A 위반 — ADR-0010)
- ❌ "관련 법령에 따라" 같은 추상적 인용

## raw 보존

```sql
create table law_text (
    id char(30) primary key,  -- law_*
    law_id varchar(50) not null,  -- 법제처 식별
    title text not null,
    revision_date date not null,
    raw_xml jsonb not null,
    fetched_at timestamptz not null
);
create index on law_text using gin (raw_xml);
```

법령 텍스트는 *영구 보존* (감사 + 분쟁 시 인용 검증).

## 임베딩 (Phase 3+)

- 법령 본문을 Gemini Embedding 2로 벡터화 → pgvector 저장
- 시맨틱 검색 ("건폐율 변경" → 관련 조항 자동)
- ADR-0011 참조

## 에이전트 경로 (참고)

`korean-law-mcp` (chrisryugj): 41개 법제처 API → 16개 도구. `verify_citations` (환각 검증) 포함.
- 메인 코드 사용 X (옵션 A이라 LLM 인용 자체 없음)
- 개발자 Claude 세션 / `reference/` 학습용
