# ADR-0008: 관측성 — Grafana + Prometheus + Loki + Tempo + Sentry + OpenTelemetry

| | |
|---|---|
| 작성일 | 2026-05-01 |
| 상태 | Accepted |
| 결정자 | 운영자 |

## 컨텍스트

SSS 7 기둥 중 *추적성*과 *가시성* 필수. 모든 외부 호출 audit, 모든 변경 추적, 새벽 3시 장애 자동 알림. 옵션 A 데이터 플랫폼 + 향후 사용자 트래픽 대비. 비용은 돈만 고려 (시간/구조 비용 무관).

## 결정

- **Tracing**: OpenTelemetry SDK + Collector (사실상 표준)
- **메트릭**: Prometheus
- **로그**: Loki (Grafana 진영) + Vector (수집)
- **분산 추적**: Tempo
- **시각화**: Grafana
- **에러/RUM**: Sentry (Phase 1-2 셀프호스트 → Phase 3+ SaaS)
- **합성 모니터링**: k6 + GitHub Actions cron
- **On-call**: Grafana OnCall (OSS) → Phase 3+ PagerDuty 재고
- **DORA 메트릭**: 자체 (PostgreSQL + Grafana 대시보드)

## 대안

- **Datadog**: 풀스택 SaaS, 매우 비쌈 (Phase 3 기준 월 ~$1,500+) → Grafana로 대체 가능
- **New Relic**: 비슷한 비용
- **Honeycomb**: 분산 추적 1급, Phase 3+ 보조 도입 가능 (옵션)
- **AWS CloudWatch만**: 록인 + 시각화 약함
- **Splunk**: 엔터프라이즈 스탠다드, 매우 비쌈

## 결과

- 긍정: 셀프호스트 풀스택 = 비용 효율 (Datadog 대비 90%+ 절감), 오픈 표준(OTel) 록인 0, 분산 추적·메트릭·로그 통합, Sentry 셀프 → SaaS 전환 부담 낮음
- 부정: 셀프호스트 운영 부담 (Phase 3+ Grafana Cloud 또는 SaaS 전환 검토), 학습 곡선 (OTel 의미 컨벤션, Grafana SLO)
- 영향 영역: `crates/observability/`, 모든 `services/`, 인프라 (Grafana stack 별도 ECS), `infrastructure/observability/`

## 재검토 트리거

- 셀프호스트 운영이 SRE 담당자 시간 30%+ 차지 시 → Grafana Cloud Pro 전환
- 분산 추적 부하가 Tempo 한계 도달 시 → Honeycomb 부분 도입
- 컴플라이언스 (ISMS-P) 요구사항이 SaaS 사용 제약 시 → 자체 호스트 강화

## 참조

- → @docs/observability/README.md (작성 예정)
- OpenTelemetry: https://opentelemetry.io
- Grafana SLO: https://grafana.com/products/cloud/slo/
