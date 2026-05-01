# cost/

Phase별 비용 추정·절감·관측 SSOT.

## 책임 영역
- AWS 인프라 비용 (Phase별)
- Cloudflare Free 활용 (CDN/WAF/DDoS 무료)
- Reserved Instance / Savings Plan 전략 (Phase 3+ 안정화 후)
- Spot 인스턴스 (워커/배치)
- 외부 SaaS 비용 (Sentry/Snyk/PagerDuty 등)
- 한국 본인인증 (NICE 건당 100-300원)
- Naver Maps API (월 12만 무료, 초과 ~3원/호출)
- 컴플라이언스 인증 (ISMS-P 5천만~1억/년, Phase 3+)
- AWS Cost Explorer + Budgets 알림

## Phase별 비용 (요약)

| Phase | 사용자 | 월 비용 (RI 적용 후) |
|-------|--------|--------------------|
| 0 (코드 작성) | 0 | ₩0 |
| 1 (스테이징) | 0 | ~₩5만 |
| 2 (베타) | 1,000 | ~₩20만 |
| 3 (출시) | 10,000 | ~₩55만 |
| 4 (성장) | 100,000 | ~₩195만 |

## 작성 예정 문서
- `phase-1-staging.md` — t3.small + RDS micro 비용 분해
- `phase-2-beta.md` — ECS Fargate 시작
- `phase-3-launch.md` — RI 30% 할인 + Multi-AZ
- `phase-4-growth.md` — 100K+ 사용자, 풀 인프라
- `aws-cost-optimization.md` — RI/Spot/Graviton/lifecycle 전략
- `cloudflare-utilization.md` — WAF/CDN/DDoS 무료 활용
- `saas-vs-self-host.md` — Sentry/Grafana 등 비용 비교
- `compliance-cost.md` — ISMS-P/SOC2/펜테스트 추정

## 관련 ADR
- → @docs/adr/0008-observability-grafana-otel-sentry.md (셀프호스트로 비용 절감)

## 관련 컨벤션
- (별도 없음 — TECH.md § 6 비용 요약 + 본 폴더가 SSOT)
