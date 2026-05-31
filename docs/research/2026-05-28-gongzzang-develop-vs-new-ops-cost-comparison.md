# gongzzang-develop vs 새 운영 구조 비용 비교

작성일: 2026-05-28
리전: AWS 서울, `ap-northeast-2`
계산 기준: 온디맨드, 월 730시간, 세금 제외
원화 기준: `1달러 ~= 1,500원` 단순 환산

이 문서는 예전 `C:/Users/admin/Desktop/gongzzang-develop` 인프라와,
지금 새로 설계 중인 `platform-core + gongzzang` 초기 운영 구조를 비교합니다.

핵심은 단순히 싸게 만드는 게 아닙니다.

1. 서비스 경계는 명확하게 분리합니다.
2. 성능을 큰 DB 하나로만 해결하지 않습니다.
3. 실제 트래픽과 지표를 보고 단계적으로 키웁니다.
4. 런칭 전부터 300만원대 고정비 구조를 만들지 않습니다.

## 1. 먼저 알아야 할 점

이 문서는 **코드 기준 추정**입니다.

이 세션의 AWS CLI에는 자격증명이 없어서 실제 AWS Cost Explorer 청구 내역은
직접 읽지 못했습니다. 그래서 정확한 300만원 청구 원인은 AWS 콘솔의
Cost Explorer 또는 비용 CSV에서 최종 확인해야 합니다.

다만 로컬 Pulumi 코드를 보면 왜 300만원대가 나올 수 있는지는 꽤 명확합니다.

확인한 주요 파일:

| 항목 | 파일 |
|---|---|
| 예전 prod 설정 | `C:/Users/admin/Desktop/gongzzang-develop/infrastructure/Pulumi.prod.yaml` |
| 예전 RDS 설정 | `C:/Users/admin/Desktop/gongzzang-develop/infrastructure/src/database/rds.ts` |
| 예전 Redis 설정 | `C:/Users/admin/Desktop/gongzzang-develop/infrastructure/src/database/elasticache.ts` |
| 예전 ALB 설정 | `C:/Users/admin/Desktop/gongzzang-develop/infrastructure/src/loadbalancing/load-balancers.ts` |
| 예전 Fargate 작업 크기 | `C:/Users/admin/Desktop/gongzzang-develop/infrastructure/src/ecs/task-definitions/*` |
| 예전 OSRM 라우팅 서비스 | `C:/Users/admin/Desktop/gongzzang-develop/infrastructure/src/routing-osrm/*` |

공식 가격 출처:

| 출처 | 링크 |
|---|---|
| AWS Price List Bulk API | <https://docs.aws.amazon.com/awsaccountbilling/latest/aboutv2/using-the-aws-price-list-bulk-api-fetching-price-list-files-manually.html> |
| AWS EC2 가격 | <https://aws.amazon.com/ec2/pricing/on-demand/> |
| AWS RDS PostgreSQL 가격 | <https://aws.amazon.com/rds/postgresql/pricing/> |
| AWS Fargate 가격 | <https://aws.amazon.com/fargate/pricing/> |
| AWS Elastic Load Balancing 가격 | <https://aws.amazon.com/elasticloadbalancing/pricing/> |
| AWS ElastiCache 가격 | <https://aws.amazon.com/elasticache/pricing/> |
| Amazon RDS 저장소 문서 | <https://docs.aws.amazon.com/AmazonRDS/latest/UserGuide/CHAP_Storage.html> |
| Cloudflare R2 가격 | <https://developers.cloudflare.com/r2/pricing/> |

## 2. 사용한 단가

아래 단가는 AWS 공식 Price List의 서울 리전 가격 파일에서 확인한 값입니다.

### 2.1 컴퓨팅

| 항목 | 단가 | 월 비용 | 설명 |
|---|---:|---:|---|
| EC2 `t3.micro` Linux | `$0.013`/시간 | `$9.49` | 작은 관리용 서버 |
| EC2 `t4g.large` Linux | `$0.0832`/시간 | `$60.74` | 2 vCPU / 8 GiB |
| EC2 `t4g.xlarge` Linux | `$0.1664`/시간 | `$121.47` | 4 vCPU / 16 GiB |
| EBS gp3 | `$0.0912`/GB-월 | 100GB = `$9.12` | EC2 디스크 |
| Fargate x86 vCPU | `$0.04656`/vCPU-시간 | 사용량별 | ARM이 아니면 보통 이쪽 |
| Fargate x86 메모리 | `$0.00511`/GB-시간 | 사용량별 | ARM이 아니면 보통 이쪽 |
| Fargate ARM vCPU | `$0.03725`/vCPU-시간 | 사용량별 | x86보다 저렴 |
| Fargate ARM 메모리 | `$0.00409`/GB-시간 | 사용량별 | x86보다 저렴 |
| Fargate 추가 임시 저장소 | `$0.000127`/GB-시간 | 사용량별 | 기본 포함량 초과분 |

