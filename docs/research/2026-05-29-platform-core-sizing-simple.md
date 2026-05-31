# Platform Core Sizing Simple Brief

Created: 2026-05-29

## 한 줄 결론

Platform Core 단독은 현재 로컬 증거 기준으로 `200 read RPS`까지 안정권이다.
초기 운영 스펙은 보수적으로 `API 2 tasks x 1 vCPU / 2GB`부터 시작한다.

이 수치는 출시 확정값이 아니라 다음 perf/staging 부하 테스트의 시작점이다.

## 쉽게 말하면

`동시접속자`는 직접 재는 값이 아니라 보통 `RPS`에서 환산한다.

공식은 이렇다.

```text
활성 동시접속자 ~= RPS x 유저 1명의 요청 간격(초) / 유저 1회 행동당 요청 수
```

예를 들어 Platform Core를 `200 read RPS`로 본다.

| 유저 행동 가정 | 환산 활성 동시접속자 |
| --- | ---: |
| 1명이 5초에 1번 Platform Core 조회 | 약 1,000명 |
| 1명이 10초에 1번 Platform Core 조회 | 약 2,000명 |
| 1명이 1초에 1번 Platform Core 조회 | 약 200명 |

그래서 지금 말할 수 있는 쉬운 표현은 이거다.

> Platform Core 단독은 현재 로컬 기준으로 일반 조회 패턴에서 활성 동접
> 약 1,000~2,000명급을 다음 테스트의 1차 목표로 잡을 수 있다.

단, 이것은 로컬 테스트 환산이다. AWS 운영 스펙 확정은 아니다.

## 현재 증거

Platform Core 단독 로컬 테스트:

| 목표 read RPS | 결과 |
| ---: | --- |
| 200 | 안정권. 에러 0%, p95 약 2.09ms, p99 약 2.64ms |
| 500 | 에러는 0%지만 dropped iterations 발생. 한계 구간 시작 |
| 1000 | 목표를 다 못 채움. 실제 약 804 read RPS 수준에서 runner/host 한계 |

Gongzzang + Platform Core 로컬 연동 스모크:

| 항목 | 값 |
| --- | ---: |
| classification | healthy |
| p95 | 15.250ms |
| p99 | 26.543ms |
| error rate | 0% |

증거 파일:

```text
target/audit/load-tests/2026-05-29/local/api-read-mix/20260529T160233+0900
C:/Users/admin/Desktop/platform-core/target/load/local-sizing-20260529T013358Z/local-sizing-matrix.json
```

## 초기 추천 스펙

### Platform Core 단독

보수적 시작점:

| 구성요소 | 시작 스펙 |
| --- | --- |
| API | ECS Fargate `2 tasks x 1 vCPU / 2GB` |
| DB | RDS PostgreSQL `db.m7g.large`, Multi-AZ |
| Cache | ElastiCache/Valkey `cache.t4g.small` 이상 |
| Autoscaling | CPU 60%, p95 latency, request count 기준 |

최소 개발/검증용:

| 구성요소 | 최소 스펙 |
| --- | --- |
| API | `1 task x 1 vCPU / 2GB` |
| DB | `db.t4g.medium` |
| Cache | small class |

운영은 최소 스펙으로 시작하지 않는다. 장애 격리와 배포 안정성을 위해 API는
처음부터 2 tasks로 둔다.

## 장애한계점 판단

현재 로컬 기준 첫 한계 신호는 `500 read RPS`부터다.

한계 신호는 다음 순서로 본다.

1. dropped iterations가 생기는가
2. p95가 500ms를 넘는가
3. p99가 1500ms를 넘는가
4. error rate가 1%를 넘는가
5. API CPU가 60~70%를 계속 넘는가
6. DB connection, DB latency, cache latency가 같이 튀는가

위 항목 중 하나라도 먼저 터지는 지점이 첫 병목이다.

## 다음에 해야 할 테스트

다음 테스트는 로컬이 아니라 perf/staging에서 한다.

1. Platform Core 단독: 200, 300, 400, 500, 700, 900 read RPS
2. Gongzzang + Platform Core: 50, 100, 200, 300, 500 RPS
3. 지도 marker tile 경로: cache hit/miss를 나눠서 측정
4. 30분 baseline, 10분 stress, 3분 spike, 2시간 soak
5. API, DB, cache, queue, worker metric을 같은 run id로 묶어서 저장

출시 사양은 위 테스트에서 `healthy`가 나온 가장 작은 스펙으로 잡고, 그 위에
30~50% headroom을 붙여 결정한다.

## 지금 결론

현재는 이렇게 잡는다.

```text
Platform Core 단독 예상 안정권: 200 read RPS
동시접속자 환산: 약 1,000~2,000 active users
초기 운영 API 스펙: 2 x 1 vCPU / 2GB
초기 운영 DB 스펙: db.m7g.large Multi-AZ
장애한계 탐색 구간: 500~800 read RPS
```

## 참고 기준

- AWS Prescriptive Guidance는 부하를 `requests per second`, 응답 시간, 또는
  concurrent users 중 무엇으로 정의할지 먼저 정하라고 안내한다.
- AWS ECS Fargate는 task 수준에서 CPU와 memory를 지정해야 한다.
- AWS Well-Architected Performance Efficiency는 workload load test를 성능
  검증 항목으로 둔다.

References:

- <https://docs.aws.amazon.com/prescriptive-guidance/latest/load-testing/test-types.html>
- <https://docs.aws.amazon.com/AmazonECS/latest/developerguide/fargate-task-size-best-practice.html>
- <https://docs.aws.amazon.com/AmazonECS/latest/developerguide/capacity-tasksize.html>
- <https://docs.aws.amazon.com/wellarchitected/latest/framework/performance-efficiency.html>
