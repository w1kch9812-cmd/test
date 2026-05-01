# ADR-0009: IaC — Pulumi (TypeScript)

| | |
|---|---|
| 작성일 | 2026-05-01 |
| 상태 | Accepted |
| 결정자 | 운영자 |

## 컨텍스트

AWS Seoul 인프라 (ECS, RDS, ElastiCache, S3, ALB, WAF, Secrets Manager 등) 코드화 필요. AWS 콘솔 직접 변경 = SSOT 위반(기둥 6). SSS = 인프라가 코드로만 변경, drift 자동 감지.

## 결정

- **도구**: Pulumi (TypeScript)
- **상태 저장**: Pulumi Cloud (무료 개인) → Phase 3+ S3 backend 자체 호스팅
- **모듈**: `infrastructure/{networking, compute, data, messaging, security, observability, keycloak, batch-scheduler, cdn, dr, compliance}/`
- **환경**: dev / staging / production stack 분리
- **drift 감지**: 정기 `pulumi refresh` (CI cron)
- **변경 절차**: PR + `pulumi preview` 결과 첨부 → 머지 시 `pulumi up`

## 대안

- **Terraform / OpenTofu**: 사실상 표준, HCL은 한계 (코드 재사용 약함). OpenTofu가 라이선스 자유 (Phase 4+ 마이그레이션 옵션)
- **AWS CDK**: AWS 록인, 다른 클라우드 못 감
- **Crossplane**: K8s 네이티브, 학습 곡선 큼
- **AWS CloudFormation**: 록인, YAML 한계

## 결과

- 긍정: TypeScript = 우리 프론트 스택과 일관, 타입 안전, 코드 재사용(`packages/tsconfig` 공유), Pulumi Free Tier 충분 (Phase 1-3)
- 부정: Pulumi 록인 (state migration 부담 — 그러나 OpenTofu 마이그레이션 도구 존재), Pulumi Cloud 비용 (Phase 4+)
- 영향 영역: `infrastructure/`, `.github/workflows/ci.yml` (deploy job), AWS 모든 리소스

## 재검토 트리거

- Pulumi Cloud 비용이 인프라 전체 5%+ 차지 시 → S3 backend 자체 호스트
- OpenTofu가 Pulumi 대비 ecosystem 큰 우위 보일 때
- 멀티 클라우드 진출 시 (AWS 외 GCP/Azure) → Crossplane 재평가
- ISMS-P 평가에서 IaC 도구 제약 시

## 참조

- → @docs/infrastructure/iac.md (작성 예정)
- → @docs/infrastructure/README.md
- Pulumi: https://www.pulumi.com