### 2.2 RDS PostgreSQL

| 항목 | 단가 | 월 비용 | 설명 |
|---|---:|---:|---|
| RDS `db.t4g.micro` Single-AZ | `$0.025`/시간 | `$18.25` | 2 vCPU / 1 GiB |
| RDS `db.t4g.medium` Single-AZ | `$0.102`/시간 | `$74.46` | 2 vCPU / 4 GiB |
| RDS `db.t4g.medium` Multi-AZ | `$0.203`/시간 | `$148.19` | 2 vCPU / 4 GiB |
| RDS `db.m7g.large` Single-AZ | `$0.2344`/시간 | `$171.11` | 2 vCPU / 8 GiB |
| RDS `db.m7g.large` Multi-AZ | `$0.4688`/시간 | `$342.22` | 2 vCPU / 8 GiB |
| RDS `db.m7g.xlarge` Single-AZ | `$0.4688`/시간 | `$342.22` | 4 vCPU / 16 GiB |
| RDS `db.m7g.xlarge` Multi-AZ | `$0.9376`/시간 | `$684.45` | 4 vCPU / 16 GiB |
| RDS gp3 저장소 Single-AZ | `$0.131`/GB-월 | 200GB = `$26.20` | DB 디스크 |
| RDS gp3 저장소 Multi-AZ | `$0.262`/GB-월 | 1000GB = `$262.00` | Multi-AZ라 저장소 비용도 커짐 |
| RDS gp3 IOPS Single-AZ | `$0.023`/IOPS-월 | 사용량별 | 프로비저닝 IOPS |
| RDS gp3 IOPS Multi-AZ | `$0.046`/IOPS-월 | 사용량별 | Multi-AZ에서 더 비쌈 |
| RDS gp3 처리량 Single-AZ | `$0.091`/MiBps-월 | 사용량별 | 프로비저닝 throughput |
| RDS gp3 처리량 Multi-AZ | `$0.182`/MiBps-월 | 사용량별 | Multi-AZ에서 더 비쌈 |

RDS gp3는 기본 IOPS와 기본 처리량이 포함됩니다. 아래 추정에서는
`12,000 IOPS`, `500 MiB/s`를 기본값 초과 성능으로 잡았을 때의 비용을 계산했습니다.
실제 청구에서는 Cost Explorer로 확인해야 합니다.

### 2.3 캐시

| 항목 | 단가 | 월 비용 | 설명 |
|---|---:|---:|---|
| ElastiCache Redis `cache.t4g.small` | `$0.047`/시간 | `$34.31` | 1.37 GiB |
| ElastiCache Valkey `cache.t4g.small` | `$0.0376`/시간 | `$27.45` | 1.37 GiB |
| ElastiCache Serverless Redis 저장소 | `$0.151`/GB-시간 | 1GB 한 달 = `$110.23` | ECPU 별도 |
| ElastiCache Serverless Valkey 저장소 | `$0.101`/GB-시간 | 1GB 한 달 = `$73.73` | ECPU 별도 |
| Redis Serverless ECPU | `$0.0041`/100만 ECPU | 사용량별 | 요청량에 따라 증가 |
| Valkey Serverless ECPU | `$0.0027`/100만 ECPU | 사용량별 | 요청량에 따라 증가 |

### 2.4 네트워크

| 항목 | 단가 | 월 비용 | 설명 |
|---|---:|---:|---|
| ALB 기본 비용 | `$0.0225`/시간 | `$16.43` | 로드밸런서 1개당 |
| ALB LCU | `$0.008`/LCU-시간 | 1 LCU 한 달 = `$5.84` | 트래픽에 따라 증가 |
| ALB 단순 추정 | 기본 + 1 LCU | `$22.27` | 낮은 트래픽 기준 1개당 |
| NAT Gateway 기본 비용 | `$0.059`/시간 | `$43.07` | NAT Gateway 1개당 |
| NAT Gateway 데이터 처리 | `$0.059`/GB | 사용량별 | 외부 요청/다운로드 많으면 증가 |

