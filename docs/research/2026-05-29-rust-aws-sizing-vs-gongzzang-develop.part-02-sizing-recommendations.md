## 6. 기존 사양이 실제로 과한 부분

### 6.1 API task

기존 API:

```text
2 vCPU / 8GB
Kotlin + Spring Boot + JPA/Hibernate + WebFlux + Security
```

새 Rust API:

```text
axum + tokio + sqlx
native binary
JVM heap 없음
```

초기 Rust API에 기존 8GB 메모리는 과하다. 일반적인 시작은 1-2GB가 맞다.

권장:

| 단계 | API 사양 |
|---|---:|
| local/prelaunch | 0.5 vCPU / 1GB |
| launch | 1 vCPU / 2GB |
| API CPU 병목 확인 후 | 2 vCPU / 4GB |
| HA 필요 시 | 큰 1개보다 작은 task 2개 |

### 6.2 DB

기존 DB:

```text
db.m7g.xlarge
4 vCPU / 16GB
1000GB gp3
Multi-AZ
12000 IOPS
500 MiB/s throughput
```

새 구조는 Bronze/raw/tiles를 R2로 빼고, 전국 정적 공간 payload를 RDS hot path로 반복 제공하지 않는 방향이다.
따라서 처음부터 1TB, Multi-AZ, 12000 IOPS를 잡을 필요가 없다.

권장:

| 단계 | DB 사양 |
|---|---:|
| 비용 최소 | `db.t4g.medium`, 200GB gp3, Single-AZ |
| 현실적 launch | `db.m7g.large`, 300GB gp3, Single-AZ |
| 증거 기반 확장 | `db.m7g.xlarge` |
| 장애 허용도 요구 시 | Multi-AZ |
| I/O 병목 증명 후 | gp3 IOPS/throughput 추가 |

DB를 `db.t4g.medium`으로 시작할 수는 있다. 다만 다음 중 하나라도 빠르게 나타나면 `db.m7g.large`로 올리는 게 맞다.

- freeable memory가 낮다.
- shared buffer hit ratio가 나쁘다.
- PostGIS query p95가 튄다.
- CPU credit/burst 문제가 있다.
- connection이 자주 포화된다.
- autovacuum/maintenance 때 API latency가 흔들린다.

### 6.3 ALB

기존은 API/Web/Admin/LLM/Keycloak로 ALB가 최소 5개다.
초기 새 구조는 단일 진입점과 path/host routing으로 ALB 1개를 목표로 해야 한다.

권장:

```text
Cloudflare
  -> ALB 1개
  -> gongzzang web/api
  -> platform-core api
  -> admin은 필요 시만
```

### 6.4 Redis/Valkey

기존은 ElastiCache Serverless Redis다. 작은 예측 가능한 부하에서는 작은 고정 Valkey 노드가 더 단순하고 저렴할 수 있다.

초기 권장:

| 단계 | Cache |
|---|---:|
| prelaunch | Docker Valkey |
| launch | ElastiCache Valkey `cache.t4g.small` |
| 성장 | marker/session/rate-limit cache 분리 |

## 7. 실제로 제대로 굴러간다는 기준

사양을 감으로 고정하지 말고, 아래 지표를 만족하면 "충분하다"고 본다.

### 7.1 API

| 지표 | 충분한 상태 |
|---|---:|
| p95 | 300ms 이하 |
| p99 | 1000ms 이하 |
| error rate | 1% 미만 |
| CPU | 평균 60% 이하, burst 후 회복 |
| memory | OOM 없음, 지속 증가 없음 |
| restart | 비정상 restart 0 |

### 7.2 DB

| 지표 | 충분한 상태 |
|---|---:|
| CPU | 평균 60% 이하 |
| freeable memory | 안정적 |
| connections | max의 70% 이하 |
| read/write latency | 안정적, spike 원인 설명 가능 |
| IOPS | baseline 안에서 여유 |
| slow query | hot endpoint에서 반복 발생 없음 |
| locks | 사용자 요청 경로 차단 없음 |

### 7.3 Cache

| 지표 | 충분한 상태 |
|---|---:|
| memory | 70% 이하 |
| eviction | 의도하지 않은 eviction 없음 |
| command latency | 낮고 안정적 |
| hit rate | marker/tile hot path에서 상승 |
| fail mode | cache 장애 시 DB 보호 정책 존재 |

### 7.4 Worker

| 지표 | 충분한 상태 |
|---|---:|
| queue backlog | 감소하거나 SLA 안에 유지 |
| retry rate | 원인 설명 가능 |
| memory | shard 처리 중 OOM 없음 |
| R2 throughput | 병목이 API에 영향 주지 않음 |
| public API quota | cap 안에서 실행 |

## 8. 권장 사양안

### 8.1 비용 최소 prelaunch

목적: 개발/검증/소규모 demo.

| 컴포넌트 | 사양 |
|---|---:|
| 앱 서버 | EC2 `t4g.large` 또는 `t4g.xlarge` 1대 Docker |
| DB | RDS `db.t4g.medium`, gp3 200GB |
| Cache | Docker Valkey |
| ALB | 1개 또는 Cloudflare tunnel/reverse proxy |
| Worker | 같은 EC2에서 제한적으로 |
| AI | 로컬 별도 장비 |

장점: 싸다.
단점: HA와 운영 분리가 약하다.

### 8.2 현실적 launch

목적: 실제 사용자 오픈, 비용 통제, 최소 안정성.

