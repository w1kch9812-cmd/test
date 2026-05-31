# Rust 기반 새 구조 AWS 사양 분석

작성일: 2026-05-29
대상: `gongzzang-develop` 기존 AWS 사양 vs `gongzzang + platform-core` Rust 기반 새 구조
목적: 기존처럼 큰 사양을 계속 써야 하는지, 실제 런칭 전후 어느 정도 사양이면 제대로 운영 가능한지 판단한다.

## 0. 분석 범위와 한계

이 문서는 **실제 AWS 인스턴스 크기를 바꿔가며 수행한 부하 테스트 결과가 아니다**.

현재 문서의 근거는 세 가지다.

1. 기존 `gongzzang-develop` Pulumi/ECS/RDS 코드에 선언된 실제 할당 사양
2. 새 `gongzzang + platform-core` Rust 코드 구조와 런타임 특성
3. 이미 존재하는 `platform-core` 로컬/regional smoke evidence

따라서 이 문서는 "기존 사양이 왜 과했을 가능성이 높은지"와 "초기 사양을 어디서 시작해야 하는지"를 판단하는 **사전 sizing 분석**이다.

아직 검증하지 않은 것은 다음이다.

| 항목 | 상태 |
|---|---|
| AWS에서 `db.t4g.medium` 실제 부하 테스트 | 미실행 |
| AWS에서 `db.m7g.large` 실제 부하 테스트 | 미실행 |
| `gongzzang api` 실제 listing/search/write 부하 테스트 | 미실행 |
| `platform-core api` 전국 규모 read model 부하 테스트 | 미실행 |
| 전유부 `getBrExposInfo` Bronze pilot size/parse 측정 | 미실행 |
| 로컬 AI worker 연결 후 end-to-end 정규화 처리량 측정 | 미실행 |

그러므로 최종 결론은 "이 사양이면 무조건 충분하다"가 아니라, **이 사양부터 테스트하는 것이 합리적이다**이다.

## 1. 결론

Rust로 바꿨기 때문에 **API 애플리케이션 서버는 기존보다 훨씬 작게 시작해도 된다**.

하지만 전체 시스템 사양을 모두 줄여도 된다는 뜻은 아니다. 이 플랫폼에서 비용과 성능을 결정하는 큰 축은 API 런타임보다 다음이다.

1. PostgreSQL/PostGIS 쿼리
2. 지도/마커/타일 hot path
3. Bronze/Silver 정규화 worker
4. R2 대용량 object 읽기/쓰기
5. 캐시/락/rate limit 설계
6. AI/정규화 worker의 GPU 또는 CPU 처리량

따라서 추천 시작점은 다음이다. 이 표는 측정 완료값이 아니라 **부하 테스트 시작값**이다.

| 영역 | 런칭 전/초기 추천 |
|---|---:|
| `gongzzang api` | 0.5-1 vCPU / 1-2GB |
| `platform-core api` | 1 vCPU / 2GB |
| `gongzzang web` | 0.5 vCPU / 1GB, SSR 무거우면 1 vCPU / 2GB |
| background worker | 평시 1 vCPU / 2GB, 배치 때 2-4 vCPU / 4-8GB |
| DB 최소 | RDS `db.t4g.medium`, gp3 200GB |
| DB 현실 권장 | RDS `db.m7g.large`, gp3 300GB, Single-AZ |
| Cache | Docker Valkey 또는 ElastiCache Valkey `cache.t4g.small` |
| Load balancer | ALB 1개 |
| Bronze/Silver/Gold 파일 | R2 중심 |
| AI | AWS 상시 서버가 아니라 로컬 AI worker 연결 |

가장 현실적인 1차 운영 후보는 `db.m7g.large` Single-AZ + 작은 Rust API task 조합이다.
비용을 더 강하게 줄여야 하면 `db.t4g.medium`으로 시작하되, PostGIS/DB 지표를 더 자주 봐야 한다.

## 2. 기존 `gongzzang-develop` 실제 할당 사양

근거 파일:

- `C:/Users/admin/Desktop/gongzzang-develop/infrastructure/Pulumi.prod.yaml`
- `C:/Users/admin/Desktop/gongzzang-develop/infrastructure/src/database/rds.ts`
- `C:/Users/admin/Desktop/gongzzang-develop/infrastructure/src/database/elasticache.ts`
- `C:/Users/admin/Desktop/gongzzang-develop/infrastructure/src/loadbalancing/load-balancers.ts`
- `C:/Users/admin/Desktop/gongzzang-develop/infrastructure/src/ecs/task-definitions/*`

기존 API는 Kotlin/Spring Boot/JVM 기반이다.

`gongzzang-develop/gongzzang-api/modules/app/build.gradle.kts` 기준으로 Spring Boot, JPA/Hibernate, Web/WebFlux, Security, Redis, Elasticsearch client, Sentry, Slack, ShedLock 등이 같이 올라간다.

기존 prod 사양:

| 항목 | 기존 할당 |
|---|---:|
| API task | 2 vCPU / 8GB |
| Platform Web | 0.5 vCPU / 1GB |
| Admin Web | 0.5 vCPU / 1GB |
| LLM service | 1 vCPU / 2GB |
| Keycloak | 0.5 vCPU / 1GB |
| API DB | RDS `db.m7g.xlarge` |
| API DB CPU/RAM | 4 vCPU / 16GB |
| API DB storage | gp3 1000GB |
| API DB Multi-AZ | true |
| API DB IOPS | 12000 |
| API DB throughput | 500 MiB/s |
| Keycloak DB | RDS `db.t4g.micro`, 20GB |
| Redis | ElastiCache Serverless Redis, max 1GB |
| ALB | API/Web/Admin/LLM/Keycloak로 최소 5개 |

이 구조는 런칭 전/초기 트래픽 대비 과하게 잡혔을 가능성이 높다. 특히 비용을 키운 것은 Rust/Java 문제가 아니라 DB와 관리형 리소스다.

## 3. Rust로 바뀌면서 줄어드는 부분

현재 새 구조는 Rust `axum + tokio + sqlx` 중심이다.

근거:

- `C:/Users/admin/Desktop/gongzzang/services/api/Cargo.toml`
- `C:/Users/admin/Desktop/platform-core/services/api/Cargo.toml`

Rust 전환의 효과:

| 효과 | 의미 |
|---|---|
| JVM heap 없음 | API 컨테이너 기본 메모리를 크게 줄일 수 있다. |
| GC pause 없음 | p95/p99 latency가 더 예측 가능하다. |
| Tokio async I/O | 외부 I/O와 DB I/O 대기 중 적은 스레드로 많은 연결을 유지할 수 있다. |
| 단일 native binary | 컨테이너 시작/재시작이 빠르고 이미지 구성이 단순하다. |
| SQLx compile-time query check | 쿼리/타입 오류를 더 일찍 잡을 수 있다. |
| 낮은 런타임 오버헤드 | 같은 요청량에서 vCPU/RAM 여유가 더 생긴다. |

따라서 기존 Spring API의 2 vCPU / 8GB 사양을 Rust API에 그대로 줄 필요는 없다.

현실적인 시작값:

| 서비스 | 시작 사양 | 이유 |
|---|---:|---|
| `gongzzang api` | 0.5-1 vCPU / 1-2GB | 대부분 DB/API I/O 중심. Rust 오버헤드 작음. |
| `platform-core api` | 1 vCPU / 2GB | Catalog/marker/tile contract serving, DB 조회 가능성 고려. |
| outbox publisher | 0.5-1 vCPU / 512MB-1GB | 이벤트 fanout 중심이면 작게 가능. |
| ingestion worker | 1-2 vCPU / 2-4GB | JSON parse, R2 write, retry, schema profile 필요. |
| normalization worker | 2-4 vCPU / 4-8GB | batch window, parquet/jsonl 변환, validation에 따라 증가. |

## 4. Rust가 해결하지 않는 병목

Rust라고 해서 아래 문제가 자동으로 사라지지는 않는다.