## 3. 예전 `gongzzang-develop` 구조

### 3.1 가장 큰 비용 원인: 메인 RDS

Pulumi 코드 기준 메인 API DB 설정은 이렇습니다.

| 설정 | 예전 값 |
|---|---:|
| 엔진 | PostgreSQL |
| 인스턴스 | `db.m7g.xlarge` |
| CPU / 메모리 | 4 vCPU / 16 GiB |
| 저장소 | 1000GB gp3 |
| Multi-AZ | prod restore 설정에서 true |
| IOPS | 12,000 |
| 처리량 | 500 MiB/s |

월 비용 추정:

| 메인 API DB 항목 | 계산식 | 월 비용 |
|---|---:|---:|
| 인스턴스 | `db.m7g.xlarge Multi-AZ` | `$684.45` |
| 저장소 | `1000GB * $0.262` | `$262.00` |
| 추가 IOPS 추정 | `(12000 - 3000) * $0.046` | `$414.00` |
| 추가 처리량 추정 | `(500 - 125) * $0.182` | `$68.25` |
| **메인 API DB 소계** |  | **`$1,428.70`** |
| **원화 환산** | `* 1,500` | **약 214만원** |

즉 예전 구조에서는 **메인 DB 하나만 약 214만원/월** 정도가 나올 수 있습니다.

중요한 점:

> 일반 RDS PostgreSQL은 쿼리 한 번마다 돈이 붙는 구조가 아닙니다.
> 여기서 비싼 이유는 큰 DB, Multi-AZ, 1TB 저장소, 높은 IOPS/처리량을
> 한 달 내내 켜두기 때문입니다.

### 3.2 그 외 계속 켜져 있는 항목

prod 설정에 있는 서비스 크기:

| 서비스 | 예전 Fargate 크기 | 월 비용 추정 |
|---|---:|---:|
| API | 2 vCPU / 8GB | `$97.82` |
| Platform Web | 0.5 vCPU / 1GB | `$20.72` |
| Admin Web | 0.5 vCPU / 1GB | `$20.72` |
| LLM | 1 vCPU / 2GB | `$41.45` |
| Keycloak | 0.5 vCPU / 1GB | `$20.72` |
| **소계** |  | **`$201.43`** |

그 외 항목:

| 항목 | 예전 형태 | 월 비용 추정 |
|---|---:|---:|
| Keycloak DB | `db.t4g.micro`, 20GB | 약 `$20~22` |
| Redis | ElastiCache Serverless Redis, 최대 1GB | 최대 `$110.23` + ECPU |
| ALB | API/Web/Admin/LLM/Keycloak = 5개 | 낮은 트래픽 기준 약 `$111.33` |
| NAT Gateway | 1개 | `$43.07` + `$0.059/GB` |
| Bastion | EC2 `t3.micro` | `$9.49` + 디스크/IP |
| 로그 | prod 보관 365일 | 로그량에 따라 변동 |

### 3.3 추가로 비용이 붙을 수 있는 항목

일부는 조건부 또는 작업 실행 시 비용이 붙습니다. 실제 AWS에 떠 있으면 비용이 추가됩니다.

| 항목 | 형태 | 비용 성격 |
|---|---:|---|
| OSRM 라우팅 | ARM Fargate 2 vCPU / 8GB x 2 | 계속 켜져 있으면 약 `$156.54`/월 |
| OSRM 내부 ALB | ALB 1개 | 낮은 트래픽 기준 약 `$22.27`/월 |
| Data pipeline | ARM Fargate 4 vCPU / 30GB + 200GB ephemeral | 실행 시간당 약 `$0.295` |
| Batch task | Fargate 4 vCPU / 16GB | 실행 시간당 약 `$0.268` |
| Lambda 수집기 | 메모리/시간 과금 | 스케줄과 실행시간에 따라 변동 |
| Step Functions | 상태 전이/실행시간 과금 | 실행량에 따라 변동 |
| NAT 트래픽 | 외부 API, 이미지 pull, scraper | GB당 `$0.059` |

### 3.4 예전 구조 기본 비용 추정

배치, 로그 폭증, 스냅샷, S3, 트래픽, dev 중복을 빼고도:

