# ADR-0007: 캐시 — moka L1 + Valkey L2

| | |
|---|---|
| 작성일 | 2026-05-01 |
| 상태 | Accepted |
| 결정자 | 운영자 |

## 컨텍스트

V-World/공공데이터포털/법제처 호출 비용·레이턴시 절감. PostGIS 공간 쿼리 결과 캐싱. 세션 저장 (Zitadel). 알림 큐(SQS와 별도). Redis 라이선스 SSPL 변경 (2024) → OSS 진영이 Valkey로 이동.

## 결정

- **L1 (인메모리)**: moka (Rust) — Caffeine 포팅, 표준
- **L2 (분산)**: Valkey (Redis 포크, AWS ElastiCache for Valkey 지원)
- **L3**: 도입하지 않음 (필요 시 별도 ADR)
- **세션 저장**: Valkey
- **TTL 정책**: V-World 응답 24h, 법령 본문 7d, 공시지가 30d, 검색 결과 5분

## 대안

- **Redis (SSPL)**: 라이선스 위험. AWS도 Valkey로 이전 권장
- **Dragonfly**: 빠름·멀티스레드, 그러나 신생, 도입 사례 적음
- **Hazelcast / Apache Ignite**: 무거움, Java
- **Memcached**: 영속성·자료구조 약함

## 결과

- 긍정: Valkey AWS ElastiCache 1급 지원, 라이선스 안전(BSD 3-Clause), Redis 호환 (fred/redis-rs crate 그대로), L1+L2 2층으로 충분 (L3은 운영 복잡도만 추가)
- 부정: Valkey 한국 도입 사례 영어권 자료 부족, Dragonfly·KeyDB 등 대안 평가 시 새로 검토 부담
- 영향 영역: `crates/cache/`, `services/api/`, `infrastructure/` (ElastiCache), `crates/auth/` (세션)

## 재검토 트리거

- Valkey 채택률이 6개월 후 정체 시 Dragonfly 재평가
- L1+L2로 hit ratio 90%+ 달성 못 시 L3 도입 ADR
- 캐시 인프라 비용이 전체 30%+ 시 — 캐시 정책 재설계 + 자체 호스트

## 참조

- → @docs/cache-messaging/README.md (작성 예정)
- → @docs/cache-messaging/redis-l2.md
- Valkey: https://valkey.io