| 병목 | Rust 효과 | 실제 대응 |
|---|---:|---|
| PostGIS 공간 쿼리 | 낮음 | 인덱스, precomputed anchor/tile, query plan 관리 |
| 큰 JOIN/정렬/aggregation | 낮음 | DB 메모리, index, materialized read model |
| R2 대용량 object scan | 중간 | streaming, shard manifest, worker 분리 |
| 공공 API 쿼터 | 없음 | quota cap, ledger, resume, backoff |
| 지도 payload 반복 제공 | 낮음 | R2/PBF/PMTiles/Valkey/CDN |
| AI 정규화 | 낮음 | 로컬 GPU worker 또는 별도 batch |
| DB lock/connection 부족 | 낮음 | pool limit, statement timeout, slow query 관리 |

즉, Rust는 앱 서버 비용을 낮춘다. DB와 데이터 파이프라인 비용은 설계로 낮춰야 한다.

## 5. 새 구조의 실제 성능 요구를 나누는 법

요구 성능은 하나의 숫자로 잡으면 안 된다. 경로별로 다르다.

### 5.1 일반 API

대상:

- 로그인/세션 검증
- 매물 생성/수정/상세
- 북마크/알림
- admin read/write
- Platform Core catalog read

초기 목표:

| 지표 | 목표 |
|---|---:|
| p95 latency | 300ms 이하 |
| p99 latency | 1000ms 이하 |
| error rate | 1% 미만 |
| API CPU | 평균 60% 이하 |
| API memory | OOM 없이 60-70% 이하 |
| DB connection 사용률 | pool limit의 70% 이하 |

Rust API는 이 범위에서 1 vCPU / 2GB로 시작해도 충분할 가능성이 높다.

### 5.2 지도/마커 API

대상:

- listing marker tile
- parcel anchor tile
- marker filter hash
- count/mask

원칙:

1. public marker request는 임의 `bbox`가 아니라 고정 tile/hash contract를 쓴다.
2. 반복 요청은 Valkey/CDN/cache를 탄다.
3. 필지/산단/정적 공간 레이어는 RDS에서 매 요청마다 만들지 않는다.
4. PostGIS는 exact 계산과 mirror/scratch 용도이지, 전국 정적 지도 hot path가 아니다.

초기 목표:

| 지표 | 목표 |
|---|---:|
| tile cache hit p95 | 100ms 이하 |
| tile cache miss p95 | 500ms 이하 |
| tile p99 | 1500ms 이하 |
| tile body | 가능한 작게, PBF 사용 |
| DB 직접 tile 생성 | hot path에서 제한 |

현재 `platform-core` 로컬 evidence는 5 read RPS + 2 health RPS에서 p95가 한 자리 ms 수준으로 통과했다.
다만 이는 로컬/작은 regional proof다. **AWS production sizing을 확정하는 근거가 아니라, Rust API와 현재 route가 최소 smoke 부하는 통과한다는 근거**로만 본다.

### 5.3 Bronze 수집/정규화

대상:

- data.go.kr 건축물대장
- VWorld cadastral/land register
- R2 Bronze object write
- Silver/Gold handoff
- 전유부 `getBrExposInfo` 확장 예정

이 경로는 사용자 요청 경로가 아니다. 따라서 상시 API 서버 사양을 키워서 해결하면 안 된다.

원칙:

| 작업 | 권장 실행 방식 |
|---|---|
| Bronze ingest | 작은 worker로 shard/resume/ledger 실행 |
| 전국 replay | batch window를 나눠 실행 |
| schema profile | worker에서 산출 후 metadata 저장 |
| Silver transform | R2 manifest filtered streaming |
| AI normalization | 로컬 AI worker 또는 별도 batch queue |

초기 worker 사양:

| 작업 | 시작 사양 |
|---|---:|
| API page fetch + R2 write | 1 vCPU / 2GB |
| JSON parse + schema profile | 2 vCPU / 4GB |
| 큰 shard transform | 4 vCPU / 8GB |
| Spark/DuckDB/Parquet heavy job | 일시적으로 4-8 vCPU / 16-32GB |
| AI 후보 정규화 | AWS가 아니라 로컬 GPU worker |

전유부는 표제부보다 훨씬 커질 가능성이 높다. 따라서 전국 수집 전에 한두 법정동 pilot으로 `totalCount`, page 수, object size, parse memory를 먼저 재야 한다.

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