| 그룹 | 월 비용 |
|---|---:|
| 메인 API DB | `$1,428.70` |
| Keycloak DB | `$20~22` |
| Fargate 앱 서비스 | `$201.43` |
| Redis Serverless 1GB 최대 사용 | 최대 `$110.23` + ECPU |
| ALB 5개 | `$111.33` |
| NAT Gateway 기본 | `$43.07` |
| Bastion | `$9.49` |
| **소계** | **약 `$1,924~1,927`** |
| **원화 환산** | **약 288만~289만원** |

그래서 300만원대 청구는 이상한 숫자가 아닙니다.

여기에 아래가 붙으면 더 올라갑니다.

- dev/prod 리소스 중복
- OSRM 상시 실행
- 배치/데이터 파이프라인 장시간 실행
- NAT 데이터 처리
- CloudWatch Logs 수집/저장
- S3/EBS 스냅샷
- 데이터 전송
- Sentry 등 SaaS 비용

## 4. 새 구조에서 우리가 써야 하는 방식

새 구조는 예전처럼 "큰 DB부터 잡고 시작"하면 안 됩니다.

새 구조의 기본 형태:

```text
Cloudflare / 도메인 / HTTPS
  -> 단일 진입점
  -> gongzzang web/api
  -> platform-core api
  -> RDS PostgreSQL + PostGIS
  -> Valkey
  -> R2/S3: 브론즈, 타일, 파일
```

서비스 경계:

| 데이터/도메인 | 주인 |
|---|---|
| 필지, 건물, 산업단지, PNU 앵커 | `platform-core` |
| 매물, 중개사, 사용자-facing 공짱 기능 | `gongzzang` |
| 더니어 workbench/site builder | `dawneer`, 나중 |
| 브론즈 파일, 타일, 공공데이터 원본 | R2/S3 + catalog metadata |
| 캐시, lock, rate limit | Valkey, 원본 아님 |

## 5. 새 구조 Option A: 저비용 초기 런칭

서비스 경계는 지키되, 비싼 관리형 리소스를 최소화하는 방식입니다.

| 항목 | 제안 스펙 | 월 비용 |
|---|---:|---:|
| 앱 서버 | EC2 `t4g.xlarge`, 4 vCPU / 16GB | `$121.47` |
| 앱 서버 디스크 | EBS gp3 100GB | `$9.12` |
| 앱 서버 안에서 실행 | Docker: `gongzzang`, `platform-core`, worker, optional Valkey | EC2 비용에 포함 |
| DB | RDS PostgreSQL/PostGIS `db.t4g.medium` Single-AZ | `$74.46` |
| DB 저장소 | RDS gp3 200GB | `$26.20` |
| Valkey | Docker Valkey on EC2 | 추가 `$0` |
| 관리형 Valkey 대안 | ElastiCache Valkey `cache.t4g.small` | `$27.45` |
| 로드밸런서 | ALB 1개 | 약 `$22.27` |
| NAT Gateway | 가능하면 초기에 회피 | `$0`, 쓰면 `$43.07` |
| R2 | 현재 134GB 기준 | 저장료 약 `$2` + 요청비 |
| 로그 | 짧은 보관, 낮은 트래픽 | 대략 `$10~20` |

예상 합계:

| 형태 | 월 비용 | 원화 환산 |
|---|---:|---:|
| Docker Valkey, NAT 없음 | 약 `$263~275` | 약 40만~41만원 |
| 관리형 Valkey, NAT 없음 | 약 `$291~303` | 약 44만~45만원 |
| 관리형 Valkey + NAT | 약 `$334~346` | 약 50만~52만원 |

이 방식은 초기 비용 통제에 가장 좋습니다.

장단점:

| 장점 | 단점 |
|---|---|
| 싸게 시작 가능 | 완전 관리형보다 운영 책임이 큼 |
| 구조를 단순하게 유지 가능 | 서버 장애 대응을 우리가 더 신경써야 함 |
| Docker로 로컬/운영 환경을 맞추기 쉬움 | 고가용성은 약함 |

## 6. 새 구조 Option B: 관리형이지만 작게 시작

대기업식 운영에 더 가깝지만, 예전 구조보다 훨씬 작게 시작하는 방식입니다.

