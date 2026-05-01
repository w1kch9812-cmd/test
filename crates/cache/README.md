# crates/cache

moka L1 (인메모리) + Valkey L2 (분산) 캐시 추상화.

## 책임
- L1: moka (Caffeine 포팅, 프로세스 내)
- L2: Valkey (fred 또는 redis-rs 클라이언트)
- 통합 인터페이스 (Cache trait — get/set/invalidate/ttl)
- TTL 정책 (캐시 종류별)
- Pub/Sub (세션 무효화)

## 의존
- `moka` crate
- `fred` 또는 `redis` crate
- `crates/observability`

## 정책
- 캐시 종류별 TTL 명시 (V-World 24h, 법령 7d, 검색 5min)
- L1 miss → L2 조회 → L2 miss → origin 호출 (자동 promote)
- 무효화 = Pub/Sub로 모든 인스턴스 즉시 전파
- 캐시 hit ratio 메트릭 + Sentry 알림

→ ADR-0007, → @docs/cache-messaging/README.md
