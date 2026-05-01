# observability/

관측성 SSOT — 메트릭/로그/추적/에러/SLO/On-call.

## 책임 영역
- OpenTelemetry SDK + Collector
- Prometheus + Grafana
- Loki (로그)
- Tempo (분산 추적)
- Vector (로그 수집)
- Sentry (에러 + RUM)
- Grafana OnCall (또는 PagerDuty Phase 3+)
- SLO 정의 + Error Budget
- DORA 메트릭
- 합성 모니터링 (k6 cron)
- 비용 관측

## 작성 예정 문서 (sub-project 7)
- `otel.md` — OTel SDK + Collector 설정
- `sentry.md` — 셀프호스트 → SaaS 전환
- `prometheus.md` — 메트릭 정의 (RED + USE)
- `loki.md` — 로그 라벨링 + retention
- `tempo.md` — 트레이스 샘플링
- `grafana.md` — 대시보드 + 알림
- `slo.md` — SLO 정의 + Error Budget
- `on-call.md` — 알림 흐름 + 런북
- `rum.md` — Real User Monitoring
- `dora-metrics.md` — DORA 자동 수집

## 관련 ADR
- → @docs/adr/0008-observability-grafana-otel-sentry.md

## 관련 컨벤션
- → @docs/conventions/comments.md (correlation_id 추적)