| 항목 | 제안 스펙 | 월 비용 |
|---|---:|---:|
| `gongzzang web` | Fargate 0.5 vCPU / 1GB | `$20.72` |
| `gongzzang api` | Fargate 1 vCPU / 2GB | `$41.45` |
| `platform-core api` | Fargate 1 vCPU / 2GB | `$41.45` |
| 앱 Fargate 소계 |  | `$103.62` |
| DB | RDS PostgreSQL/PostGIS `db.m7g.large` Single-AZ | `$171.11` |
| DB 저장소 | RDS gp3 300GB | `$39.30` |
| Valkey | ElastiCache Valkey `cache.t4g.small` | `$27.45` |
| 로드밸런서 | ALB 1개 | 약 `$22.27` |
| NAT Gateway | 1개 | `$43.07` + 트래픽 |
| R2 | 현재 134GB 기준 | 저장료 약 `$2` + 요청비 |
| 로그 | 짧은 보관, 필요한 metric | 대략 `$20~40` |

예상 합계:

| 형태 | 월 비용 | 원화 환산 |
|---|---:|---:|
| Option B 기본 | 약 `$429~449` | 약 64만~67만원 |
| Admin web 상시 운영 추가 | `+$20.72` | 약 +3.1만원 |
| DB Multi-AZ 추가 | 인스턴스 + 저장소로 약 `+$210` | 약 +31.5만원 |

이 방식은 "운영 관리 편의성"과 "비용" 사이 균형이 좋습니다.

## 7. 나란히 비교

| 항목 | 예전 `gongzzang-develop` | 새 Option A | 새 Option B |
|---|---:|---:|---:|
| 기본 철학 | 큰 DB + 여러 관리형 서비스 | EC2 Docker + 관리형 DB | 작은 ECS/Fargate |
| 메인 DB | `db.m7g.xlarge`, Multi-AZ | `db.t4g.medium`, Single-AZ | `db.m7g.large`, Single-AZ |
| DB CPU/메모리 | 4 vCPU / 16GB | 2 vCPU / 4GB | 2 vCPU / 8GB |
| DB 저장소 | 1000GB | 200GB | 300GB |
| DB IOPS | 12,000 | 기본값부터 | 기본값부터 |
| DB 처리량 | 500 MiB/s | 기본값부터 | 기본값부터 |
| 메인 DB 월 비용 | 약 `$1,429` | 약 `$101` | 약 `$210` |
| 앱 실행 | Fargate 여러 개 | EC2 Docker | Fargate 소수 |
| 상시 실행 서비스 | API, Web, Admin, LLM, Keycloak | 필수만 | 필수만 |
| ALB 개수 | 5개 이상 | 1개 | 1개 |
| 캐시 | Redis Serverless | Docker Valkey 또는 작은 Valkey | 작은 Valkey |
| 파일/타일 저장 | legacy 혼합 | R2/S3 중심 | R2/S3 중심 |
| 월 비용 추정 | 약 `$1,925+` | 약 `$263~346` | 약 `$429~449` |
| 원화 환산 | 약 289만원+ | 약 40만~52만원 | 약 64만~67만원 |

## 8. 예전 비용이 높은 이유

예전 구조는 "계속 켜져 있는 고정 용량" 때문에 비쌉니다.

비용 영향이 큰 순서:

1. 메인 RDS가 런칭 전/초기 운영치고 너무 큽니다.
   `db.m7g.xlarge + Multi-AZ + 1000GB + 12000 IOPS + 500 MiB/s`.
2. 실제 지표를 보고 키우는 방식이 아니라, 처음부터 높은 IOPS/처리량을 잡았습니다.
3. ALB가 최소 5개라서 고정비가 중복됩니다.
4. 여러 Fargate 서비스가 상시 실행됩니다.
5. Redis Serverless는 작고 예측 가능한 부하에서는 작은 Valkey 노드보다 비쌀 수 있습니다.
6. NAT Gateway는 기본 비용과 GB당 처리비가 같이 붙습니다.
7. prod 로그 보관이 365일입니다. 성숙한 운영에는 의미가 있지만, 초기에는 로그량이 많으면 부담입니다.
8. dev/prod가 같이 떠 있으면 비용이 조용히 두 벌로 붙습니다.

RDS 조회량은 성능에는 중요합니다. 하지만 일반적으로 비용의 직접 원인은
"조회 한 번당 과금"이 아닙니다.

조회량이 많아지면 아래 때문에 비용 문제가 됩니다.

- 더 큰 DB 인스턴스가 필요해짐
- 더 높은 IOPS/처리량이 필요해짐
- read replica가 필요해짐
- 로그/네트워크/앱 인스턴스도 같이 커짐

## 9. 새 아키텍처에 주는 의미

