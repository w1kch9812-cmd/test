# infrastructure/

IaC, Kubernetes/ECS, GitOps, CI/CD, 배포의 SSOT.

## 책임 영역
- Pulumi 코드 (모든 AWS 리소스)
- 컨테이너 오케스트레이션 (ECS Fargate → EKS)
- GitOps (ArgoCD 또는 Pulumi Cloud)
- CI/CD (GitHub Actions)
- 컨테이너 베이스 이미지 (Wolfi/Distroless)
- DNS·SSL (Cloudflare + Route53)
- WAF·DDoS (Cloudflare Free + AWS Shield Phase 4+)

## 작성 예정 문서 (sub-project 8)
- `iac.md` — Pulumi 모듈 구조
- `kubernetes.md` 또는 `ecs.md` — 오케스트레이션 결정
- `gitops.md` — 배포 흐름
- `ci-cd.md` — GitHub Actions 상세
- `containers.md` — Dockerfile 정책 (multi-stage, Wolfi)
- `dns-ssl.md` — Cloudflare + Route53
- `dr.md` — DR/BCP (Phase 4+)

## 관련 ADR
- → @docs/adr/0009-iac-pulumi.md

## 관련 컨벤션
- → @docs/conventions/git-and-pr.md
