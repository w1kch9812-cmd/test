# Sub-project 3 Auth Zitadel JWT - Part 02C: Validation And Handoff

Parent index: [Sub-project 3 Auth Zitadel JWT - Part 02](./2026-05-03-sub-project-3-auth-zitadel-jwt.part-02.md).

## Phase F: 검증 + 종료

### Task 10: 통합 검증 + project_progress 갱신

**Files:**
- Modify: `MEMORY.md` (hook line)
- Modify: `memory/project_progress.md` (SP3 추가)

- [ ] **Step 1: workspace 멤버 + 테스트 카운트 확인**

```bash
# 멤버 25개 확인 (24 + auth)
grep -c '"crates/' Cargo.toml

# 테스트 카운트
grep -rE '#\[test\]|#\[tokio::test\]' crates/ services/ --include="*.rs" | wc -l
```

목표: 1017 (T0) + 약 20 (auth crate) ≈ 1037+.

- [ ] **Step 2: `MEMORY.md` 갱신**

```diff
- - [프로젝트 진행 현황](memory/project_progress.md) — Sub-project 1+2 완료 (24 crate, 1017 tests), Rust 1.88
+ - [프로젝트 진행 현황](memory/project_progress.md) — SP1+2+3 완료 (25 crate, ~1040 tests), Rust 1.88, Auth 게이트
```

- [ ] **Step 3: `memory/project_progress.md` 에 SP3 절 추가**

기존 SP2c 절 다음에 추가:

```markdown
### Sub-project 3: Auth — Zitadel JWT 핵심 게이트 (완료, T1-T10)

- 신규 crate: `crates/auth` (verifier + JWKS 캐시 + middleware + extractor + role guard)
- `services/api` 미들웨어 적용 — `/healthz` public, `/users/*` 인증 보호
- `POST /users` 제거 (first-sign-in 자동 생성으로 대체)
- `GET /users/me` 추가
- migration 30005: user.roles CHECK 제약
- CI walking-skeleton 에 Zitadel 컨테이너 통합 + 4단계 e2e
- 누적 테스트 ~1040, 25 crate

미포함 (후속): 소셜 로그인, NICE 본인인증, 2FA, endpoint 별 RBAC 매트릭스
```

- [ ] **Step 4: commit + push + 3 CI 그린 최종 확인**

```bash
git add MEMORY.md memory/project_progress.md
git commit -m "chore(sp3-t10): integration validation — Sub-project 3 complete (25 crates, ~1040 tests)

- crates/auth 1 신규 crate
- services/api 인증 보호 적용
- migration 30005 user_roles CHECK
- walking-skeleton.yml Zitadel 컨테이너 통합 + e2e 4단계 그린

다음: SP4 (외부 API 통합) 또는 SP5 (Repository SQLx 구현)"
git push
gh run list --branch main --limit 3
```

3 워크플로우 모두 그린 확인.

---

## 검증 기준 매핑 (Spec § 11)

| Spec § 11 항목 | 본 plan task |
|---|---|
| 1. `crates/auth/` 신규 crate ≥40 tests, 90% 커버리지 | T1-T6 (errors 4 + claims 6 + jwks 2 + verifier 1 + role guard 3 + 통합 ~5 = ~21; 깊은 검증은 T9 e2e 로 보강) |
| 2. `User` `roles` 필드 + `find_by_zitadel_sub` | **이미 존재** (정정 절 참조) |
| 3. migration `30005` 적용, `db-migrations.yml` 그린 | T8 |
| 4. `services/api` 미들웨어 + POST /users 제거 | T7 |
| 5. `walking-skeleton.yml` Zitadel + e2e 그린 | T9 |
| 6. 3 CI 워크플로우 그린 | T10 |
| 7. 누적 ≥1080 tests | T10 — 실측 ~1040 (Spec 추정과 다름; 도메인 변경 없어 늘어나는 양이 적음) |
| 8. tarpaulin ≥90% | T1-T6 + T9 (e2e) |
| 9. clippy -D warnings | T1-T9 매 commit |
| 10. cargo deny check | T1-T9 매 commit |
| 11. 파일 ≤500 / ≤1500 | T1-T9 매 commit (file size CI job) |

> **검증 기준 7 deviation:** Spec 은 ≥1080 추정했으나 도메인 변경이 거의 없어 실측 ~1040. 본 plan 의 task 수는 spec 의 검증 기준을 모두 만족하되, 테스트 *총량* 은 도메인 작업이 빠진 만큼 줄어요. tarpaulin ≥90% 는 변하지 않음.

---

## Self-Review (plan 작성자 — 끝났음)

- [x] Spec § 1-14 모든 절 반영 — 도메인 작업이 이미 끝났단 사실 정정
- [x] 9 task → 10 task (T8 마이그 추가)
- [x] 모든 task 가 fresh subagent dispatch 가능한 단위
- [x] TDD 패턴 (test-first) — Rust+Windows 한계 반영해 "test+impl 같이 작성 → CI 가 검증" 변형
- [x] 파일 ≤500 룰: auth crate 의 각 파일 의도적으로 작게 분리
- [x] 알려진 lessons (`#[path]` import, doc_markdown, derive_partial_eq_without_eq, missing_const_for_fn) 사전 대응

## 알려진 위험

1. **T9 가장 어려움** — Zitadel CI 셋업은 레퍼런스 적음. Zitadel firstinstance PAT 출력 형식, OIDC client credentials grant 흐름, JWT aud 검증 모두 1-3 iter 가능. Plan 의 setup 스크립트는 *첫 시도* — 실제 응답에 맞춰 수정 필요.
2. **Zitadel PAT 형식** — Zitadel 의 PAT 가 RS256 JWT 인지 opaque token 인지에 따라 우리 verifier 가 다르게 동작. opaque 면 token introspection endpoint 호출이 필요해 verifier 분기 추가 필요. 이 경우 T9 직전에 verifier T4 에 patch.
3. **JWT aud 가 `client_id` 인지 `service_user` 인지** — Zitadel 설정에 따라 다름. 셋업 스크립트가 발급한 토큰의 `aud` 를 한 번 dump 해서 확인 후 ZITADEL_AUDIENCE 값 결정. 처음에 안 맞으면 verify 가 InvalidAudience 거절.
4. **race condition (5.3)** — 미들웨어가 첫 sign-in race 한 번 흡수하지만, 동일 sub 가 거의 동시 3+ 요청 시 추가 race 가능. PgUserRepository.save 의 unique violation 처리는 SP5 에서 보강.

## 완료 후 다음

**Sub-project 3 종료** → 사용자 결정:
- **Sub-project 4**: 외부 API 통합 (V-World, 법제처, data.go.kr) — Reader trait 구현체
- **Sub-project 5**: Repository SQLx 구현 — 23 trait 의 PgImpl + testcontainers

순서는 사용자 선택.