우리는 지도/매물 성능을 큰 RDS 하나로 해결하면 안 됩니다.

더 좋은 역할 분담:

| 작업 | 더 좋은 담당 |
|---|---|
| 매물 원본 데이터 | PostgreSQL |
| 필지/건물/산단 원본 | `platform-core` PostgreSQL/PostGIS |
| 공공데이터 raw 원본 | R2/S3 bronze |
| 정적/준정적 폴리곤 타일 | R2/S3의 PBF/PMTiles |
| 매물 마커 제공 | marker index/tile + Valkey cache |
| 필터 count | DB index query + cache summary |
| 반복되는 hot 요청 | Valkey |
| 전국 수집/정규화 | worker, 사용자 요청 경로와 분리 |
| 나중 AI/vector | platform-core entity 하위로 연결 |

이렇게 하면 DB는 **원본과 정합성의 중심**으로 남고,
반복 조회와 무거운 지도 payload는 R2/타일/캐시/worker로 분산됩니다.

## 10. 추천 결론

런칭 준비 기준 추천:

1. 비용 통제가 최우선이면 **Option A**.
2. AWS 관리형 운영이 더 중요하면 **Option B**.
3. 실제 지표 없이 예전처럼 `db.m7g.xlarge + 1TB + Multi-AZ + 12000 IOPS`로 시작하지 않습니다.
4. `dawneer`는 실제 런칭 전까지 상시 production 운영에서 제외합니다.
5. Valkey는 작게 시작하되 나중에 분리 가능하게 prefix를 나눕니다.
   예: `gongzzang:*`, `platform-core:*`, `dawneer:*`.
6. 브론즈, 타일, 파일, 원본 응답은 R2/S3를 우선 사용합니다.
7. RDS/PostGIS는 authoritative state와 indexed spatial query에 씁니다.
   같은 무거운 지도 payload를 반복 제공하는 용도로 쓰지 않습니다.

추천 초기 목표:

```text
EC2 t4g.xlarge 또는 작은 ECS/Fargate
RDS PostgreSQL/PostGIS db.t4g.medium 또는 db.m7g.large
RDS gp3 200~300GB
작은 Valkey 또는 Docker Valkey
R2: bronze/tile/file data
ALB 1개 또는 Cloudflare + reverse proxy
짧은 로그 보관 + 핵심 metrics
```

확장 경로:

```text
RDS db.t4g.medium
  -> db.m7g.large
  -> Multi-AZ
  -> read replica / 높은 IOPS는 지표로 증명된 뒤

Docker Valkey 또는 cache.t4g.small
  -> 더 큰 Valkey
  -> marker-cache / rate-limit / session cache 분리

EC2 Docker
  -> ECS/Fargate
  -> 서비스별 독립 확장
```

## 11. 비용 guardrail

SSS급 비용 통제를 하려면 아래가 필요합니다.

1. AWS Budget alarm: 50%, 80%, 100%.
2. 서비스별 tag:
   `Service=gongzzang|platform-core|dawneer`,
   `Environment=dev|prod`,
   `CostCenter=...`.
3. dev/prod 예산 분리.
4. 로그 보관 기본값:
   dev 7일, staging 14~30일, prod 30~90일. 감사 요건이 있으면 별도.
5. DB 저장소 자동 증가와 최대치 alarm.
6. RDS CPU, memory, IOPS, connection, slow query alarm.
7. Valkey memory, eviction, CPU, command latency alarm.
8. NAT Gateway data processing alarm.
9. ALB 개수 정기 점검. 작은 서비스마다 ALB를 하나씩 만들지 않습니다.
10. 월 1회 미사용 리소스 점검:
    오래된 EBS volume, snapshot, unattached EIP, unused ALB, stopped task 등.

## 12. 한 줄 결론

예전 `gongzzang-develop` 비용이 높은 이유는 **성능을 큰 RDS와 많은 AWS 관리형 리소스로
먼저 확보한 구조**이기 때문입니다.

새 구조는 이렇게 가야 합니다.

- RDS는 작게 시작
- `platform-core`와 `gongzzang` 경계 명확화
- R2/S3에 무거운 파일/타일/브론즈 저장
- Valkey로 캐시/lock/rate-limit 처리
- batch/수집은 worker로 분리
- 진입점은 단순하게
- 실제 지표를 보고 확장

이 방식이 더 싸고, 더 구조적이고, 나중에 대기업식으로 확장하기도 쉽습니다.
