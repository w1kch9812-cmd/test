# 대기업 레이크하우스, 미디어, 레지스트리 벤치마크

상태: 조사 기준선
작성일: 2026-06-07
범위: Gongzzang 레이크하우스, 미디어 객체 저장, Platform Core 레지스트리, 향후 AI/벡터

## 결론

현재 Gongzzang 방향은 대기업식 목표와 맞습니다.

- 서비스가 자기 데이터를 직접 소유합니다.
- Platform Core 는 중앙 레지스트리와 거버넌스 컨트롤 플레인입니다.
- 사진, 영상, 도면, 문서 같은 바이너리 미디어는 소유 서비스의 레이크하우스
  `media/` 아래에 둡니다.
- OCR, 캡션, 라벨, 임베딩, 품질 리포트, 검색/벡터 인덱스는 원본 미디어에서 파생된
  governed asset 으로 따로 등록합니다.
- R2 Data Catalog / Iceberg 는 쿼리 가능한 테이블 메타데이터용이고, Platform Core 의
  cross-service 레지스트리를 대체하지 않습니다.

즉, 목표는 "공용 버킷 하나에 폴더만 많이 두는 방식"이 아닙니다.
목표는 "도메인 소유 물리 저장소 + 중앙에서 관리되는 메타데이터"입니다.

## 실제 사례

| 사례 | 실제 방식 | Gongzzang 목표 |
|---|---|---|
| Google Cloud Data Mesh / Dataplex | 도메인 데이터 소유자가 데이터를 소유하고, 중앙 서비스가 catalog, discovery, policy, governance 를 제공합니다. Cloud Storage / BigQuery 자산을 lake/zone 에 붙이고, 이미지/텍스트 같은 비정형 데이터도 fileset metadata 로 스캔합니다. | Gongzzang 은 매물, 경매, 온비드, 마커, 미디어를 소유합니다. Platform Core 는 중앙 registry/catalog 이지 모든 사실의 소유자가 아닙니다. |
| AWS Lake Formation + Glue Data Catalog | 데이터 레이크 접근권한, 공유, 감사, governance 를 중앙에서 관리하고, Glue 가 catalog/ETL 관련 catalog API 를 담당합니다. | Registry/catalog 와 권한 정책은 1급 시스템이어야 합니다. R2 object key 자체가 public contract 가 되면 안 됩니다. |
| AWS S3 Metadata / S3 Tables | S3 객체 메타데이터를 read-only managed Iceberg table 로 자동 캡처하고, S3 Tables 를 Glue 와 연결해 분석 엔진이 테이블을 발견할 수 있게 합니다. | 미디어 객체도 queryable metadata, inventory, lineage 가 필요합니다. 애플리케이션이 폴더를 훑어서 자산을 발견하면 안 됩니다. |
| Netflix Iceberg | Netflix 의 원래 Iceberg 저장소는 table snapshot, metadata file, manifest, atomic metadata replacement 로 snapshot isolation 을 설명합니다. | 구조화된 lakehouse table 은 "최신 폴더 추측"이 아니라 table metadata 와 active version pointer 로 관리해야 합니다. |
| Uber Apache Hudi | Uber 는 고용량 ETL 을 Hudi 기반 incremental processing 으로 옮겨 full recompute 를 줄이고 changed data 를 upsert 하는 구조를 썼습니다. | 변경이 잦은 데이터는 재수집/재계산 전체 반복보다 resumable incremental run, checkpoint, artifact registration 이 목표입니다. |
| Databricks Lakehouse / Unity Catalog | raw landing, curated layer, ACID table format, lineage, data/AI unified governance 를 함께 둡니다. | Bronze/Silver/Gold 는 유지하되 version, lineage, policy, quality evidence 로 관리될 때만 의미가 있습니다. |
| Cloudflare R2 Data Catalog | R2 위에 managed Apache Iceberg REST catalog 를 제공합니다. 객체 데이터와 Iceberg metadata file 은 object storage 에 있고, catalog 가 table 목록과 current metadata pointer 를 추적합니다. | R2 Data Catalog 는 R2 lakehouse table 쿼리에 유용합니다. 그래도 cross-service asset identity, active version, lineage, consumer contract 는 Platform Core registry 가 맡습니다. |

## 근거 링크