| 컴포넌트 | 사양 |
|---|---:|
| `gongzzang web` | Fargate 0.5 vCPU / 1GB |
| `gongzzang api` | Fargate 1 vCPU / 2GB |
| `platform-core api` | Fargate 1 vCPU / 2GB |
| worker | Fargate/EC2 1-2 vCPU / 2-4GB, 필요 시만 실행 |
| DB | RDS `db.m7g.large`, gp3 300GB, Single-AZ |
| Cache | ElastiCache Valkey `cache.t4g.small` |
| ALB | 1개 |
| R2 | Bronze/Silver/Gold/tile/file |
| AI | 로컬 GPU worker |

이 구성이 현재 기준 가장 균형이 좋다.

### 8.3 성장 단계

조건: 실제 지표로 병목이 확인된 경우.

| 증상 | 확장 |
|---|---|
| API CPU 60-70% 지속 | API task 2개 또는 2 vCPU / 4GB |
| API memory 지속 증가 | leak 확인 후 2-4GB |
| DB CPU/메모리 병목 | `db.m7g.xlarge` |
| DB read-heavy | read replica 또는 read model |
| IOPS latency | gp3 IOPS/throughput 추가 |
| 장애 허용도 요구 | Multi-AZ |
| marker tile miss 부하 | Valkey cache, precompute, CDN 강화 |
| worker backlog 증가 | worker만 수평/일시 확장 |

## 9. 기존 대비 절감 판단

| 영역 | 기존 | 새 구조 필요 추정 | 판단 |
|---|---:|---:|---|
| API app | 2 vCPU / 8GB | 1 vCPU / 2GB | 크게 줄여도 됨 |
| Platform Core API | 없음/혼재 | 1 vCPU / 2GB | 새로 필요하지만 작음 |
| DB | 4 vCPU / 16GB, 1TB, Multi-AZ, high IOPS | 2 vCPU / 8GB, 300GB, Single-AZ | 기존은 초기 기준 과함 |
| Redis | Serverless 1GB | 작은 Valkey | 작게 시작 가능 |
| ALB | 5개 | 1개 | 크게 줄일 수 있음 |
| LLM | AWS 상시 | 로컬 worker | AWS 고정비 제거 가능 |
| Bronze/raw | DB 또는 혼재 위험 | R2 | DB 부담 감소 |
| 지도 정적 payload | PostGIS hot path 위험 | R2/PBF/PMTiles | DB 비용 방어 |

## 10. 사양 결정 규칙

초기에는 아래 순서로 결정한다.

1. API는 Rust 기준으로 작게 시작한다.
2. DB는 `db.t4g.medium`과 `db.m7g.large` 중 비용/안정성 우선순위로 고른다.
3. Bronze/raw/tile은 R2로 보내 DB 저장소를 키우지 않는다.
4. PostGIS는 hot path를 제한한다.
5. worker는 API와 분리하고, 필요할 때만 키운다.
6. AI는 AWS 상시 CPU/RAM이 아니라 로컬 GPU worker로 붙인다.
7. Multi-AZ, high IOPS, read replica는 지표로 필요성이 증명된 뒤 켠다.

## 11. 런칭 전 반드시 할 테스트

`gongzzang-develop`의 기존 큰 사양이 실제로 필요했는지 비교하려면 같은 시나리오로 새 구조를 때려봐야 한다.

이 장의 테스트를 실행해야 비로소 "제대로 굴러가는 실제 사양"이라고 말할 수 있다.

### 11.1 최소 k6 시나리오

| 시나리오 | 목표 |
|---|---|
| health/readiness | 기본 안정성 |
| listing list/detail | B2C 기본 조회 |
| listing write | DB write path |
| marker tile read | 지도 hot path |
| platform-core catalog read | PNU/building/anchor read |
| auth/session refresh | Redis/Valkey path |
| webhook/outbox receive | consumer invalidation |

### 11.2 단계별 부하

| 단계 | 부하 |
|---|---:|
| smoke | 5 read RPS + 2 health RPS |
| beta | 20 read RPS + 5 write RPS |
| launch rehearsal | 50 read RPS + 10 write RPS |
| spike | 100-200 read RPS 단기 burst |

### 11.3 판정

각 단계에서 아래를 기록한다.

- API p95/p99
- API CPU/memory
- DB CPU/free memory/connection
- DB read/write latency
- slow query top N
- Valkey latency/hit rate
- R2 request latency
- worker backlog
- error rate

이 데이터를 보고 `db.t4g.medium`으로 충분한지, `db.m7g.large`가 필요한지, API task를 늘려야 하는지 결정한다.

## 12. 최종 판단

기존 `gongzzang-develop` 사양은 "서비스를 안정적으로 돌리기 위한 최소치"라기보다, JVM/Spring 단일 대형 API와 큰 RDS/PostGIS 중심 설계에 맞춰 **미리 크게 잡은 운영 사양**에 가깝다.

새 구조는 다르게 가야 한다.

```text
Rust API는 작게 시작
DB는 중간 크기부터 보수적으로
전국 raw/tile은 R2
PostGIS는 hot path가 아니라 serving mirror/scratch
worker는 API와 분리
AI는 로컬 GPU worker
```

따라서 현재 추천은 다음이다.

```text
초기 비용 최소:
  EC2 t4g.xlarge + RDS db.t4g.medium + Docker Valkey + R2

실제 launch 권장:
  small Fargate tasks + RDS db.m7g.large Single-AZ + cache.t4g.small Valkey + R2

확장:
  지표로 증명된 뒤 db.m7g.xlarge, Multi-AZ, read replica, high IOPS 순서
```

Rust 덕분에 기존 API task의 2 vCPU / 8GB는 초기 Rust API에 그대로 필요하지 않을 가능성이 높다.
하지만 제대로 굴러가는지를 결정하는 최종 기준은 언어가 아니라 DB/PostGIS 쿼리, 지도 hot path, cache hit, worker 격리, 그리고 부하 테스트 지표다.

현재 문서의 결론은 **측정 완료 결론이 아니라 테스트할 사양 후보 결론**이다.
