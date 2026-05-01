# 데이터 소스 카탈로그

외부 공공 API + 상용 API 통합 SSOT.

## 등록된 소스

| 소스 | 운영 기관 | 진입점 | 인증 | 라이선스 | 문서 |
|------|----------|-------|------|---------|------|
| V-World | 공간정보산업진흥원 | api.vworld.kr (REST/WMS/WFS) | API 키 + 도메인 | 공공저작물 (출처 표기) | [v-world.md](./v-world.md) |
| 법제처 (국가법령정보) | 법제처 | open.law.go.kr Open API | API 키 | 공공저작물 | [korean-law.md](./korean-law.md) |
| 공공데이터포털 | 행정안전부 | data.go.kr REST | serviceKey | 데이터셋별 상이 | [data-go-kr.md](./data-go-kr.md) |
| NICE 본인인증 | NICE 평가정보 | (CP 등록 후) | API 키 + 인증서 | 상용 (건당 과금) | [nice-identity.md](./nice-identity.md) |
| Naver Maps | 네이버 클라우드 | maps.apigw.ntruss.com | Client ID/Secret | 무료 12만/월 | [naver-maps.md](./naver-maps.md) |

## 메인 시스템 vs 에이전트 경로

| 경로 | 사용 | 정책 |
|------|-----|------|
| **메인 시스템** (apps/services) | 위 5개 모두 **공식 API 직접** | Circuit Breaker + Retry + Timeout + Audit log + raw_response 보존 |
| **개발자 Claude 세션** | MCP (`korean-land-mcp`, `korean-law-mcp`, `opendata-mcp`) | 메인 코드 import 금지, reference/ 학습용만 |
| **향후 AI 어시스턴트** (옵션 C) | 별도 모듈에서 MCP 사용 가능 | `apps/ai-assistant/` 자리 비워둠 |

## 새 소스 추가 시 템플릿

각 소스 문서는 다음 섹션 포함:

1. 개요 + 운영 기관 + 공식 사이트
2. 인증 방식 + 키 발급 절차
3. Rate Limit / 쿼터
4. 핵심 엔드포인트 + 요청·응답 예시
5. 라이선스 / 재배포 조건
6. 프로덕션 사용 시 주의 (캐시 정책, 출처 표기)
7. Circuit Breaker 정책 (timeout, retry, fallback)
8. raw_response 보존 컬럼 매핑
9. 비용 추정 (월별)

## 추가 후보 (Phase 2+)

- 도로명주소 (juso.go.kr)
- SGIS 통계지리정보 (sgis.kostat.go.kr)
- 토스 페이먼츠 (결제, sub-project별)
- KCB / Toss 본인인증 (NICE 대체)
- Sentry / Grafana Cloud (관측 SaaS)
- Cloudflare API (CDN/WAF 자동화)