- [Google Cloud: Architecture and functions in a data mesh](https://docs.cloud.google.com/architecture/data-mesh)
- [Google Cloud: Build a data mesh with Dataplex / Knowledge Catalog](https://cloud.google.com/dataplex/docs/build-a-data-mesh)
- [Google Cloud: Manage metadata of lakes, zones, and assets](https://docs.cloud.google.com/dataplex/docs/metadata)
- [AWS Lake Formation](https://aws.amazon.com/lake-formation/)
- [AWS Lake Formation components](https://docs.aws.amazon.com/lake-formation/latest/dg/how-it-works-components.html)
- [AWS S3 Metadata tables](https://docs.aws.amazon.com/AmazonS3/latest/userguide/metadata-tables-overview.html)
- [AWS S3 Tables](https://docs.aws.amazon.com/AmazonS3/latest/userguide/s3-tables-tables.html)
- [AWS: Unstructured data management and governance](https://aws.amazon.com/blogs/big-data/unstructured-data-management-and-governance-using-aws-ai-ml-and-analytics-services/)
- [Netflix Iceberg repository](https://github.com/Netflix/iceberg)
- [Uber: Transactional data lake with Apache Hudi](https://www.uber.com/ie/en/blog/ubers-lakehouse-architecture/)
- [Databricks: What is a lakehouse?](https://docs.databricks.com/aws/en/lakehouse/)
- [Cloudflare R2 Data Catalog](https://developers.cloudflare.com/r2/data-catalog/)

## 목표 구조

```text
Platform Core
|-- Lakehouse Registry
|   |-- service owner
|   |-- dataset / media object set
|   |-- artifact version
|   |-- active pointer
|   |-- lineage
|   |-- quality evidence
|   |-- policy / consumer binding
|
|-- Catalog-owned lakehouse
|   |-- parcel / building / PNU anchor / public spatial layers
|
Gongzzang
|-- Gongzzang-owned lakehouse
|   |-- bronze/source=onbid-sale/...
|   |-- bronze/source=court-auction/...
|   |-- silver/dataset=onbid-sale/...
|   |-- silver/dataset=court-auction/...
|   |-- gold/listing-marker-tiles/...
|   |-- media/listing-photo/listings/{listing_id}/photos/{photo_id}.{ext}
|   |-- media/listing-video/...
|   |-- media/floor-plan/...
|   |-- media/broker-document/...
|   `-- __r2_data_catalog/
|
Dawneer
`-- Dawneer-owned lakehouse
    `-- dawneer-owned operational/site-workbench datasets and media
```

## 앞으로 유지할 결정

1. 모든 데이터를 하나의 공용 root medallion bucket 으로 합치지 않습니다.
2. Platform Core 위에 네 번째 상위 서비스를 만들지 않습니다.
3. 물리 저장소는 서비스별로 분리합니다:
   `platformcore-lakehouse-prod`, `gongzzang-lakehouse-prod`, `dawneer-lakehouse-prod`.
4. 논리 governance 는 Platform Core 에 둡니다:
   registry, policy, lineage, active version, consumer contract.
5. 미디어는 기본적으로 소유 서비스 레이크하우스의 `media/` 아래에 둡니다.
   단, 보안/리전/개인정보 요구가 생기면 restricted bucket 으로 분리할 수 있습니다.
6. AI/search 파생물은 `media/` 에 넣지 않습니다. 원본 미디어를 참조하는 derived dataset
   또는 serving index 로 등록합니다.
7. schema evolution, snapshot isolation, multi-engine read, metadata planning 이 필요한
   구조화 데이터는 Iceberg/R2 Data Catalog 대상으로 봅니다.
8. 미디어 발견은 object metadata/inventory 로 합니다. 런타임 코드가 folder prefix 를
   훑어서 자산을 찾으면 안 됩니다.
9. `LISTING_PHOTO_R2_*` 는 업로드/다운로드 signing 과 authorization 을 위한 runtime edge
   boundary 로 유지합니다. 같은 bucket 을 가리킬 수 있지만 object key 는 반드시
   `media/listing-photo/` 아래여야 합니다.

## AI 와 벡터 목표

AI 는 governed asset 을 읽어야 합니다. AI 시스템이 두 번째 source of truth 가 되면 안 됩니다.

```text
media object
-> extracted text / captions / labels / OCR
-> normalized metadata dataset
-> embedding dataset
-> vector serving index
-> AI assistant / search / recommendation / call-center workflows
```

벡터 인덱스는 serving 성능 때문에 물리적으로 별도 저장소에 있을 수 있습니다.
하지만 identity 는 Platform Core registry 에 등록되어야 합니다.

- source media object ID
- extraction model and version
- embedding model and version
- source artifact checksum
- index build run ID
- retention / privacy class
- active serving version

이렇게 해야 AI 가 유용해지면서도 별도의 무관리 중복 DB 가 되지 않습니다.

## 현재 Gongzzang 적합성

현재 ADR 0039 와 service-owned lakehouse spec 은 대기업 벤치마크와 맞습니다.

이미 맞는 부분:

- Gongzzang 이 Gongzzang 비즈니스 데이터를 소유합니다.
- Platform Core 가 top-level registry/control plane 입니다.
- listing media 는 `media/listing-photo/` 아래입니다.
- Bronze/Silver/Gold 와 binary media 가 분리되어 있습니다.
- AI/vector 결과물은 raw media 가 아니라 derived asset 으로 설명되어 있습니다.
- runtime photo signing 과 batch lakehouse writer config 가 분리되어 있습니다.

남은 hardening:

- Gongzzang pipeline completion 에 Platform Core registry call 을 붙입니다.
- shared root `bronze/`, `silver/`, `gold/` 에 새 Gongzzang write 가 들어가면 CI 가 막게 합니다.
- media asset metadata schema 를 추가합니다:
  object ID, entity reference, content hash, object key, MIME, size, retention, privacy class,
  lineage.
- Bronze coverage 가 안정된 뒤 고가치 구조화 dataset 에 Iceberg/R2 Data Catalog 를 도입합니다.
- 기존 stashed marker work 는 listing marker 구현 재개 전까지 별도로 보존합니다.

## 최종 목표

```text
도메인 소유 저장소.
중앙 governance metadata.
lineage-first AI.
folder guessing 금지.
duplicate truth 금지.
```
