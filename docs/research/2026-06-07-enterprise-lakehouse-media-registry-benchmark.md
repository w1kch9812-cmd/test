# 대기업형 레이크하우스·미디어 Registry 벤치마크

상태: 조사 기반 결정 보강
작성일: 2026-06-07
범위: Gongzzang-owned lakehouse, listing photo media, Platform Core Lakehouse Registry, AI/vector 확장

## 결론

현재 방향은 대기업식 구조와 맞다.

- Gongzzang은 Gongzzang 비즈니스 데이터와 미디어 객체를 소유한다.
- Platform Core는 모든 서비스를 가로지르는 Registry, policy, lineage, active version pointer를 관리한다.
- API는 외부 Registry에 직접 쓰지 않고 DB transaction 안에서 outbox event만 만든다.
- outbox worker가 R2 객체를 읽고, size/content-type/SHA-256을 검증한 뒤 Platform Core Lakehouse Registry에 등록한다.
- AI/vector는 원본 미디어 자체가 아니라 원본에서 파생된 governed asset으로 등록한다.

즉 목표는 “모든 데이터를 한 bucket/root에 몰아넣기”가 아니라 “도메인별 물리 저장소 + 중앙 governance metadata”다.

## 실제 사례에서 확인한 원칙

| 사례 | 확인한 원칙 | Gongzzang 적용 |
|---|---|---|
| AWS Transactional Outbox | DB write와 외부 message 발행을 한 요청에서 동시에 처리하면 dual-write 문제가 생긴다. outbox table을 DB transaction 안에 쓰고 별도 publisher가 외부 전송을 맡긴다. | `confirm_photo_upload`는 listing photo DB 변경과 outbox event를 같은 transaction에 남긴다. Registry 호출은 worker가 수행한다. |
| Amazon Builders' Library idempotent APIs | 재시도 가능한 API는 caller-provided request id 또는 idempotency token으로 중복 side effect를 막고 감사 가능성을 높인다. | Registry 등록은 checksum/object key/artifact identity를 기준으로 같은 객체를 반복 등록해도 의미가 같아야 한다. |
| Stripe idempotency keys | POST 요청도 idempotency key와 parameter comparison으로 안전하게 retry할 수 있게 만든다. | Platform Core Registry API는 같은 object key/checksum 조합의 재시도에 안전해야 한다. |
| Google Cloud Dataplex / Knowledge Catalog | data mesh는 domain owner가 데이터를 소유하고, catalog/lake/zone/asset으로 governance와 discovery를 제공한다. | Gongzzang, Platform Core, Dawneer는 각각 service-owned lakehouse를 갖고 Platform Core가 catalog 역할을 한다. |
| AWS Lake Formation + Glue | Lake Formation은 권한/governance, Glue는 catalog와 ETL 관리 역할을 나눠 수행한다. | Platform Core Registry는 governance/control plane이고, Gongzzang worker는 실제 artifact producer다. |
| Cloudflare R2 Data Catalog | R2 bucket 안 Iceberg REST catalog는 query engine이 table과 current metadata pointer를 찾게 해 준다. | R2 Data Catalog/Iceberg는 queryable table에 적합하다. Cross-service asset identity와 consumer contract는 Platform Core Registry가 유지한다. |
| Databricks Lakehouse / Unity Catalog | Lakehouse는 storage와 compute를 분리하고, medallion layers, lineage, governance, AI/ML use case를 하나의 데이터 체계로 묶는다. | Bronze/Silver/Gold와 media/derived AI assets는 lineage와 version을 갖고 Registry에 등록한다. |
| Apache Iceberg | object storage 위 큰 analytics table을 여러 engine이 안전하게 읽고 쓸 수 있게 해 준다. | 대량 tabular data에는 Iceberg/R2 Data Catalog를 후보로 두되, 사진 같은 binary media는 object metadata + derived asset lineage가 우선이다. |

## 적용 구조

```text
API request
-> listing photo DB transaction
   -> listing_photo row update
   -> outbox event: listing_photo.upload_confirmed
-> response

outbox-publisher worker
-> read unpublished outbox event
-> ignore unrelated event types
-> fetch R2 object stream
-> verify content-type, size, SHA-256
-> POST /internal/lakehouse/artifacts to Platform Core
-> mark outbox event published only after success
```

## 현재 구현 원칙

- API는 Registry HTTP client를 소유하지 않는다.
- Platform Core service auth는 `crates/auth/src/platform_core_service.rs`가 SSOT다.
- Lakehouse Registry HTTP adapter는 `services/outbox-publisher`가 소유한다.
- listing photo media asset identity는 worker가 policy에서 파생한다.
- listing row나 photo row가 canonical Registry metadata를 중복 소유하지 않는다.
- production에서 worker registry sink는 기본 활성화된다.
- local/dev에서는 명시적으로 `OUTBOX_LAKEHOUSE_REGISTRY_ENABLED=true`를 설정해야 실제 Registry 호출을 한다.

## AI/vector 확장

AI는 원본 미디어를 덮어쓰거나 별도 “정체불명 DB”에 복제하면 안 된다.

```text
media/listing-photo/...
-> OCR/caption/label extraction artifact
-> normalized metadata artifact
-> embedding artifact
-> vector serving index
-> AI assistant / search / recommendation / call-center
```

각 derived artifact는 다음을 가져야 한다.

- source object key
- source checksum
- source entity reference
- model name/version
- extraction or embedding run id
- privacy/retention class
- active serving version

## 남은 hardening

- Platform Core Registry API가 idempotent registration contract를 확정해야 한다.
- worker sink는 Registry 성공 후 outbox를 published 처리한다. Registry API가 불안정하면 같은 event가 재시도될 수 있으므로 Registry 쪽 idempotency가 필요하다.
- 향후 Kafka를 도입해도 API 구조는 갈아엎지 않는다. DB outbox가 source of truth이고 Kafka는 downstream fanout lane이 된다.
- tabular Bronze/Silver/Gold가 커지면 R2 Data Catalog/Iceberg를 dataset별로 도입한다. media binary object는 그대로 object namespace에 두고 metadata/derived asset만 table화한다.

## 근거 링크

- AWS Transactional Outbox: https://docs.aws.amazon.com/prescriptive-guidance/latest/cloud-design-patterns/transactional-outbox.html
- Amazon Builders' Library, idempotent APIs: https://aws.amazon.com/builders-library/making-retries-safe-with-idempotent-APIs/
- Stripe idempotent requests: https://docs.stripe.com/api/idempotent_requests
- Google Cloud Dataplex data mesh: https://cloud.google.com/dataplex/docs/build-a-data-mesh
- AWS Lake Formation components: https://docs.aws.amazon.com/lake-formation/latest/dg/how-it-works-components.html
- Cloudflare R2 Data Catalog: https://developers.cloudflare.com/r2/data-catalog/
- Databricks lakehouse: https://docs.databricks.com/aws/en/lakehouse/
- Apache Iceberg: https://iceberg.apache.org/
