# crates/observability

tracing + OpenTelemetry + Sentry 통합 어댑터.

## 책임
- `tracing` 매크로 표준 (info!, warn!, error!, debug!, trace!)
- OTel SDK 초기화 (Tempo로 트레이스 전송)
- Prometheus 메트릭 export
- Sentry 에러 보고
- correlation_id 미들웨어 (전 호출 체인 통과)
- PII 마스킹 (로그 필터)

## 의존
- `tracing`, `tracing-subscriber`, `tracing-opentelemetry`
- `opentelemetry`, `opentelemetry-otlp`
- `sentry`, `sentry-tracing`
- `prometheus`

## 정책
- 모든 서비스가 *동일* 초기화 함수 호출 (`init_observability()`)
- correlation_id = ULID, 모든 외부 호출에 전파
- PII (이메일·전화·DI/CI) = 자동 마스킹 (필드명 기반)
- log level = `RUST_LOG` 환경변수
- Sentry sampling rate = production 1.0, 일부 노이즈 endpoint는 0.1

→ ADR-0008, → @docs/observability/README.md
