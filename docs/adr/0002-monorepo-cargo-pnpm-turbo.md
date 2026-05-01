# ADR-0002: 모노레포 — Cargo + pnpm + Turborepo

| | |
|---|---|
| 작성일 | 2026-05-01 |
| 상태 | Accepted |
| 결정자 | 운영자 |

## 컨텍스트

ADR-0001로 Rust + TypeScript 폴리글랏 결정. 두 언어를 한 레포에서 일관 빌드·배포 + 의존성 방향 강제 필요.
풀 도메인 (매물 + 제조업체 + 분석) → 다수의 워크스페이스 멤버 (apps × 2, services × 3, crates × 11, packages × 5).

## 결정

- **Rust**: Cargo workspace (resolver = "3", workspace.dependencies + workspace.lints)
- **TypeScript**: pnpm 9.12 workspaces
- **빌드 오케스트레이터**: Turborepo 2 (Rust + TS 둘 다 task 캐싱·병렬)

## 대안

- **Nx**: 더 강력한 그래프, 그러나 학습 곡선 큼. 모노레포 50+ 패키지 후 재고
- **Yarn workspaces / npm workspaces**: pnpm보다 디스크/속도 열위
- **Bazel**: 대규모 검증, 그러나 셋업 무거움. Phase 3+ 재고
- **Cargo만 (TS는 별도 레포)**: 폴리글랏 분리 — 의존성 동기화 깨짐

## 결과

- 긍정: 단일 진리(루트), 의존성 방향 강제 가능(dependency-cruiser + cargo-arch), 변경 한 PR로 묶음 가능, CI 캐싱 효율, Vercel/Vercel-like 1급 통합
- 부정: 빌드 시간 (Turbo 캐싱으로 완화), pnpm-lock + Cargo.lock 두 lockfile 관리
- 영향 영역: 루트 (`Cargo.toml`, `pnpm-workspace.yaml`, `turbo.json`), 모든 워크스페이스 멤버

## 재검토 트리거

- 워크스페이스 멤버 50+ 시 Nx 재평가
- Turbo Remote Cache 비용이 인프라 5%+ 차지 시
- Bazel 도입이 빌드 시간 50%+ 절감 입증 시

## 참조

- → @docs/conventions/git-and-pr.md
- → Cargo.toml / pnpm-workspace.yaml / turbo.json
