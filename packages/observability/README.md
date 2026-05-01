# @gongzzang/observability

로깅, 메트릭, 추적, 에러 리포팅 통합 어댑터.

## 도구

- **Sentry** (에러 추적)
- **OpenTelemetry** (분산 추적, 메트릭)
- **Pino** (구조화 로깅)

## 통합 인터페이스

```typescript
import { logger, metrics, tracer } from "@gongzzang/observability";

logger.info({ pnu: "..." }, "parcel fetched");
metrics.counter("vworld.calls").inc();
tracer.startSpan("fetchVWorld", async () => { ... });
```

## 정책

- 모든 외부 API 호출 자동 추적 (`packages/data-clients`에서 미들웨어 적용)
- PII는 로그에 남기지 않음 (마스킹 미들웨어)
- 환경별 destination 분리 (개발=stdout, 운영=Sentry+CloudWatch)
