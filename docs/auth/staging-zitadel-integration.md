# Auth — Staging Zitadel 통합 테스트 (deferred from SP3)

SP3 (Auth — Zitadel JWT 핵심 게이트) 의 walking-skeleton CI 는
*mock JWT 모드* (`AUTH_DEV_MODE=true`, `DEV.<sub>` 토큰) 로 e2e 검증해요.
실제 Zitadel access_token 검증은 본 문서에 기록된 별도 sub-project 로 분리.

## 분리 이유

SP3 T9 첫 시도 (commits `9ad70e2`-`1c39b96`) 7 iter 결과 발견된 구조적
어려움:

1. **Zitadel v4 firstinstance PAT 는 opaque token** (길이 277, JWT 아님).
   `JwtVerifier` 의 `RS256` 검증 통과 못함. → OIDC `client_credentials`
   grant 흐름 별도 필요.
2. **`/debug/healthz` 가 거짓말함** — 200 OK 반환 후에도 management API
   는 +25-30 초 503 반환. grpc-gateway 가 internal grpc backend 준비 전
   응답.
3. **`docker run` vs `services:` map** — 기본 entrypoint 가 subcommand
   필요. service container 로는 자동 시작 불가.
4. **GitHub Actions billing** — 7 iter 동안 최소 21 워크플로우 실행 →
   billing 한도 도달.

위 어려움은 *CI 배경 작업이 아닌 별도 인프라 통합 task* 로 분리하는 게
적정. 매 PR 마다 Zitadel 컨테이너 부팅 (5 분+) 도 SSS 효율 기준 미달.

## 해야 할 것 (후속 sub-project)

1. Staging 환경 Zitadel 인스턴스 구성 (Pulumi `infra/zitadel/`)
2. Staging 전용 e2e 테스트 워크플로우 (`.github/workflows/staging-auth-integration.yml`):
   - 매 PR 이 아닌, main merge 후 또는 manual dispatch
   - Zitadel API 로 OIDC project + machine user 생성 (idempotent)
   - `client_credentials` grant 로 RS256 JWT 발급
   - Staging API 로 round-trip 검증
3. 첫 시도 발견사항 (위 4 가지) 활용해 1-2 iter 안에 완료 목표

## 첫 시도 산출물 (참고)

- `crates/auth/src/verifier.rs` 의 `Verifier::Real(JwtVerifier)` 분기 — 구현 완료
- middleware/extractor/role guard — 모두 mock 모드로 e2e 검증됨
- 즉, *실제 Zitadel 통합 시 변경할 것은 walking-skeleton.yml 만* (또는 신규 staging workflow)

## 우선순위

SP4 (외부 API) / SP5 (Repository SQLx 구현) 후 처리. SSS 평가 시
"production 배포 전 처리" 필수 항목으로 분류.
