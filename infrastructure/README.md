# infrastructure/

Pulumi (TypeScript) IaC. AWS Seoul + Cloudflare 무료 활용. sub-project 8에서 본격 작성.

## 향후 모듈 (sub-project 8+)
- `networking/` — VPC, Subnet, NACL, Security Group
- `compute/` — ECS Fargate (Phase 3+) → EKS (Phase 4+)
- `data/` — RDS (Postgres + PostGIS), ElastiCache (Valkey), S3
- `messaging/` — SQS, SNS, EventBridge, Kafka MSK (Phase 4+)
- `security/` — WAF, KMS, Secrets Manager, IAM
- `observability/` — CloudWatch, OTel Collector
- `keycloak/` (실제는 Zitadel) — IdP ECS
- `batch-scheduler/` — EventBridge + Lambda
- `cdn/` — CloudFront 또는 Cloudflare Pages
- `dr/` — DR 리전 (Phase 4+)
- `compliance/` — Audit log S3 Object Lock

## 정책
- AWS 콘솔 직접 변경 = SSOT 위반 (drift 자동 감지)
- 모든 변경 PR + `pulumi preview` 첨부
- destroy 작업 = 사용자 명시 승인 필수
- 멀티 환경: dev / staging / production stack 분리
- 비용 알림 (AWS Budgets) + DORA 메트릭 자동 수집

## 환경 변수
환경별 secrets는 Pulumi config (`pulumi config set --secret`) 또는 AWS Secrets Manager.

→ ADR-0009, → @docs/infrastructure/iac.md (sub-project 8에서 작성)
